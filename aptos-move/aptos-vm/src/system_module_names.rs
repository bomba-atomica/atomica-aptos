// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Names of modules, functions, and types used by Aptos System.

use aptos_types::account_config;
use move_core_types::{ident_str, identifier::IdentStr, language_storage::ModuleId};
use once_cell::sync::Lazy;

pub static ACCOUNT_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("account").to_owned(),
    )
});

pub static ACCOUNT_ABSTRACTION_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("account_abstraction").to_owned(),
    )
});

pub const CREATE_ACCOUNT_IF_DOES_NOT_EXIST: &IdentStr =
    ident_str!("create_account_if_does_not_exist");

pub const AUTHENTICATE: &IdentStr = ident_str!("authenticate");

// Data to resolve basic account and transaction flow functions and structs
/// The ModuleId for the aptos block module
pub static BLOCK_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("block").to_owned(),
    )
});

pub const BLOCK_PROLOGUE: &IdentStr = ident_str!("block_prologue");
pub const BLOCK_PROLOGUE_EXT: &IdentStr = ident_str!("block_prologue_ext");
pub const BLOCK_PROLOGUE_EXT_V2: &IdentStr = ident_str!("block_prologue_ext_v2");
pub const BLOCK_PROLOGUE_EXT_V3: &IdentStr = ident_str!("block_prologue_ext_v3");
pub const BLOCK_EPILOGUE: &IdentStr = ident_str!("block_epilogue");

pub static RECONFIGURATION_WITH_DKG_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("reconfiguration_with_dkg").to_owned(),
    )
});

pub const FINISH_WITH_DKG_RESULT: &IdentStr = ident_str!("finish_with_dkg_result");
pub const FINISH_WITH_CHUNKY_DKG_RESULT: &IdentStr = ident_str!("finish_with_chunky_dkg_result");
pub const FINISH_WITH_TIMELOCK_DKG_RESULT: &IdentStr =
    ident_str!("finish_with_timelock_dkg_result");
pub const GET_EPOCH_INTERVAL_SECS: &IdentStr = ident_str!("get_epoch_interval_secs");

pub static IBE_CONFIG_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("ibe_config").to_owned(),
    )
});

pub const SUBMIT_DK_SHARES_FUNCTION: &IdentStr = ident_str!("submit_dk_shares");
pub const FINALIZE_TIMELOCK_REVEAL_FUNCTION: &IdentStr = ident_str!("finalize_timelock_reveal");
pub const FAIL_TIMELOCK_KEY_CHECK_FUNCTION: &IdentStr = ident_str!("fail_timelock_key_check");
pub const IS_RECONSTRUCTABLE_FUNCTION: &IdentStr = ident_str!("is_reconstructable");
pub const GET_RECONSTRUCTION_INPUTS_FUNCTION: &IdentStr = ident_str!("get_reconstruction_inputs");
pub const GET_IDENTITY_FUNCTION: &IdentStr = ident_str!("get_identity");
pub const GET_TIMELOCK_EPOCH_FUNCTION: &IdentStr = ident_str!("get_timelock_epoch");
pub const GET_SHARE_VERIFICATION_KEYS_FUNCTION: &IdentStr =
    ident_str!("get_share_verification_keys");
pub const REPORT_REJECTED_SHARE_FUNCTION: &IdentStr = ident_str!("report_rejected_share");
pub const GET_MPK_FUNCTION: &IdentStr = ident_str!("get_mpk");

pub static JWKS_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("jwks").to_owned(),
    )
});

pub const UPSERT_INTO_OBSERVED_JWKS: &IdentStr = ident_str!("upsert_into_observed_jwks");

pub static MULTISIG_ACCOUNT_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("multisig_account").to_owned(),
    )
});
pub const VALIDATE_MULTISIG_TRANSACTION: &IdentStr = ident_str!("validate_multisig_transaction");
pub const GET_NEXT_TRANSACTION_PAYLOAD: &IdentStr = ident_str!("get_next_transaction_payload");
pub const SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP: &IdentStr =
    ident_str!("successful_transaction_execution_cleanup");
pub const FAILED_TRANSACTION_EXECUTION_CLEANUP: &IdentStr =
    ident_str!("failed_transaction_execution_cleanup");

pub static TRANSACTION_FEE_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("transaction_fee").to_owned(),
    )
});

pub const EMIT_FEE_STATEMENT: &IdentStr = ident_str!("emit_fee_statement");

pub static TRANSACTION_VALIDATION_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        account_config::CORE_CODE_ADDRESS,
        ident_str!("transaction_validation").to_owned(),
    )
});

pub const VERSIONED_PROLOGUE_NAME: &IdentStr = ident_str!("versioned_prologue");
pub const VERSIONED_EPILOGUE_NAME: &IdentStr = ident_str!("versioned_epilogue");
