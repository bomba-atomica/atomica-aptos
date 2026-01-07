// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Debug tests for ethereum_derivable_account using FakeExecutor
//!
//! This test file allows us to submit transactions with custom authenticators
//! directly to the MoveVM without running a full validator node, enabling
//! detailed introspection of the signature verification process.

use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    SigningKey, Uniform,
};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        authenticator::{AccountAuthenticator, TransactionAuthenticator},
        SignedTransaction, TransactionStatus,
    },
};
use move_core_types::vm_status::StatusCode;

/// Helper to create a simple APT transfer transaction using ethereum_derivable_account
/// This mimics what our TypeScript code is doing
#[test]
fn test_ethereum_derivable_simple_transfer() {
    // 1. Create executor from genesis
    let mut executor = FakeExecutor::from_head_genesis();

    // 2. Create accounts
    let sender = executor.create_accounts(1, 100_000_000, 0).remove(0);
    let receiver = AccountAddress::from_hex_literal("0x2").unwrap();

    // 3. Build a simple APT transfer transaction
    let raw_tx = sender
        .transaction()
        .payload(aptos_stdlib::aptos_account_transfer(receiver, 100))
        .sequence_number(0)
        .gas_unit_price(100)
        .max_gas_amount(10000)
        .raw();

    println!("=== Transaction Details ===");
    println!("Sender: {}", sender.address());
    println!("Receiver: {}", receiver);
    println!("Function: 0x1::aptos_account::transfer");
    println!("Sequence Number: {}", raw_tx.sequence_number());
    println!("Chain ID: {}", raw_tx.chain_id());

    // 4. Sign with standard Ed25519 (this should work)
    let signed_tx = sender
        .transaction()
        .payload(aptos_stdlib::aptos_account_transfer(receiver, 100))
        .sequence_number(0)
        .gas_unit_price(100)
        .max_gas_amount(10000)
        .sign();

    // 5. Execute and verify it works
    let outputs = executor.execute_block(vec![signed_tx]).unwrap();
    let output = &outputs[0];

    println!("\n=== Standard Signature Test ===");
    println!("Status: {:?}", output.status());

    assert!(matches!(output.status(), TransactionStatus::Keep(_)));

    // Now let's try with a custom ethereum_derivable_account authenticator
    // TODO: Implement ethereum authenticator construction
}

/// Test to understand how entry_function_name is extracted
#[test]
fn test_entry_function_name_extraction() {
    let mut executor = FakeExecutor::from_head_genesis();
    let sender = executor.create_accounts(1, 100_000_000, 0).remove(0);
    let receiver = AccountAddress::from_hex_literal("0x2").unwrap();

    // Create different entry function calls and see how they're represented
    let test_cases = vec![
        (
            "0x1::aptos_account::transfer",
            aptos_stdlib::aptos_account_transfer(receiver, 100),
        ),
        (
            "0x1::aptos_coin::transfer",
            aptos_stdlib::aptos_coin_transfer(receiver, 100),
        ),
    ];

    for (expected_name, payload) in test_cases {
        println!("\n=== Testing: {} ===", expected_name);

        let signed_tx = sender
            .transaction()
            .payload(payload)
            .sequence_number(0)
            .gas_unit_price(100)
            .max_gas_amount(10000)
            .sign();

        // Execute and check
        let outputs = executor.execute_block(vec![signed_tx]).unwrap();
        let output = &outputs[0];

        println!("Status: {:?}", output.status());
        println!("Gas used: {}", output.gas_used());
    }
}

/// Placeholder for ethereum authenticator test
/// This is where we'll construct the exact same authenticator as TypeScript
#[test]
#[ignore] // TODO: Implement ethereum authenticator construction
fn test_ethereum_authenticator_debug() {
    let mut executor = FakeExecutor::from_head_genesis();

    // TODO:
    // 1. Derive Aptos address from Ethereum address using ethereum_derivable_account scheme
    // 2. Fund that derived address
    // 3. Construct SIWE message exactly as TypeScript does
    // 4. Sign with secp256k1 (simulating MetaMask)
    // 5. Build BCS SIWEAbstractSignature
    // 6. Create TransactionAuthenticator::Abstract
    // 7. Submit transaction
    // 8. Debug why it fails

    println!("TODO: Implement ethereum authenticator test");
    println!("This will require:");
    println!("  - ethers-rs for secp256k1 signing");
    println!("  - Derivation scheme matching ethereum_derivable_account.move");
    println!("  - BCS serialization of SIWEAbstractPublicKey and SIWEAbstractSignature");
}

#[cfg(test)]
mod helpers {
    use super::*;

    /// Helper to derive Aptos address from Ethereum address
    /// Must match the scheme in ethereum_derivable_account.move
    pub fn derive_aptos_address_from_ethereum(eth_address: &str, domain: &str) -> AccountAddress {
        // TODO: Implement derivation scheme
        // This should match getDerivedAddress() in TypeScript
        unimplemented!("Need to implement ethereum -> aptos address derivation")
    }

    /// Helper to construct SIWE message
    /// Must match constructSIWEMessage() in TypeScript
    pub fn construct_siwe_message(
        domain: &str,
        eth_address: &str,
        entry_function: &str,
        chain_id: u8,
        nonce: &str,
        issued_at: &str,
        scheme: &str,
        network_name: &str,
    ) -> String {
        format!(
            "{} wants you to sign in with your Ethereum account:\n\
            {}\n\
            \n\
            Please confirm you explicitly initiated this request from {}. \
            You are approving to execute transaction {} on Aptos blockchain ({}).\n\
            \n\
            URI: {}://{}\n\
            Version: 1\n\
            Chain ID: {}\n\
            Nonce: {}\n\
            Issued At: {}",
            domain,
            eth_address,
            domain,
            entry_function,
            network_name,
            scheme,
            domain,
            chain_id,
            nonce,
            issued_at
        )
    }
}
