// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{IBE_CONFIG_MODULE, SUBMIT_DK_SHARE_FUNCTION},
    AptosVM,
};
use aptos_types::{transaction::TransactionStatus, validator_txn::TimelockShare};
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
    InvalidShareLength = 0x10002,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

const G1_LENGTH: usize = 48;

impl AptosVM {
    pub(crate) fn process_timelock_share(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        timelock_share: TimelockShare,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.process_timelock_share_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            timelock_share,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(ExecutionFailure::Expected(failure)) => Ok((
                VMStatus::MoveAbort(AbortLocation::Script, failure as u64),
                VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
            )),
            Err(ExecutionFailure::Unexpected(vm_status)) => Err(vm_status),
        }
    }

    fn process_timelock_share_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        timelock_share: TimelockShare,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        // Verify share length (must be 48 bytes for G1 compressed point)
        if timelock_share.share.len() != G1_LENGTH {
            return Err(ExecutionFailure::Expected(
                ExpectedFailure::InvalidShareLength,
            ));
        }

        // All checks passed, invoke VM to submit the share on chain
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);

        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            MoveValue::U64(timelock_share.deadline_id),
            MoveValue::Address(timelock_share.author),
            MoveValue::Vector(
                timelock_share
                    .share
                    .iter()
                    .map(|&b| MoveValue::U8(b))
                    .collect(),
            ),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &IBE_CONFIG_MODULE,
                SUBMIT_DK_SHARE_FUNCTION,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, SUBMIT_DK_SHARE_FUNCTION.as_str(), log_context)
            })
            .map_err(|r| ExecutionFailure::Unexpected(r.unwrap_err()))?;

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
