// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Handler of the `TimelockDKGResult` validator transaction (fork only).
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` section 4. The handler copies the checks
//! of `process_chunky_dkg_result_inner` (epoch is current, a timelock session is in progress, voting power
//! quorum, multi-signature over the aggregated subtranscript) with three differences:
//!
//! - The multi-signature is checked in the **timelock hash domain** (`TimelockSubtranscriptSigningMessage`).
//!   A certified mempool aggregate does not verify here, and the reverse.
//! - Nothing FPTX is derived. The handler rederives `mpk` from the subtranscript, compares it with the claimed
//!   value, checks that `V0` equals the G2 interpolation of threshold sets of the verification keys, and
//!   hands every derived value to `reconfiguration_with_dkg::finish_with_timelock_dkg_result`.
//! - The blob hash the dealers used is compared with the local timelock parameters, when this node has them.
//!
//! [`check_timelock_dkg_result`] is a pure function of its arguments so the checks can be tested without a chain.

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{
        BLOCK_MODULE, FINISH_WITH_TIMELOCK_DKG_RESULT, GET_EPOCH_INTERVAL_SECS,
        RECONFIGURATION_WITH_DKG_MODULE,
    },
    AptosVM,
};
use aptos_logger::{info, warn};
use aptos_types::{
    account_config::ChainIdResource,
    dkg::timelock_dkg::{
        check_timelock_dkg_result, CertifiedTimelockDKGOutput, ResultRejection, TimelockDKGState,
        TIMELOCK_PUBLIC_PARAMETERS,
    },
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    transaction::TransactionStatus,
    validator_verifier::ValidatorVerifier,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;

#[derive(Debug)]
enum ExpectedFailure {
    // Move equivalent: `errors::invalid_argument(*)`
    EpochNotCurrent = 0x10501,
    MultiSigVerificationFailed = 0x10502,
    NotEnoughVotingPower = 0x10503,
    TranscriptDeserializationFailed = 0x10504,
    KeyDerivationFailed = 0x10505,
    ClaimedMpkMismatch = 0x10506,
    ParametersHashMismatch = 0x10507,

    // Move equivalent: `errors::invalid_state(*)`
    MissingResourceTimelockDKGState = 0x30501,
    MissingResourceInprogressTimelockDKGSession = 0x30502,
    MissingResourceConfiguration = 0x30503,
    MissingResourceValidatorSet = 0x30504,
    MissingResourceChainId = 0x30505,
    /// The timelock DKG is switched off on chain.
    NotEnabled = 0x30506,
}

impl From<ResultRejection> for ExpectedFailure {
    fn from(r: ResultRejection) -> Self {
        match r {
            ResultRejection::EpochNotCurrent => Self::EpochNotCurrent,
            ResultRejection::NotEnoughVotingPower => Self::NotEnoughVotingPower,
            ResultRejection::TranscriptDeserializationFailed => {
                Self::TranscriptDeserializationFailed
            },
            ResultRejection::MultiSigVerificationFailed => Self::MultiSigVerificationFailed,
            ResultRejection::KeyDerivationFailed(_) => Self::KeyDerivationFailed,
            ResultRejection::ClaimedMpkMismatch => Self::ClaimedMpkMismatch,
            ResultRejection::ParametersHashMismatch => Self::ParametersHashMismatch,
        }
    }
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

impl AptosVM {
    pub(crate) fn process_timelock_dkg_result(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        output: CertifiedTimelockDKGOutput,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        info!("Process timelock DKG Result");
        match self.process_timelock_dkg_result_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            output,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(ExecutionFailure::Expected(failure)) => {
                warn!("Timelock DKG result rejected: {:?}", failure);
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
                warn!("Timelock DKG result: unexpected failure: {:?}", vm_status);
                Err(vm_status)
            },
        }
    }

    fn process_timelock_dkg_result_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        output: CertifiedTimelockDKGOutput,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let config_resource = ConfigurationResource::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceConfiguration,
            ))?;
        let validator_set = ValidatorSet::fetch_config(resolver).ok().flatten().ok_or(
            ExecutionFailure::Expected(ExpectedFailure::MissingResourceValidatorSet),
        )?;
        let state = TimelockDKGState::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceTimelockDKGState,
            ))?;
        if !state.enabled {
            return Err(ExecutionFailure::Expected(ExpectedFailure::NotEnabled));
        }
        let in_progress = state
            .in_progress
            .as_ref()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceInprogressTimelockDKGSession,
            ))?;
        let chain_id = ChainIdResource::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceChainId,
            ))?
            .chain_id();

        let verifier = ValidatorVerifier::from(&validator_set);
        let local_params_hash = TIMELOCK_PUBLIC_PARAMETERS.as_ref().map(|p| p.blob_hash);
        let checked = check_timelock_dkg_result(
            &output,
            config_resource.epoch(),
            &in_progress.metadata,
            chain_id,
            &verifier,
            local_params_hash,
        )
        .map_err(|r| ExecutionFailure::Expected(r.into()))?;

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);

        // The epoch length at the time of the result. `ibe_config` derives the epoch end from it.
        let traversal_storage = TraversalStorage::new();
        let interval_values = session
            .execute_function_bypass_visibility(
                &BLOCK_MODULE,
                GET_EPOCH_INTERVAL_SECS,
                vec![],
                serialize_values(&Vec::<MoveValue>::new()),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, GET_EPOCH_INTERVAL_SECS.as_str(), log_context)
            })
            .map_err(|r| ExecutionFailure::Unexpected(r.unwrap_err()))?;
        let epoch_interval_secs: u64 = interval_values
            .return_values
            .first()
            .and_then(|(bytes, _)| bcs::from_bytes(bytes).ok())
            .ok_or_else(|| {
                ExecutionFailure::Unexpected(VMStatus::error(
                    StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION,
                    Some("block::get_epoch_interval_secs returned no u64".to_string()),
                ))
            })?;

        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            output.certified_transcript.transcript_bytes.as_move_value(),
            checked.derived.mpk.as_move_value(),
            MoveValue::U64(checked.derived.reconstruct_threshold_weight),
            MoveValue::Vector(
                checked
                    .derived
                    .weights
                    .iter()
                    .map(|w| MoveValue::U64(*w))
                    .collect(),
            ),
            MoveValue::U64(epoch_interval_secs.saturating_mul(1_000_000)),
            output.public_parameters_hash.to_vec().as_move_value(),
            checked.subtranscript_hash.to_vec().as_move_value(),
            MoveValue::Vector(
                checked
                    .derived
                    .verification_keys
                    .iter()
                    .map(|k| MoveValue::vector_u8(k.clone()))
                    .collect(),
            ),
        ];

        session
            .execute_function_bypass_visibility(
                &RECONFIGURATION_WITH_DKG_MODULE,
                FINISH_WITH_TIMELOCK_DKG_RESULT,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(
                    e,
                    FINISH_WITH_TIMELOCK_DKG_RESULT.as_str(),
                    log_context,
                )
            })
            .map_err(|r| ExecutionFailure::Unexpected(r.unwrap_err()))?;

        let vm_output = get_system_transaction_output(
            session,
            module_storage,
            &self
                .storage_gas_params(log_context)
                .map_err(ExecutionFailure::Unexpected)?
                .change_set_configs,
        )
        .map_err(ExecutionFailure::Unexpected)?;

        Ok((VMStatus::Executed, vm_output))
    }
}
