// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The share-submitting service of the timelock (spec `aptos-upgrade-ibe-chunky-spec.md` sections 5 and 8).
//!
//! One service runs per epoch on a validator whose timelock state is `Ready`. It reads the registry from
//! committed chain state, and for every timelock that passes the node rule
//! ([`DerivableTimelock::authorize`]) it derives the node's shares and puts one `TimelockShare` validator
//! transaction into the pool. It never derives a share for an identity it did not read from the registry,
//! and the committed block time (not the wall clock) decides when the deadline has passed.
//!
//! The service holds no secret of its own: the shares are the ones of the epoch's [`TimelockServiceState`].
//! Dropping the service (at the epoch change) drops the pool guards, which removes its transactions.

use crate::counters;
use anyhow::{anyhow, Result};
use aptos_dkg::ibe::MasterScalar;
use aptos_logger::{info, warn};
use aptos_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReader,
};
use aptos_types::{
    on_chain_config::{ConfigurationResource, CurrentTimeMicroseconds, OnChainConfig},
    state_store::{state_key::StateKey, table::TableHandle, TStateView},
    timelock::{
        timelock_registry_table_handle, DerivableTimelock, TimelockInfoResource,
        TimelockStatusResource,
    },
    validator_txn::{TimelockShare, Topic, ValidatorTransaction},
};
use aptos_validator_transaction_pool::{TxnGuard, VTxnPoolState};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{StructTag, CORE_CODE_ADDRESS},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

/// How often the service reads chain state.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// What the service needs from committed chain state.
pub trait TimelockChainView: Send {
    /// Epoch and block timestamp (microseconds) of the latest committed state.
    fn epoch_and_time(&self) -> Result<(u64, u64)>;
    /// The registry row of `timelock_id`, or `None` if it is not registered (yet).
    fn timelock(&self, timelock_id: u64) -> Result<Option<TimelockInfoResource>>;
}

/// [`TimelockChainView`] over the node's database.
pub struct DbTimelockChainView {
    reader: Arc<dyn DbReader>,
}

impl DbTimelockChainView {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        Self { reader }
    }
}

impl TimelockChainView for DbTimelockChainView {
    fn epoch_and_time(&self) -> Result<(u64, u64)> {
        let view = self.reader.latest_state_checkpoint_view()?;
        let config = ConfigurationResource::fetch_config(&view)?
            .ok_or_else(|| anyhow!("no configuration resource"))?;
        let now = CurrentTimeMicroseconds::fetch_config(&view)?
            .ok_or_else(|| anyhow!("no timestamp resource"))?;
        Ok((config.epoch(), now.microseconds))
    }

    fn timelock(&self, timelock_id: u64) -> Result<Option<TimelockInfoResource>> {
        let view = self.reader.latest_state_checkpoint_view()?;
        let registry_tag = StructTag {
            address: CORE_CODE_ADDRESS,
            module: move_core_types::identifier::Identifier::new("ibe_config")?,
            name: move_core_types::identifier::Identifier::new("TimelockRegistry")?,
            type_args: vec![],
        };
        let key = StateKey::resource(&CORE_CODE_ADDRESS, &registry_tag)?;
        let Some(bytes) = view.get_state_value_bytes(&key)? else {
            return Ok(None);
        };
        let handle = timelock_registry_table_handle(&bytes)
            .ok_or_else(|| anyhow!("TimelockRegistry resource is too short"))?;
        let item_key = StateKey::table_item(
            &TableHandle(AccountAddress::new(handle)),
            &bcs::to_bytes(&timelock_id)?,
        );
        match view.get_state_value_bytes(&item_key)? {
            Some(bytes) => Ok(Some(bcs::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }
}

/// The decision logic of the service, separate from the timer and the database.
pub struct ShareServiceCore {
    epoch: u64,
    chain_id: u8,
    my_addr: AccountAddress,
    shares: Vec<MasterScalar>,
    /// Timelock ids below this were seen in the registry.
    next_probe: u64,
    /// Registered timelocks that are still pending in this epoch.
    open: BTreeMap<u64, ()>,
    /// One pooled transaction per timelock. Dropping a guard removes the transaction.
    guards: HashMap<u64, TxnGuard>,
}

impl ShareServiceCore {
    pub fn new(
        epoch: u64,
        chain_id: u8,
        my_addr: AccountAddress,
        shares: Vec<MasterScalar>,
    ) -> Self {
        Self {
            epoch,
            chain_id,
            my_addr,
            shares,
            next_probe: 0,
            open: BTreeMap::new(),
            guards: HashMap::new(),
        }
    }

    /// Number of share transactions currently in the pool.
    pub fn pooled(&self) -> usize {
        self.guards.len()
    }

    /// One pass: find new registrations, then serve every open timelock that is due. Errors of the view are
    /// returned and the pass is retried on the next tick.
    pub fn tick(&mut self, view: &dyn TimelockChainView, pool: &VTxnPoolState) -> Result<()> {
        let (epoch, now_us) = view.epoch_and_time()?;
        if epoch != self.epoch {
            // The epoch changed: pending timelocks of this epoch expire. Serve nothing.
            self.guards.clear();
            self.open.clear();
            return Ok(());
        }
        while view.timelock(self.next_probe)?.is_some() {
            self.open.insert(self.next_probe, ());
            self.next_probe += 1;
        }
        let ids: Vec<u64> = self.open.keys().copied().collect();
        for id in ids {
            let Some(info) = view.timelock(id)? else {
                continue;
            };
            let finished = info.status != TimelockStatusResource::Pending
                || info.submitters.contains(&self.my_addr);
            if finished || info.epoch != self.epoch {
                self.guards.remove(&id);
                if info.status != TimelockStatusResource::Pending || info.epoch != self.epoch {
                    self.open.remove(&id);
                }
                continue;
            }
            if self.guards.contains_key(&id) {
                continue;
            }
            if share_service_held() {
                continue;
            }
            // The node rule: pending, this epoch, committed deadline passed, window open, identity as
            // computed from (chain, epoch, id, deadline).
            let derivable = match DerivableTimelock::authorize(
                &info.to_registry_timelock(),
                self.chain_id,
                self.epoch,
                now_us,
            ) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let shares = match derivable.derive_shares(&self.shares) {
                Ok(s) => s,
                Err(e) => {
                    warn!("[TimelockShare] cannot derive shares for timelock {id}: {e:?}");
                    continue;
                },
            };
            let shares = maybe_corrupt_for_failpoint(shares);
            let txn = ValidatorTransaction::TimelockShare(TimelockShare {
                epoch: self.epoch,
                timelock_id: id,
                author: self.my_addr,
                shares,
            });
            let guard = pool.put(
                Topic::TimelockShare {
                    epoch: self.epoch,
                    timelock_id: id,
                },
                Arc::new(txn),
                None,
            );
            counters::TIMELOCK_SHARES_SUBMITTED.inc();
            info!(
                epoch = self.epoch,
                timelock_id = id,
                "[TimelockShare] timelock share transaction put into the vtxn pool."
            );
            self.guards.insert(id, guard);
        }
        Ok(())
    }
}

/// Test hook: with the failpoint `timelock_share::corrupt` set, the first share is replaced by a different
/// valid curve point (the doubled share), so it parses but fails the pairing check. Only builds with the
/// `failpoints` feature of `fail` can trigger it. It is how the smoke test makes one validator misbehave.
fn maybe_corrupt_for_failpoint(shares: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    fail::fail_point!("timelock_share::corrupt", |_| {
        let mut bad = shares.clone();
        if let Some(first) = bad.first_mut() {
            if let Ok(doubled) = aptos_dkg::ibe::double_g1_share(first) {
                *first = doubled.to_vec();
            }
        }
        bad
    });
    shares
}

/// Test hook: with the failpoint `timelock_share::hold` set, the service derives and submits nothing. The
/// smoke test uses it to let a corrupted share arrive before any honest share.
fn share_service_held() -> bool {
    fail::fail_point!("timelock_share::hold", |_| true);
    false
}

/// Runs the service until the task is aborted.
pub async fn run(mut core: ShareServiceCore, view: DbTimelockChainView, pool: VTxnPoolState) {
    let mut ticker = tokio::time::interval(POLL_INTERVAL);
    loop {
        ticker.tick().await;
        if let Err(e) = core.tick(&view, &pool) {
            warn!("[TimelockShare] pass failed: {e:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_dkg::ibe::{
        compute_identity, decode_g1, decode_g2, encode_g2, master_public_key, random_scalar,
        verify_dk_share,
    };
    use aptos_validator_transaction_pool::TransactionFilter;
    use rand::{rngs::StdRng, SeedableRng};
    use std::{cell::RefCell, collections::BTreeMap};

    /// The failpoint registry is process global, so the tests of this module run one at a time.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    const CHAIN: u8 = 4;
    const EPOCH: u64 = 3;

    struct FakeView {
        epoch: RefCell<u64>,
        now_us: RefCell<u64>,
        rows: RefCell<BTreeMap<u64, TimelockInfoResource>>,
    }

    // The test view is only used from one thread.
    unsafe impl Send for FakeView {}

    impl TimelockChainView for FakeView {
        fn epoch_and_time(&self) -> Result<(u64, u64)> {
            Ok((*self.epoch.borrow(), *self.now_us.borrow()))
        }

        fn timelock(&self, id: u64) -> Result<Option<TimelockInfoResource>> {
            Ok(self.rows.borrow().get(&id).cloned())
        }
    }

    fn row(id: u64, deadline_us: u64) -> TimelockInfoResource {
        TimelockInfoResource {
            timelock_id: id,
            epoch: EPOCH,
            deadline_us,
            reveal_expiry_us: deadline_us + 600_000_000,
            identity: compute_identity(CHAIN, EPOCH, id, deadline_us).to_vec(),
            decryption_key: vec![],
            status: TimelockStatusResource::Pending,
            share_count: 0,
            reveal_threshold: 2,
            validator_indices: vec![],
            submitted_shares: vec![],
            validator_weights: vec![],
            submitters: vec![],
        }
    }

    fn pooled_share(pool: &VTxnPoolState) -> Vec<TimelockShare> {
        pool.pull(
            std::time::Instant::now() + Duration::from_secs(1),
            100,
            1 << 20,
            TransactionFilter::no_op(),
        )
        .into_iter()
        .filter_map(|t| match t {
            ValidatorTransaction::TimelockShare(s) => Some(s),
            _ => None,
        })
        .collect()
    }

    fn setup(now_us: u64) -> (FakeView, ShareServiceCore, VTxnPoolState, Vec<MasterScalar>) {
        let mut rng = StdRng::seed_from_u64(7);
        let secrets: Vec<MasterScalar> = (0..2).map(|_| random_scalar(&mut rng)).collect();
        let view = FakeView {
            epoch: RefCell::new(EPOCH),
            now_us: RefCell::new(now_us),
            rows: RefCell::new(BTreeMap::new()),
        };
        let me = AccountAddress::from_hex_literal("0xa11ce").unwrap();
        let core = ShareServiceCore::new(EPOCH, CHAIN, me, secrets.clone());
        (view, core, VTxnPoolState::default(), secrets)
    }

    #[test]
    fn serves_a_timelock_only_after_the_committed_deadline() {
        let _serial = serial();
        let (view, mut core, pool, secrets) = setup(500);
        view.rows.borrow_mut().insert(0, row(0, 1_000));
        core.tick(&view, &pool).unwrap();
        assert!(pooled_share(&pool).is_empty(), "before the deadline");
        *view.now_us.borrow_mut() = 1_000;
        core.tick(&view, &pool).unwrap();
        let shares = pooled_share(&pool);
        assert_eq!(shares.len(), 1);
        let share = &shares[0];
        assert_eq!((share.epoch, share.timelock_id), (EPOCH, 0));
        // Every share is the secret share times H1(id), so it verifies against g2^s.
        let id = compute_identity(CHAIN, EPOCH, 0, 1_000);
        for (s, bytes) in secrets.iter().zip(share.shares.iter()) {
            let vk = decode_g2(&encode_g2(&master_public_key(s))).unwrap();
            assert!(verify_dk_share(&decode_g1(bytes).unwrap(), &id, &vk));
        }
    }

    #[test]
    fn refuses_a_forged_identity_and_other_epochs() {
        let _serial = serial();
        let (view, mut core, pool, _) = setup(5_000);
        let mut forged = row(0, 1_000);
        forged.identity = vec![9u8; 32];
        let mut other_epoch = row(1, 1_000);
        other_epoch.epoch = EPOCH + 1;
        view.rows.borrow_mut().insert(0, forged);
        view.rows.borrow_mut().insert(1, other_epoch);
        core.tick(&view, &pool).unwrap();
        assert!(pooled_share(&pool).is_empty());
    }

    #[test]
    fn stops_once_the_registry_has_the_share_or_the_timelock_is_final() {
        let _serial = serial();
        let (view, mut core, pool, _) = setup(2_000);
        view.rows.borrow_mut().insert(0, row(0, 1_000));
        view.rows.borrow_mut().insert(1, row(1, 1_000));
        core.tick(&view, &pool).unwrap();
        assert_eq!(core.pooled(), 2);
        // Timelock 0: the registry now holds our share. Timelock 1: revealed by others.
        view.rows.borrow_mut().get_mut(&0).unwrap().submitters =
            vec![AccountAddress::from_hex_literal("0xa11ce").unwrap()];
        view.rows.borrow_mut().get_mut(&1).unwrap().status = TimelockStatusResource::Revealed;
        core.tick(&view, &pool).unwrap();
        assert_eq!(core.pooled(), 0);
        assert!(pooled_share(&pool).is_empty());
    }

    #[test]
    fn picks_up_registrations_made_later_and_drops_everything_at_the_epoch_change() {
        let _serial = serial();
        let (view, mut core, pool, _) = setup(2_000);
        core.tick(&view, &pool).unwrap();
        view.rows.borrow_mut().insert(0, row(0, 1_000));
        view.rows.borrow_mut().insert(1, row(1, 1_500));
        core.tick(&view, &pool).unwrap();
        assert_eq!(core.pooled(), 2);
        *view.epoch.borrow_mut() = EPOCH + 1;
        core.tick(&view, &pool).unwrap();
        assert_eq!(core.pooled(), 0);
        assert!(pooled_share(&pool).is_empty());
    }

    #[cfg(feature = "failpoints")]
    #[test]
    fn the_hold_failpoint_pauses_submission_until_it_is_removed() {
        let _serial = serial();
        let (view, mut core, pool, _) = setup(2_000);
        view.rows.borrow_mut().insert(0, row(0, 1_000));
        let scenario = fail::FailScenario::setup();
        fail::cfg("timelock_share::hold", "return").unwrap();
        core.tick(&view, &pool).unwrap();
        assert!(pooled_share(&pool).is_empty(), "held");
        fail::remove("timelock_share::hold");
        core.tick(&view, &pool).unwrap();
        scenario.teardown();
        assert_eq!(pooled_share(&pool).len(), 1, "released");
    }

    #[cfg(feature = "failpoints")]
    #[test]
    fn the_failpoint_corrupts_a_share_into_a_different_valid_point() {
        let _serial = serial();
        let (view, mut core, pool, secrets) = setup(2_000);
        view.rows.borrow_mut().insert(0, row(0, 1_000));
        let scenario = fail::FailScenario::setup();
        fail::cfg("timelock_share::corrupt", "return").unwrap();
        core.tick(&view, &pool).unwrap();
        fail::remove("timelock_share::corrupt");
        scenario.teardown();
        let share = pooled_share(&pool).remove(0);
        let id = compute_identity(CHAIN, EPOCH, 0, 1_000);
        let vk = decode_g2(&encode_g2(&master_public_key(&secrets[0]))).unwrap();
        // Still a valid point, but it no longer verifies.
        let first = decode_g1(&share.shares[0]).unwrap();
        assert!(!verify_dk_share(&first, &id, &vk));
        let second_vk = decode_g2(&encode_g2(&master_public_key(&secrets[1]))).unwrap();
        assert!(verify_dk_share(
            &decode_g1(&share.shares[1]).unwrap(),
            &id,
            &second_vk
        ));
    }
}
