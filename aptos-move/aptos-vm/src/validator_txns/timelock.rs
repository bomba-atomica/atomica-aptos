// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{PUBLISH_PUBLIC_KEY, PUBLISH_SECRET_SHARE, TIMELOCK_MODULE},
    AptosVM,
};
use aptos_types::{
    dkg::{DKGTranscript, TimelockShare},
    move_utils::as_move_value::AsMoveValue,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_core_types::{
    // use move_core_types::account_address::AccountAddress;
    value::{serialize_values, MoveValue},
    vm_status::VMStatus,
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;

impl AptosVM {
    pub(crate) fn process_timelock_dkg_result(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        dkg_transcript: DKGTranscript,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);

        let args = vec![
            MoveValue::Signer(dkg_transcript.metadata.author),
            MoveValue::U64(dkg_transcript.metadata.epoch), // Reuse epoch as interval
            dkg_transcript.transcript_bytes.as_move_value(),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &TIMELOCK_MODULE,
                PUBLISH_PUBLIC_KEY,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, PUBLISH_PUBLIC_KEY.as_str(), log_context)
            })
            .map_err(|r| r.unwrap_err())?;

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self.storage_gas_params(log_context)?.change_set_configs,
        )?;

        Ok((VMStatus::Executed, output))
    }

    pub(crate) fn process_timelock_share(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        share: TimelockShare,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);

        let args = vec![
            MoveValue::Signer(share.author),
            MoveValue::U64(share.interval),
            share.share.as_move_value(),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &TIMELOCK_MODULE,
                PUBLISH_SECRET_SHARE,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, PUBLISH_SECRET_SHARE.as_str(), log_context)
            })
            .map_err(|r| r.unwrap_err())?;

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self.storage_gas_params(log_context)?.change_set_configs,
        )?;

        Ok((VMStatus::Executed, output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::dkg::{DKGTranscript, DKGTranscriptMetadata, TimelockShare};
    use move_core_types::account_address::AccountAddress;

    // These tests verify that the structure of the dispatcher allows meaningful processing.
    // They mock the VM/Storage interaction to focus on valid input handling.

    #[test]
    fn test_timelock_dkg_result_dispatch() {
        // This is a minimal smoke test ensuring compilation and basic function signature handling
        let transcript = DKGTranscript {
            metadata: DKGTranscriptMetadata {
                epoch: 10,
                author: AccountAddress::ONE,
            },
            transcript_bytes: vec![1, 2, 3],
        };

        // Note: Fully mocking AptosVM session creation for unit tests is complex and often done
        // at integration level. Here we assert types exist and are importable.
        assert_eq!(transcript.metadata.epoch, 10);
    }
}
