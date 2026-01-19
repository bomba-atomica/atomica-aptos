// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for the Timelock Registry functionality.
//!
//! This module contains end-to-end (E2E) tests that verify the complete timelock
//! encryption workflow, from registration through DK share submission and reveal.
//!
//! ## Test Coverage
//!
//! | Test Module | Description | Status |
//! |-------------|-------------|--------|
//! | [`register_and_query`] | Timelock registration and view functions | ✅ PASS |
//! | [`deadline_reveal`] | DK share submission and threshold reconstruction | ✅ PASS |
//!
//! ## Complete Workflow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           TIMELOCK WORKFLOW                                  │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │  1. REGISTRATION                                                             │
//! │     ┌─────────────────┐                                                     │
//! │     │ User calls      │                                                     │
//! │     │ register_timelock│                                                    │
//! │     └────────┬────────┘                                                     │
//! │              ▼                                                              │
//!     ┌─────────────────────────────┐                                          │
//!     │ On-chain:                   │                                          │
//!     │ - TimelockInfo stored       │                                          │
//!     │ - Identity computed         │                                          │
//!     │ - Event emitted             │                                          │
//!     └─────────────────────────────┘                                          │
//!                                                                              │
//!  2. ENCRYPTION (Anyone)                                                       │
//!     ┌─────────────────────────────┐                                          │
//!     │ - Get MPK from on-chain     │                                          │
//!     │ - Compute identity          │                                          │
//!     │ - Encrypt message with IBE  │                                          │
//!     │ - Store ciphertext          │                                          │
//!     └─────────────────────────────┘                                          │
//!                                                                              │
//!  3. DKG (Validators)                                                          │
//!     ┌─────────────────────────────┐                                          │
//!     │ - Run DKG for epoch         │                                          │
//!     │ - Each validator gets share │                                          │
//!     │ - MPK published on-chain    │                                          │
//!     └─────────────────────────────┘                                          │
//!                                                                              │
//!  4. REVEAL (After Deadline)                                                   │
//!     ┌─────────────────────────────┐                                          │
//!     │ Each validator:             │                                          │
//!     │ - Decrypt their share       │                                          │
//!     │ - Compute DK contribution   │                                          │
//!     │ - Submit TimelockShare TX   │                                          │
//!     └────────┬────────────────────┘                                          │
//!              │                                                                │
//!              ▼                                                                │
//!     ┌─────────────────────────────┐                                          │
//!     │ On-chain:                   │                                          │
//!     │ - Aggregate G1 contributions│                                          │
//!     │ - Reconstruct DK via        │                                          │
//!       Lagrange interpolation      │                                          │
//!     │ - Publish DK                │                                          │
//!     └─────────────────────────────┘                                          │
//!                                                                              │
//!  5. DECRYPTION (Anyone with ciphertext)                                       │
//!     ┌─────────────────────────────┐                                          │
//!     │ - Get DK from on-chain      │                                          │
//!     │ - Decrypt ciphertext        │                                          │
//!     │ - Recover plaintext         │                                          │
//!     └─────────────────────────────┘                                          │
//!                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Documentation References
//!
//! **Architecture:**
//! - [ADR-001: Dual-Output DKG](atomica/docs/adr-001-dual-output-dkg.md)
//! - [Implementation Plan](atomica/docs/implementation-plan-unified-dkg-ibe.md)
//! - [Timelock Specification](atomica/docs/product-spec/atomica-timelock-spec.md)
//!
//! **Source Code:**
//! - [Timelock Move Module](aptos-move/framework/aptos-framework/sources/ibe_config.move)
//! - [DKG Integration](types/src/dkg/real_dkg/mod.rs)
//! - [Scalar ElGamal PVSS](crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs)
//! - [IBE Primitives](crates/aptos-dkg/src/ibe/mod.rs)
//!
//! **Related Tests:**
//! - [Randomness E2E](randomness/e2e_correctness.rs) - DKG correctness test
//! - [MPK On-Chain](timelock/mpk_on_chain.rs) - MPK storage test
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all timelock tests
//! cargo test -p smoke-test --lib timelock
//!
//! # Run specific test
//! cargo test -p smoke-test --lib timelock::register_and_query
//! cargo test -p smoke-test --lib timelock::deadline_reveal
//!
//! # Run with output
//! cargo test -p smoke-test --lib timelock -- --nocapture
//! ```

pub mod deadline_reveal;
pub mod register_and_query;
