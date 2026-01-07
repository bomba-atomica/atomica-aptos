//! DKG Transcript Availability Tests
//!
//! This module verifies that the Distributed Key Generation (DKG) protocol
//! completes successfully and produces a transcript on-chain.
//!
//! # Conceptual Flow (Happy Case)
//! 1. **Network Startup**: A swarm of validators starts.
//! 2. **DKG Trigger**: The validators automatically detect the need for a DKG session (usually epoch-based or initial setup).
//! 3. **Consensus**: Validators exchange messages to generate a distributed key.
//! 4. **Publication**: The resulting DKG transcript is published to the `0x1::dkg::DKGState` resource.
//!
//! # Failure Modes & Debugging
//!
//! ## Symptom: Test times out (60s)
//! - **Cause 1: Validators stuck**: The validators might not be communicating. Check node logs for connectivity issues.
//! - **Cause 2: DKG aborts**: If DKG fails to reach consensus, it might abort. Check logs for "DKG abort" or "insufficient participation".
//! - **Cause 3: Event non-emission**: If the `StartKeyGenEvent` wasn't emitted or processed, DKG won't start.
//!
//! ## Debugging Steps
//! 1. Run with `RUST_LOG=debug` to see detailed validator logs.
//! 2. Inspect the `0x1::dkg::DKGState` resource manually using the REST API to see if `last_completed` is present.
//! 3. Check if the epoch change triggered ensuring `0x1::reconfiguration::current_epoch` increased if expected.

use super::test_helpers::{create_timelock_swarm, TimelockTestConfig};
use aptos_logger::info;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Verify that DKG transcript is available in DKG state conversation
///
/// This checks that the standard DKG process completes and stores
/// the transcript in the `dkg::DKGState` resource.
/// 
/// This is the "upstream" check. If this fails, the MPK publication will definitely fail.
#[tokio::test]
async fn test_dkg_transcript_available() {
    let config = TimelockTestConfig::default();
    let (_swarm, client, _chain_id) = create_timelock_swarm(config).await;

    info!("Waiting for DKG transcript to be available...");
    
    let mut transcript_available = false;
    let mut last_error = String::new();
    
    for attempt in 0..60 {
        match super::verify_public_key_published(&client, 1).await {
            Ok(bytes) => {
                info!("✅ DKG transcript available after {}s: {} bytes", attempt, bytes.len());
                transcript_available = true;
                break;
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt % 10 == 0 {
                    info!("Attempt {}/60: DKG transcript not yet available - {}", attempt, e);
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
    
    assert!(
        transcript_available, 
        "DKG transcript should be available within 60s. Last error: {}", 
        last_error
    );
}
