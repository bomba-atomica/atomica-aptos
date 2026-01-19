// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for the Timelock Registry functionality.
//!
//! These tests verify:
//! 1. Timelock registration works correctly
//! 2. View functions return correct information
//! 3. Identity computation is deterministic
//! 4. Registry state is persisted on-chain
//! 5. DK Share submission via validator transactions

pub mod deadline_reveal;
pub mod register_and_query;
