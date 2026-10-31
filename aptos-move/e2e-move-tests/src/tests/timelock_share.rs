// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Handler level tests of the `TimelockShare` validator transaction with the Boneh-Franklin share
//! verifier installed. A `TimelockShare` goes through `AptosVM` block execution against the real framework:
//! registry, epoch config with verification keys, committed block time.
//!
//! The verifier is a process wide static, so the tests of this module take one lock.

use crate::MoveHarness;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use aptos_dkg::ibe::{
    compute_identity, derive_decryption_key, encode_g1, encode_g2, master_public_key,
    random_scalar, MasterScalar,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    contract_event::ContractEvent,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    validator_txn::{TimelockShare, ValidatorTransaction},
};
use aptos_vm::validator_txns::{
    install_boneh_franklin_share_verifier, set_timelock_share_verifier,
};
use move_core_types::value::MoveValue;
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

const MIN: u64 = 60_000_000;
const EPOCH_INTERVAL: u64 = 7_200_000_000;
const DEADLINE: u64 = 20 * MIN;

/// A chunky-DKG-shaped dealing of one epoch.
struct Dealing {
    secret: MasterScalar,
    shares: Vec<MasterScalar>,
    weights: Vec<u64>,
}

impl Dealing {
    fn new(weights: &[u64], threshold: usize, seed: u64) -> Self {
        let total = weights.iter().sum::<u64>() as usize;
        let mut rng = StdRng::seed_from_u64(seed);
        let secret = random_scalar(&mut rng);
        let config = ShamirThresholdConfig::<MasterScalar>::new(threshold, total);
        let (_, shares) = config.sample_polynomial_and_compute_shares(secret, &mut rng);
        Self {
            secret,
            shares,
            weights: weights.to_vec(),
        }
    }

    fn mpk(&self) -> Vec<u8> {
        encode_g2(&master_public_key(&self.secret)).to_vec()
    }

    fn vks(&self) -> Vec<Vec<u8>> {
        self.shares
            .iter()
            .map(|s| encode_g2(&master_public_key(s)).to_vec())
            .collect()
    }

    /// Shares of `validator` for `identity`.
    fn shares_for(&self, validator: usize, identity: &[u8]) -> Vec<Vec<u8>> {
        let start: u64 = self.weights[..validator].iter().sum();
        (0..self.weights[validator])
            .map(|j| {
                let s = &self.shares[(start + j) as usize];
                encode_g1(&derive_decryption_key(s, identity).unwrap()).to_vec()
            })
            .collect()
    }
}

struct Env {
    h: MoveHarness,
    author: AccountAddress,
    epoch: u64,
    chain_id: u8,
    timelock_id: u64,
}

impl Env {
    /// Epoch config from `dealing` (threshold in weight units), one registered timelock with a deadline
    /// of 60 minutes. Block time stays at its start value.
    fn new(dealing: &Dealing, threshold: u64) -> Self {
        aptos_logger::Logger::init_for_testing();
        let mut h = MoveHarness::new();
        let validators = ValidatorSet::fetch_config(&h.executor.get_state_view())
            .expect("validator set read")
            .expect("validator set")
            .payload()
            .map(|v| *v.account_address())
            .collect::<Vec<_>>();
        let author = validators[0];
        let epoch = ConfigurationResource::fetch_config(&h.executor.get_state_view())
            .expect("configuration read")
            .expect("configuration")
            .epoch();
        let chain_id = h.executor.get_chain_id().id();

        h.executor
            .exec("ibe_config", "set_epoch_config", vec![], vec![
                MoveValue::Signer(CORE_CODE_ADDRESS)
                    .simple_serialize()
                    .unwrap(),
                bcs::to_bytes(&epoch).unwrap(),
                bcs::to_bytes(&dealing.mpk()).unwrap(),
                bcs::to_bytes(&threshold).unwrap(),
                bcs::to_bytes(&dealing.weights).unwrap(),
                bcs::to_bytes(&EPOCH_INTERVAL).unwrap(),
                bcs::to_bytes(&vec![1u8; 32]).unwrap(),
                bcs::to_bytes(&vec![2u8; 32]).unwrap(),
                bcs::to_bytes(&dealing.vks()).unwrap(),
            ]);
        let user = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
        let status = h.run_entry_function(
            &user,
            str::parse("0x1::ibe_config::register_timelock").unwrap(),
            vec![],
            vec![bcs::to_bytes(&DEADLINE).unwrap()],
        );
        assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        Self {
            h,
            author,
            epoch,
            chain_id,
            timelock_id: 0,
        }
    }

    fn identity(&self) -> Vec<u8> {
        compute_identity(self.chain_id, self.epoch, self.timelock_id, DEADLINE).to_vec()
    }

    fn advance_to(&mut self, time_us: u64) {
        self.h.executor.new_block_with_timestamp(time_us);
    }

    /// Runs one `TimelockShare` and applies its write set.
    fn submit(&mut self, epoch: u64, shares: Vec<Vec<u8>>) -> TransactionOutput {
        let txn =
            Transaction::ValidatorTransaction(ValidatorTransaction::TimelockShare(TimelockShare {
                epoch,
                timelock_id: self.timelock_id,
                author: self.author,
                shares,
            }));
        let mut outputs = self
            .h
            .executor
            .execute_transaction_block(vec![txn])
            .unwrap();
        let output = outputs.pop().unwrap();
        if !output.status().is_discarded() {
            self.h.executor.apply_write_set(output.write_set());
        }
        output
    }

    fn view<T: serde::de::DeserializeOwned>(&mut self, f: &str, index: usize) -> T {
        let out = self.h.execute_view_function(
            str::parse(&format!("0x1::ibe_config::{f}")).unwrap(),
            vec![],
            vec![bcs::to_bytes(&self.timelock_id).unwrap()],
        );
        bcs::from_bytes(&out.values.unwrap()[index]).unwrap()
    }

    fn share_count(&mut self) -> u64 {
        self.view("get_timelock", 3)
    }

    fn is_revealed(&mut self) -> bool {
        self.view("is_revealed", 0)
    }

    fn status(&mut self) -> u8 {
        self.view("get_status", 0)
    }

    fn expiry_reason(&mut self) -> u8 {
        self.view("get_expiry_reason", 0)
    }
}

fn event_named(output: &TransactionOutput, name: &str) -> Vec<ContractEvent> {
    output
        .events()
        .iter()
        .filter(|e| {
            e.type_tag()
                .to_canonical_string()
                .contains(&format!("ibe_config::{name}"))
        })
        .cloned()
        .collect()
}

fn discarded(output: &TransactionOutput) -> bool {
    output.status().is_discarded()
}

fn kept_ok(output: &TransactionOutput) -> bool {
    matches!(
        output.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    )
}

/// Validator 0 holds weight 2 of total 4 with threshold 3, so one submission stays below the threshold.
fn partial() -> (Dealing, u64) {
    (Dealing::new(&[2, 2], 3, 11), 3)
}

#[test]
fn no_verifier_installed_fails_closed() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (d, t) = partial();
    let mut e = Env::new(&d, t);
    e.advance_to(DEADLINE);
    set_timelock_share_verifier(None);
    let shares = d.shares_for(0, &e.identity());
    let out = e.submit(e.epoch, shares);
    assert!(discarded(&out));
    assert_eq!(e.share_count(), 0);
}

#[test]
fn valid_shares_are_stored_without_revealing_below_threshold() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    let (d, t) = partial();
    let mut e = Env::new(&d, t);
    e.advance_to(DEADLINE);
    let shares = d.shares_for(0, &e.identity());
    let out = e.submit(e.epoch, shares.clone());
    assert!(kept_ok(&out), "{:?}", out.status());
    assert_eq!(e.share_count(), 2);
    assert!(!e.is_revealed());
    // The same author cannot take a second slot.
    let again = e.submit(e.epoch, shares);
    assert!(discarded(&again));
    assert_eq!(e.share_count(), 2);
    set_timelock_share_verifier(None);
}

#[test]
fn valid_shares_reach_threshold_and_reveal_the_checked_key() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    // One validator of weight 3 holds the threshold of 2 alone.
    let d = Dealing::new(&[3], 2, 12);
    let mut e = Env::new(&d, 2);
    e.advance_to(DEADLINE);
    let id = e.identity();
    let out = e.submit(e.epoch, d.shares_for(0, &id));
    assert!(kept_ok(&out), "{:?}", out.status());
    assert!(e.is_revealed());
    let dk: Vec<u8> = e.view("get_decryption_key", 0);
    let expected = encode_g1(&derive_decryption_key(&d.secret, &id).unwrap()).to_vec();
    assert_eq!(dk, expected);
    assert_eq!(event_named(&out, "TimelockRevealEvent").len(), 1);
    set_timelock_share_verifier(None);
}

/// Every bad share set is refused, nothing is stored, the author's slot stays free, and one event names
/// the author. A good set from the same author is accepted right after.
#[test]
fn bad_shares_are_rejected_keep_the_slot_free_and_emit_an_event() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    let (d, t) = partial();
    let mut e = Env::new(&d, t);
    e.advance_to(DEADLINE);
    let id = e.identity();
    let good = d.shares_for(0, &id);

    let other_identity = compute_identity(e.chain_id, e.epoch, 99, DEADLINE);
    let other_epoch = compute_identity(e.chain_id, e.epoch + 1, e.timelock_id, DEADLINE);
    let foreign = Dealing::new(&[2, 2], 3, 777);
    let mut tampered = good.clone();
    tampered.swap(0, 1);
    let cases: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("another player's shares", d.shares_for(1, &id)),
        ("swapped players", tampered),
        ("another identity", d.shares_for(0, &other_identity)),
        (
            "an identity of another epoch",
            d.shares_for(0, &other_epoch),
        ),
        ("another DKG secret", foreign.shares_for(0, &id)),
    ];
    for (name, shares) in cases {
        let out = e.submit(e.epoch, shares);
        assert!(kept_ok(&out), "{name}: {:?}", out.status());
        let events = event_named(&out, "TimelockShareRejectedEvent");
        assert_eq!(events.len(), 1, "{name}");
        let data = events[0].event_data().to_vec();
        assert!(
            data.windows(32).any(|w| w == e.author.as_ref()),
            "{name}: the event names the author"
        );
        assert_eq!(e.share_count(), 0, "{name}: nothing stored");
    }
    // The slot is still free.
    let out = e.submit(e.epoch, good);
    assert!(kept_ok(&out));
    assert!(event_named(&out, "TimelockShareRejectedEvent").is_empty());
    assert_eq!(e.share_count(), 2);
    set_timelock_share_verifier(None);
}

#[test]
fn non_subgroup_and_malformed_points_are_discarded_before_any_state_change() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    let (d, t) = partial();
    let mut e = Env::new(&d, t);
    e.advance_to(DEADLINE);
    let good = d.shares_for(0, &e.identity());

    // A compressed point with an x that is on the curve but outside the prime-order subgroup:
    // flag bits set for a compressed point, then scan x until the point is off the subgroup.
    let non_subgroup = {
        use ark_bls12_381::{Fq, G1Affine};
        use ark_serialize::{CanonicalSerialize, Compress};
        let mut x = Fq::from(1u64);
        loop {
            if let Some(p) = G1Affine::get_point_from_x_unchecked(x, false)
                .filter(|p| p.is_on_curve() && !p.is_in_correct_subgroup_assuming_on_curve())
            {
                let mut v = vec![];
                p.serialize_with_mode(&mut v, Compress::Yes).unwrap();
                break v;
            }
            x += Fq::from(1u64);
        }
    };
    let mut identity_point = vec![0u8; 48];
    identity_point[0] = 0xC0;
    for bad in [non_subgroup, identity_point, vec![0u8; 47]] {
        let mut shares = good.clone();
        shares[1] = bad;
        let out = e.submit(e.epoch, shares);
        assert!(discarded(&out));
        assert!(event_named(&out, "TimelockShareRejectedEvent").is_empty());
        assert_eq!(e.share_count(), 0);
    }
    set_timelock_share_verifier(None);
}

#[test]
fn wrong_epoch_wrong_count_and_early_submission_are_discarded() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    let (d, t) = partial();
    let mut e = Env::new(&d, t);
    let good = d.shares_for(0, &e.identity());

    // The deadline has not passed in the committed block time.
    let early = e.submit(e.epoch, good.clone());
    assert!(discarded(&early));
    assert_eq!(e.share_count(), 0);

    e.advance_to(DEADLINE - 1);
    assert!(discarded(&e.submit(e.epoch, good.clone())));
    e.advance_to(DEADLINE);

    // Epoch different from the current epoch.
    assert!(discarded(&e.submit(e.epoch + 1, good.clone())));
    // Weight and threshold come from the config: one share instead of two is refused.
    assert!(discarded(&e.submit(e.epoch, good[..1].to_vec())));
    assert!(discarded(&e.submit(e.epoch, vec![good[0].clone(); 3])));
    assert_eq!(e.share_count(), 0);
    // And the right submission still works.
    assert!(kept_ok(&e.submit(e.epoch, good)));
    assert_eq!(e.share_count(), 2);
    set_timelock_share_verifier(None);
}

/// The shares are valid per player but the epoch's mpk belongs to another secret, so the rebuilt key fails
/// `e(dk, g2) = e(H1(id), mpk)`. The key must not be stored, and the status is `Expired(key-check-failed)`.
#[test]
fn rebuilt_key_that_fails_the_mpk_check_expires_and_is_not_stored() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    install_boneh_franklin_share_verifier();
    let d = Dealing::new(&[3], 2, 13);
    let other = Dealing::new(&[3], 2, 14);
    let mut e = Env::new(&d, 2);
    // Replace the config: verification keys of `d` (so the shares verify), mpk of another secret.
    e.h.executor
        .exec("ibe_config", "set_epoch_config", vec![], vec![
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
            bcs::to_bytes(&e.epoch).unwrap(),
            bcs::to_bytes(&other.mpk()).unwrap(),
            bcs::to_bytes(&2u64).unwrap(),
            bcs::to_bytes(&d.weights).unwrap(),
            bcs::to_bytes(&EPOCH_INTERVAL).unwrap(),
            bcs::to_bytes(&vec![1u8; 32]).unwrap(),
            bcs::to_bytes(&vec![2u8; 32]).unwrap(),
            bcs::to_bytes(&d.vks()).unwrap(),
        ]);
    e.advance_to(DEADLINE);
    let out = e.submit(e.epoch, d.shares_for(0, &e.identity()));
    assert!(kept_ok(&out), "{:?}", out.status());
    assert!(!e.is_revealed());
    assert_eq!(e.status(), 2, "Expired");
    assert_eq!(e.expiry_reason(), 3, "key-check-failed");
    assert_eq!(event_named(&out, "TimelockExpiredEvent").len(), 1);
    assert!(event_named(&out, "TimelockRevealEvent").is_empty());
    let key = e.h.execute_view_function(
        str::parse("0x1::ibe_config::get_decryption_key").unwrap(),
        vec![],
        vec![bcs::to_bytes(&e.timelock_id).unwrap()],
    );
    assert!(key.values.is_err(), "no decryption key is stored");
    set_timelock_share_verifier(None);
}
