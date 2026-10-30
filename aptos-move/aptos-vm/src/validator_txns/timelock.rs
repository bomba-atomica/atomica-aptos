// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Handler of the `TimelockShare` validator transaction.
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` sections 5, 7 and 8.
//!
//! Flow of one transaction, in one VM session:
//!
//! 1. Cheap structural checks and validated G1 decoding of every share (on curve, prime-order subgroup,
//!    not the identity). Nothing is stored yet.
//! 2. `ibe_config::submit_dk_shares` stores the shares. Its Move checks are cheap: current epoch, the
//!    timelock's epoch, pending status, deadline passed in the committed block time, reveal window, one
//!    slot per validator, exact share count. Any Move abort discards the transaction.
//! 3. The share check. A [`TimelockShareVerifier`] must accept the shares against the verification keys
//!    of the author's virtual players, which the handler reads from `TimelockEpochConfig` through
//!    `ibe_config::get_share_verification_keys`. [`BonehFranklinShareVerifier`] is the real verifier and the
//!    node installs it at startup ([`install_boneh_franklin_share_verifier`]). **With no verifier installed
//!    every share is rejected (fail closed).** When the check fails the session is dropped, so the shares
//!    are not stored and the author's slot stays free. A fresh session then only emits
//!    `TimelockShareRejectedEvent` naming the author, and the transaction is kept.
//! 4. If verified weight reached the threshold, the key is rebuilt in Rust and checked with
//!    `e(dk, g2) = e(H1(id), mpk)`. Only a key that passes goes to `finalize_timelock_reveal`, which checks
//!    it again in a native. A key that fails sets `Expired(key-check-failed)` through
//!    `fail_timelock_key_check`.
//!
//! A discarded transaction leaves no state, so a rejected share never takes the author's slot.

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionExt, SessionId},
    system_module_names::{
        FAIL_TIMELOCK_KEY_CHECK_FUNCTION, FINALIZE_TIMELOCK_REVEAL_FUNCTION, GET_IDENTITY_FUNCTION,
        GET_MPK_FUNCTION, GET_RECONSTRUCTION_INPUTS_FUNCTION, GET_SHARE_VERIFICATION_KEYS_FUNCTION,
        GET_TIMELOCK_EPOCH_FUNCTION, IBE_CONFIG_MODULE, IS_RECONSTRUCTABLE_FUNCTION,
        REPORT_REJECTED_SHARE_FUNCTION, SUBMIT_DK_SHARES_FUNCTION,
    },
    AptosVM,
};
use aptos_dkg::ibe;
use aptos_logger::{info, warn};
use aptos_types::{
    on_chain_config::{ConfigurationResource, OnChainConfig},
    transaction::TransactionStatus,
    validator_txn::TimelockShare,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::SerializedReturnValues,
};
use move_vm_types::gas::UnmeteredGasMeter;
use serde::de::DeserializeOwned;
use std::sync::{Arc, RwLock};

// Codes of discarded transactions. Move equivalents: `errors::invalid_argument(*)` is 0x1, and
// `errors::invalid_state(*)` is 0x3, in the high bits of the category.
#[derive(Debug)]
enum ExpectedFailure {
    InvalidShareEncoding = 0x10401,
    EpochNotCurrent = 0x10402,
    TimelockEpochMismatch = 0x10403,
    /// Kept for the code table. A failed share check now keeps the transaction with a rejection event.
    #[allow(dead_code)]
    ShareCheckFailed = 0x10404,
    /// The Move registry refused the shares. The abort code is logged.
    RegistryAbort = 0x10405,
    /// No verifier is installed. Fail closed.
    ShareVerifierUnavailable = 0x30401,
    MissingResourceConfiguration = 0x30402,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

/// What a verifier needs to check the shares of one author for one timelock. All fields come from chain
/// state or from the transaction. The identity and the master public key are the registry's.
#[derive(Debug)]
pub struct ShareCheckRequest<'a> {
    pub epoch: u64,
    pub timelock_id: u64,
    /// The validator that submitted the shares. The verifier maps it to the validator index and to the
    /// verification keys of its virtual players.
    pub author: AccountAddress,
    /// `ibe_config::get_identity(timelock_id)`: the 32 byte identity `id` with `Q = H1(id)`.
    pub identity: &'a [u8],
    /// The master public key of the epoch (compressed G2, 96 bytes).
    pub mpk: &'a [u8],
    /// The verification keys `g2^{s_p}` (compressed G2) of the author's virtual players, in player order,
    /// read from `TimelockEpochConfig` on chain.
    pub verification_keys: &'a [Vec<u8>],
    /// One compressed G1 share per virtual player of the author, in player order. Each already passed
    /// validated decoding (`aptos_dkg::ibe::decode_g1`).
    pub shares: &'a [Vec<u8>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareCheckError(pub String);

/// The interface the share verification feature fills in.
///
/// An implementation must return `Ok(())` only when, for every virtual player `p` of `author`,
/// `e(H1(identity), vk_p) = e(share_p, g2)`. `vk_p` must come from the epoch's timelock DKG output, and the
/// result must be a deterministic function of chain state and the request, because every validator runs
/// this inside block execution. It must not read local files, clocks or node-local caches that can differ
/// between validators.
pub trait TimelockShareVerifier: Send + Sync {
    fn verify_shares(&self, request: &ShareCheckRequest<'_>) -> Result<(), ShareCheckError>;
}

/// The Boneh-Franklin share check of spec section 5: for every virtual player `p` of the author,
/// `e(H1(identity), vk_p) = e(share_p, g2)`, over validated points. Stateless and a pure function of the
/// request, which the handler fills from chain state, so all validators agree.
#[derive(Debug, Default, Clone, Copy)]
pub struct BonehFranklinShareVerifier;

impl TimelockShareVerifier for BonehFranklinShareVerifier {
    fn verify_shares(&self, request: &ShareCheckRequest<'_>) -> Result<(), ShareCheckError> {
        ibe::verify_player_shares(request.identity, request.shares, request.verification_keys)
            .map_err(|e| ShareCheckError(format!("{:?}", e)))
    }
}

/// Installs [`BonehFranklinShareVerifier`]. The node calls this before it executes blocks.
pub fn install_boneh_franklin_share_verifier() {
    set_timelock_share_verifier(Some(Arc::new(BonehFranklinShareVerifier)));
}

static SHARE_VERIFIER: RwLock<Option<Arc<dyn TimelockShareVerifier>>> = RwLock::new(None);

/// Installs (or, with `None`, removes) the share verifier. With none installed the handler rejects every
/// share. The node installs the verifier before it executes blocks.
pub fn set_timelock_share_verifier(verifier: Option<Arc<dyn TimelockShareVerifier>>) {
    *SHARE_VERIFIER
        .write()
        .expect("timelock share verifier lock poisoned") = verifier;
}

fn current_share_verifier() -> Option<Arc<dyn TimelockShareVerifier>> {
    SHARE_VERIFIER
        .read()
        .expect("timelock share verifier lock poisoned")
        .clone()
}

/// Validated decoding of every share. Returns the index of the first bad share.
fn validate_share_encoding(shares: &[Vec<u8>]) -> Result<(), usize> {
    if shares.is_empty() {
        return Err(0);
    }
    for (i, share) in shares.iter().enumerate() {
        let _point = ibe::decode_g1(share).map_err(|_| i)?;
    }
    Ok(())
}

/// The outcome of rebuilding the key from the stored shares.
#[derive(Debug, PartialEq, Eq)]
enum RebuiltKey {
    /// The key passed `e(dk, g2) = e(H1(id), mpk)`.
    Checked(Vec<u8>),
    /// Reconstruction or the pairing check failed.
    Failed,
}

/// Rebuilds the key by weighted Lagrange interpolation and checks it against the master public key.
fn rebuild_and_check_key(
    validator_indices: &[u64],
    dk_shares: &[Vec<Vec<u8>>],
    weights: &[u64],
    threshold: u64,
    total_weight: u64,
    identity: &[u8],
    mpk: &[u8],
) -> RebuiltKey {
    let Ok(mpk) = ibe::decode_g2(mpk) else {
        return RebuiltKey::Failed;
    };
    let Ok(dk) = ibe::reconstruct_ibe_dk_from_g1_shares(
        validator_indices,
        dk_shares,
        weights,
        threshold,
        total_weight,
    ) else {
        return RebuiltKey::Failed;
    };
    if ibe::verify_decryption_key(&dk, identity, &mpk) {
        RebuiltKey::Checked(ibe::encode_g1(&dk).to_vec())
    } else {
        RebuiltKey::Failed
    }
}

fn decode_return<T: DeserializeOwned>(
    values: &SerializedReturnValues,
    index: usize,
) -> Result<T, ExecutionFailure> {
    let (bytes, _) = values.return_values.get(index).ok_or_else(|| {
        ExecutionFailure::Unexpected(VMStatus::error(
            StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
            Some("missing return value from ibe_config".to_string()),
        ))
    })?;
    bcs::from_bytes(bytes).map_err(|_| {
        ExecutionFailure::Unexpected(VMStatus::error(
            StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
            Some("undecodable return value from ibe_config".to_string()),
        ))
    })
}

/// Calls a function of `ibe_config`. A Move abort becomes an expected failure (the transaction is
/// discarded). Anything else is an unexpected failure.
fn call_ibe_config(
    session: &mut SessionExt<'_, impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    log_context: &AdapterLogSchema,
    function: &IdentStr,
    args: Vec<MoveValue>,
) -> Result<SerializedReturnValues, ExecutionFailure> {
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    session
        .execute_function_bypass_visibility(
            &IBE_CONFIG_MODULE,
            function,
            vec![],
            serialize_values(&args),
            &mut gas_meter,
            &mut TraversalContext::new(&traversal_storage),
            module_storage,
        )
        .map_err(|e| {
            if let VMStatus::MoveAbort { code, .. } = e.clone().into_vm_status() {
                warn!(
                    "timelock share: ibe_config::{} aborted with code {}",
                    function, code
                );
                return ExecutionFailure::Expected(ExpectedFailure::RegistryAbort);
            }
            ExecutionFailure::Unexpected(
                expect_only_successful_execution(e, function.as_str(), log_context)
                    .expect_err("expect_only_successful_execution always returns Err"),
            )
        })
}

impl AptosVM {
    pub(crate) fn process_timelock_share(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        timelock_share: TimelockShare,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        info!(
            "Process timelock share: epoch {}, timelock {}, author {}",
            timelock_share.epoch, timelock_share.timelock_id, timelock_share.author
        );
        match self.process_timelock_share_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            timelock_share,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(ExecutionFailure::Expected(failure)) => {
                warn!("Timelock share rejected: {:?}", failure);
                Ok((
                    VMStatus::MoveAbort {
                        location: AbortLocation::Script,
                        code: failure as u64,
                        message: None,
                    },
                    VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
                ))
            },
            Err(ExecutionFailure::Unexpected(vm_status)) => {
                warn!("Timelock share: unexpected failure: {:?}", vm_status);
                Err(vm_status)
            },
        }
    }

    /// Keeps a transaction whose only effect is `TimelockShareRejectedEvent`. If the registry refuses even
    /// that (for example the window closed), the transaction is discarded.
    #[allow(clippy::too_many_arguments)]
    fn report_rejected_share(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        timelock_id: u64,
        author: AccountAddress,
        epoch: u64,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let mut session = self.new_session(resolver, session_id, None);
        call_ibe_config(
            &mut session,
            module_storage,
            log_context,
            REPORT_REJECTED_SHARE_FUNCTION,
            vec![
                MoveValue::U64(timelock_id),
                MoveValue::Address(author),
                MoveValue::U64(epoch),
            ],
        )?;
        let output = get_system_transaction_output(
            session,
            module_storage,
            &self
                .storage_gas_params(log_context)
                .map_err(ExecutionFailure::Unexpected)?
                .change_set_configs,
        )
        .map_err(ExecutionFailure::Unexpected)?;
        Ok((VMStatus::Executed, output))
    }

    fn process_timelock_share_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        timelock_share: TimelockShare,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let TimelockShare {
            epoch,
            timelock_id,
            author,
            shares,
        } = timelock_share;

        // 1. Epoch and encoding, before any state is touched.
        let config = ConfigurationResource::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceConfiguration,
            ))?;
        if epoch != config.epoch() {
            return Err(ExecutionFailure::Expected(ExpectedFailure::EpochNotCurrent));
        }
        if validate_share_encoding(&shares).is_err() {
            return Err(ExecutionFailure::Expected(
                ExpectedFailure::InvalidShareEncoding,
            ));
        }
        // Fail closed before spending any VM or pairing work.
        let verifier = current_share_verifier().ok_or(ExecutionFailure::Expected(
            ExpectedFailure::ShareVerifierUnavailable,
        ))?;

        let mut session = self.new_session(resolver, session_id.clone(), None);

        // 2. Store the shares. The Move registry enforces epoch, status, time window, slot and count.
        call_ibe_config(
            &mut session,
            module_storage,
            log_context,
            SUBMIT_DK_SHARES_FUNCTION,
            vec![
                MoveValue::U64(timelock_id),
                MoveValue::Vector(
                    shares
                        .iter()
                        .map(|s| MoveValue::vector_u8(s.clone()))
                        .collect(),
                ),
                MoveValue::Address(author),
                MoveValue::U64(epoch),
            ],
        )?;

        // 3. The share check against the verification keys.
        let timelock_epoch: u64 = decode_return(
            &call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                GET_TIMELOCK_EPOCH_FUNCTION,
                vec![MoveValue::U64(timelock_id)],
            )?,
            0,
        )?;
        if timelock_epoch != epoch {
            return Err(ExecutionFailure::Expected(
                ExpectedFailure::TimelockEpochMismatch,
            ));
        }
        let identity: Vec<u8> = decode_return(
            &call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                GET_IDENTITY_FUNCTION,
                vec![MoveValue::U64(timelock_id)],
            )?,
            0,
        )?;
        let mpk: Vec<u8> = decode_return(
            &call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                GET_MPK_FUNCTION,
                vec![],
            )?,
            0,
        )?;
        let verification_keys: Vec<Vec<u8>> = decode_return(
            &call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                GET_SHARE_VERIFICATION_KEYS_FUNCTION,
                vec![MoveValue::Address(author)],
            )?,
            0,
        )?;
        let request = ShareCheckRequest {
            epoch,
            timelock_id,
            author,
            identity: &identity,
            mpk: &mpk,
            verification_keys: &verification_keys,
            shares: &shares,
        };
        if let Err(e) = verifier.verify_shares(&request) {
            warn!(
                "Timelock share check failed: author {}, timelock {}: {}",
                author, timelock_id, e.0
            );
            // Drop the session: the shares are not stored and the slot stays free. A fresh session
            // only records the rejection in an event that names the author.
            drop(session);
            return self.report_rejected_share(
                resolver,
                module_storage,
                log_context,
                session_id,
                timelock_id,
                author,
                epoch,
            );
        }

        // 4. Reveal, once verified weight reached the threshold.
        let reconstructable: bool = decode_return(
            &call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                IS_RECONSTRUCTABLE_FUNCTION,
                vec![MoveValue::U64(timelock_id)],
            )?,
            0,
        )?;
        if reconstructable {
            let inputs = call_ibe_config(
                &mut session,
                module_storage,
                log_context,
                GET_RECONSTRUCTION_INPUTS_FUNCTION,
                vec![MoveValue::U64(timelock_id)],
            )?;
            let validator_indices: Vec<u64> = decode_return(&inputs, 0)?;
            let dk_shares: Vec<Vec<Vec<u8>>> = decode_return(&inputs, 1)?;
            let weights: Vec<u64> = decode_return(&inputs, 2)?;
            let threshold: u64 = decode_return(&inputs, 3)?;
            let total_weight: u64 = decode_return(&inputs, 4)?;
            let input_identity: Vec<u8> = decode_return(&inputs, 5)?;
            match rebuild_and_check_key(
                &validator_indices,
                &dk_shares,
                &weights,
                threshold,
                total_weight,
                &input_identity,
                &mpk,
            ) {
                RebuiltKey::Checked(dk) => {
                    let revealed: bool = decode_return(
                        &call_ibe_config(
                            &mut session,
                            module_storage,
                            log_context,
                            FINALIZE_TIMELOCK_REVEAL_FUNCTION,
                            vec![MoveValue::U64(timelock_id), MoveValue::vector_u8(dk)],
                        )?,
                        0,
                    )?;
                    info!(
                        "Timelock {} reveal finalized: revealed={}",
                        timelock_id, revealed
                    );
                },
                RebuiltKey::Failed => {
                    warn!(
                        "Timelock {}: rebuilt key failed the pairing check, expiring (key-check-failed). Alert.",
                        timelock_id
                    );
                    call_ibe_config(
                        &mut session,
                        module_storage,
                        log_context,
                        FAIL_TIMELOCK_KEY_CHECK_FUNCTION,
                        vec![MoveValue::U64(timelock_id)],
                    )?;
                },
            }
        }

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self
                .storage_gas_params(log_context)
                .map_err(ExecutionFailure::Unexpected)?
                .change_set_configs,
        )
        .map_err(ExecutionFailure::Unexpected)?;

        Ok((VMStatus::Executed, output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_dkg::ibe::{
        compute_identity, derive_decryption_key, encode_g1, encode_g2, master_public_key,
        random_scalar,
    };
    use rand::{rngs::StdRng, SeedableRng};

    fn id() -> [u8; 32] {
        compute_identity(4, 3, 11, 1_700_000_000_000_000)
    }

    #[test]
    fn single_player_key_rebuilds_and_passes_pairing_check() {
        let mut rng = StdRng::seed_from_u64(1);
        let s = random_scalar(&mut rng);
        let mpk = encode_g2(&master_public_key(&s)).to_vec();
        let dk = encode_g1(&derive_decryption_key(&s, &id()).unwrap()).to_vec();
        let out = rebuild_and_check_key(&[0], &[vec![dk.clone()]], &[1], 1, 1, &id(), &mpk);
        assert_eq!(out, RebuiltKey::Checked(dk));
    }

    #[test]
    fn two_player_key_rebuilds_from_degree_one_sharing() {
        // f(x) = s + a x on the two point domain {1, -1}: shares are s + a and s - a.
        let mut rng = StdRng::seed_from_u64(2);
        let s = random_scalar(&mut rng);
        let a = random_scalar(&mut rng);
        let mpk = encode_g2(&master_public_key(&s)).to_vec();
        let sh = |x| encode_g1(&derive_decryption_key(&x, &id()).unwrap()).to_vec();
        let shares = vec![vec![sh(s + a)], vec![sh(s - a)]];
        let expected = encode_g1(&derive_decryption_key(&s, &id()).unwrap()).to_vec();
        let out = rebuild_and_check_key(&[0, 1], &shares, &[1, 1], 2, 2, &id(), &mpk);
        assert_eq!(out, RebuiltKey::Checked(expected));
    }

    #[test]
    fn rebuilt_key_from_a_bad_share_fails_the_check() {
        let mut rng = StdRng::seed_from_u64(3);
        let s = random_scalar(&mut rng);
        let a = random_scalar(&mut rng);
        let wrong = random_scalar(&mut rng);
        let mpk = encode_g2(&master_public_key(&s)).to_vec();
        let sh = |x| encode_g1(&derive_decryption_key(&x, &id()).unwrap()).to_vec();
        // Second share is not f(-1).
        let shares = vec![vec![sh(s + a)], vec![sh(wrong)]];
        let out = rebuild_and_check_key(&[0, 1], &shares, &[1, 1], 2, 2, &id(), &mpk);
        assert_eq!(out, RebuiltKey::Failed);
        // A key for another identity fails against this identity.
        let other = compute_identity(4, 3, 12, 1_700_000_000_000_000);
        let dk_other = encode_g1(&derive_decryption_key(&s, &other).unwrap()).to_vec();
        let out = rebuild_and_check_key(&[0], &[vec![dk_other]], &[1], 1, 1, &id(), &mpk);
        assert_eq!(out, RebuiltKey::Failed);
    }

    #[test]
    fn rebuild_never_panics_on_malformed_input() {
        let mpk = vec![0u8; 96];
        assert_eq!(
            rebuild_and_check_key(&[], &[], &[1], 1, 1, &id(), &mpk),
            RebuiltKey::Failed
        );
        assert_eq!(
            rebuild_and_check_key(&[0], &[vec![vec![0u8; 48]]], &[1], 1, 1, &id(), &mpk),
            RebuiltKey::Failed
        );
    }

    #[test]
    fn share_encoding_rejects_bad_points() {
        let mut rng = StdRng::seed_from_u64(4);
        let s = random_scalar(&mut rng);
        let good = encode_g1(&derive_decryption_key(&s, &id()).unwrap()).to_vec();
        assert_eq!(
            validate_share_encoding(&[good.clone(), good.clone()]),
            Ok(())
        );
        assert_eq!(validate_share_encoding(&[]), Err(0));
        assert_eq!(
            validate_share_encoding(&[good.clone(), vec![0u8; 47]]),
            Err(1)
        );
        // All zero bytes is not a valid compressed point. The compressed identity (0xc0 then zeros)
        // is a valid encoding of the identity and must be refused.
        let mut identity = vec![0u8; 48];
        identity[0] = 0xc0;
        assert_eq!(validate_share_encoding(&[good, identity]), Err(1));
    }

    struct AcceptAll;
    impl TimelockShareVerifier for AcceptAll {
        fn verify_shares(&self, _: &ShareCheckRequest<'_>) -> Result<(), ShareCheckError> {
            Ok(())
        }
    }

    #[test]
    fn no_verifier_means_fail_closed() {
        set_timelock_share_verifier(None);
        assert!(current_share_verifier().is_none());
        set_timelock_share_verifier(Some(Arc::new(AcceptAll)));
        assert!(current_share_verifier().is_some());
        set_timelock_share_verifier(None);
        assert!(current_share_verifier().is_none());
    }

    #[test]
    fn failure_codes_do_not_collide_with_chunky_dkg() {
        // chunky_dkg.rs uses 0x10201..0x10205 and 0x30201..0x30204.
        for code in [
            ExpectedFailure::InvalidShareEncoding as u64,
            ExpectedFailure::EpochNotCurrent as u64,
            ExpectedFailure::TimelockEpochMismatch as u64,
            ExpectedFailure::ShareCheckFailed as u64,
            ExpectedFailure::RegistryAbort as u64,
            ExpectedFailure::ShareVerifierUnavailable as u64,
            ExpectedFailure::MissingResourceConfiguration as u64,
        ] {
            assert!((0x10401..=0x10405).contains(&code) || (0x30401..=0x30402).contains(&code));
        }
    }

    // ---- Boneh-Franklin verifier on a request ----

    fn dealing(seed: u64) -> (Vec<aptos_dkg::ibe::MasterScalar>, Vec<Vec<u8>>) {
        let mut rng = StdRng::seed_from_u64(seed);
        let secrets: Vec<_> = (0..3).map(|_| random_scalar(&mut rng)).collect();
        let vks = secrets
            .iter()
            .map(|s| encode_g2(&master_public_key(s)).to_vec())
            .collect();
        (secrets, vks)
    }

    fn shares(secrets: &[aptos_dkg::ibe::MasterScalar], identity: &[u8]) -> Vec<Vec<u8>> {
        secrets
            .iter()
            .map(|s| encode_g1(&derive_decryption_key(s, identity).unwrap()).to_vec())
            .collect()
    }

    fn check(identity: &[u8], shares: &[Vec<u8>], vks: &[Vec<u8>]) -> Result<(), ShareCheckError> {
        BonehFranklinShareVerifier.verify_shares(&ShareCheckRequest {
            epoch: 3,
            timelock_id: 11,
            author: AccountAddress::ONE,
            identity,
            mpk: &[],
            verification_keys: vks,
            shares,
        })
    }

    #[test]
    fn boneh_franklin_verifier_accepts_valid_and_refuses_each_negative() {
        let (secrets, vks) = dealing(7);
        let good = shares(&secrets, &id());
        assert_eq!(check(&id(), &good, &vks), Ok(()));

        // wrong share
        let mut wrong = good.clone();
        wrong[2] = good[0].clone();
        assert!(check(&id(), &wrong, &vks).is_err());
        // another identity, another epoch
        for other in [
            compute_identity(4, 3, 12, 1_700_000_000_000_000),
            compute_identity(4, 4, 11, 1_700_000_000_000_000),
        ] {
            assert!(check(&id(), &shares(&secrets, &other), &vks).is_err());
        }
        // another secret
        let (foreign, _) = dealing(8);
        assert!(check(&id(), &shares(&foreign, &id()), &vks).is_err());
        // a share count that differs from the key count
        assert!(check(&id(), &good[..2], &vks).is_err());
        assert!(check(&id(), &[], &[]).is_err());
        // an invalid point
        let mut identity_point = vec![0u8; 48];
        identity_point[0] = 0xc0;
        let mut bad_point = good.clone();
        bad_point[1] = identity_point;
        assert!(check(&id(), &bad_point, &vks).is_err());
    }
}
