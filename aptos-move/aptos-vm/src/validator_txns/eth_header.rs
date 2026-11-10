// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Handler of the `ObservedEthHeader` validator transaction (fork only).
//!
//! Design: `atomica/docs/decisions/eth-header-attestation.md`. Modeled on the JWK handler. The handler
//! reads the current epoch, the validator set and `EthereumHeaderConfig` from chain state, runs
//! [`check_certified_eth_header`] (epoch, Ethereum chain id, 2/3+ voting power, multi-signature), then
//! calls `ethereum_headers::store_certified_header`. Every rejection, including a Move abort for a block
//! that already has a certified header, discards the transaction and leaves state unchanged.

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{ETHEREUM_HEADERS_MODULE, STORE_CERTIFIED_HEADER},
    AptosVM,
};
use aptos_logger::{debug, warn};
use aptos_types::{
    eth_header::{
        check_certified_eth_header, EthHeaderRejection, EthereumHeaderConfig,
        QuorumCertifiedEthHeader,
    },
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
    EpochNotCurrent = 0x10601,
    ChainIdMismatch = 0x10602,
    NotEnoughVotingPower = 0x10603,
    MultiSigVerificationFailed = 0x10604,
    /// `store_certified_header` aborted: the block already has a header, or a hash has the wrong length.
    StoreAborted = 0x10605,

    // Move equivalent: `errors::invalid_state(*)`
    MissingResourceConfiguration = 0x30601,
    MissingResourceValidatorSet = 0x30602,
    /// Header attestation is off: no `EthereumHeaderConfig` on chain.
    NotEnabled = 0x30603,
}

impl From<EthHeaderRejection> for ExpectedFailure {
    fn from(r: EthHeaderRejection) -> Self {
        match r {
            EthHeaderRejection::EpochNotCurrent => Self::EpochNotCurrent,
            EthHeaderRejection::ChainIdMismatch => Self::ChainIdMismatch,
            EthHeaderRejection::NotEnoughVotingPower => Self::NotEnoughVotingPower,
            EthHeaderRejection::MultiSigVerificationFailed => Self::MultiSigVerificationFailed,
        }
    }
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

impl AptosVM {
    pub(crate) fn process_eth_header(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        certified: QuorumCertifiedEthHeader,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        debug!("Processing eth header transaction");
        match self.process_eth_header_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            certified,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(ExecutionFailure::Expected(failure)) => {
                // Like the JWK handler: an expected failure looks like a Move abort and is discarded.
                debug!("Eth header transaction rejected: {:?}", failure);
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
                warn!("Eth header transaction unexpected failure: {:?}", vm_status);
                Err(vm_status)
            },
        }
    }

    fn process_eth_header_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        certified: QuorumCertifiedEthHeader,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let config = EthereumHeaderConfig::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(ExpectedFailure::NotEnabled))?;
        let epoch = ConfigurationResource::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(ExecutionFailure::Expected(
                ExpectedFailure::MissingResourceConfiguration,
            ))?
            .epoch();
        let validator_set = ValidatorSet::fetch_config(resolver).ok().flatten().ok_or(
            ExecutionFailure::Expected(ExpectedFailure::MissingResourceValidatorSet),
        )?;
        let verifier = ValidatorVerifier::from(&validator_set);
        check_certified_eth_header(&certified, epoch, config.eth_chain_id, &verifier)
            .map_err(|r| ExecutionFailure::Expected(r.into()))?;

        let observation = certified.observation;
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            MoveValue::U64(observation.eth_chain_id),
            MoveValue::U64(observation.block_number),
            MoveValue::vector_u8(observation.block_hash.to_vec()),
            MoveValue::vector_u8(observation.state_root.to_vec()),
        ];
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);
        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &ETHEREUM_HEADERS_MODULE,
                STORE_CERTIFIED_HEADER,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                if let VMStatus::MoveAbort { code, .. } = e.clone().into_vm_status() {
                    debug!(
                        "ethereum_headers::store_certified_header aborted with code {}",
                        code
                    );
                    return ExecutionFailure::Expected(ExpectedFailure::StoreAborted);
                }
                ExecutionFailure::Unexpected(
                    expect_only_successful_execution(
                        e,
                        STORE_CERTIFIED_HEADER.as_str(),
                        log_context,
                    )
                    .expect_err("expect_only_successful_execution always returns Err"),
                )
            })?;

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
