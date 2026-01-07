//! MPK (Master Public Key) Publication Tests
//!
//! This module verifies that the Master Public Key (MPK) is derived from the DKG transcript
//! and published to the `0x1::threshold_dsa` module, making it available for applications.
//!
//! # Conceptual Flow (Happy Case)
//! 1. **DKG Completion**: The DKG protocol completes (verified by `test_dkg_transcript.rs`).
//! 2. **Transcript Processing**: The `threshold_dsa` module reads the `DKGState`.
//! 3. **Derivation**: The module derives the MPK from the transcript.
//! 4. **Publication**: The MPK is written to `0x1::threshold_dsa::MasterPubKeys`.
//!
//! # Failure Modes & Debugging
//!
//! ## Symptom: Test times out (60s)
//! - **Cause 1: DKG failed**: If DKG didn't produce a transcript, this will fail. Always check `test_dkg_transcript.rs` first.
//! - **Cause 2: Transcript invalid**: The transcript exists but is invalid/corrupt, causing derivation to fail.
//! - **Cause 3: Logic error in Move**: The `threshold_dsa` module might have a bug preventing it from reading the DKG state.
//!
//! ## Debugging Steps
//! 1. Verify `test_dkg_transcript.rs` passes.
//! 2. Check if the `MasterPubKeys` resource exists on-chain: `aptos move view --function-id 0x1::threshold_dsa::get_master_public_key ...`
//! 3. Look for Move VM execution errors in the validator logs which might indicate a failure during the "system transaction" that processes the DKG result.

use super::test_helpers::{create_timelock_swarm, TimelockTestConfig};
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify that MPK is published on-chain after swarm starts
///
/// This is the "downstream" check. It confirms that the application layer (`threshold_dsa`)
/// has successfully consumed the protocol layer's output (`dkg`).
#[tokio::test]
async fn test_mpk_published_on_chain() {
    let config = TimelockTestConfig::default();
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Waiting for Master Public Key (ID=1) to be published...");

    let mut mpk_published = false;
    let mut last_error = String::new();

    for attempt in 0..60 {
        match super::verify_master_public_key_on_chain(&client, 1).await {
            Ok(mpk_bytes) => {
                info!(
                    "✅ Master Public Key published after {}s: {} bytes",
                    attempt,
                    mpk_bytes.len()
                );
                mpk_published = true;
                break;
            },
            Err(e) => {
                last_error = e.to_string();
                if attempt % 10 == 0 {
                    info!("Attempt {}/60: MPK not yet available - {}", attempt, e);
                }
            },
        }
        sleep(Duration::from_secs(1)).await;
    }

    assert!(
        mpk_published,
        "MPK should be published within 60s. Last error: {}",
        last_error
    );
}
