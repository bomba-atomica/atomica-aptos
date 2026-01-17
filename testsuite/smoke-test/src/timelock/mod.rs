// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for the IBE/Timelock functionality.
//!
//! These tests validate the end-to-end flow of:
//! 1. MPK storage on-chain after DKG
//! 2. IBE encryption using the on-chain MPK
//! 3. Decryption using keys derived from validator shares

mod mpk_encrypt_decrypt;
mod mpk_on_chain;
