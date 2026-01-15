// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Timelock E2E smoke tests
//!
//! This module contains utilities and tests for the timelock encryption feature,
//! which uses distributed key generation (DKG) to enable time-based encryption
//! for sealed bid auctions.

pub mod dkg_startup;

pub mod test_dkg_sequencing;
pub mod test_events;
pub mod test_helpers;
pub mod test_ibe;
pub mod test_secret_revelation;

use anyhow::{anyhow, Result};
use aptos_api_types::ViewFunction;
use aptos_logger::debug;
use aptos_rest_client::Client;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::str::FromStr;

/// Verify public key is published for interval.
///
/// Gets the DKG transcript and extracts the public key (MPK) for IBE encryption.
/// The public key is stored in the DKG state, not in a separate timelock module store.
///
/// # Arguments
/// - client: REST client to query blockchain state
/// - interval: Interval number to check (currently unused, uses latest DKG transcript)
///
/// # Returns
/// Public key bytes (BCS-serialized DKG transcript)
///
/// # Errors
/// Returns error if DKG transcript is not available
pub async fn verify_public_key_published(client: &Client, _interval: u64) -> Result<Vec<u8>> {
    use crate::utils::get_on_chain_resource;
    use aptos_types::dkg::DKGState;

    debug!("Querying DKG state for public key...");

    // Get DKG state which contains the transcript
    let dkg_state = get_on_chain_resource::<DKGState>(&client).await;

    debug!("DKG state retrieved, checking for completed transcript...");

    // Check if DKG has completed
    let last_completed = dkg_state
        .last_completed
        .ok_or_else(|| anyhow!("DKG has not completed yet - no completed transcript found"))?;

    debug!(
        "DKG transcript found: dealer_epoch={}, target_epoch={}, transcript_len={}",
        last_completed.metadata.dealer_epoch,
        last_completed.target_epoch(),
        last_completed.transcript.len()
    );

    // Return the raw transcript bytes
    // Tests will deserialize this to extract the public key
    Ok(last_completed.transcript.clone())
}

/// Verify secret is aggregated for interval.
///
/// Queries the timelock module to check if the aggregated decryption key
/// has been revealed for the specified interval.
///
/// # Arguments
/// - client: REST client to query blockchain state
/// - interval: Interval number to check
/// - _expected_threshold: (unused) Expected number of shares that should be aggregated
///
/// # Returns
/// Aggregated secret key bytes if revealed
///
/// # Errors
/// Returns error if secret is not revealed
pub async fn verify_secret_aggregated(
    client: &Client,
    timelock_id: u64,
    _expected_threshold: u64,
) -> Result<Vec<u8>> {
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::timelock").map_err(|e| anyhow!("{}", e))?,
        function: Identifier::from_str("get_decryption_key").map_err(|e| anyhow!("{}", e))?,
        ty_args: vec![],
        args: vec![bcs::to_bytes(&timelock_id)?],
    };

    debug!("Querying timelock decryption key for ID {}...", timelock_id);

    // Result is Option<vector<u8>>
    let result: Vec<Option<Vec<u8>>> = client
        .view_bcs(&view_function, None)
        .await
        .map_err(|e| anyhow!("Failed to call get_decryption_key: {}", e))?
        .into_inner();

    debug!(
        "get_decryption_key response for ID {}: {:?}",
        timelock_id,
        result.first()
    );

    result
        .first()
        .cloned()
        .flatten()
        .ok_or_else(|| anyhow!("Secret not aggregated for timelock {}", timelock_id))
}

/// Verify master public key is published in threshold_dsa module.
///
/// Queries `0x1::threshold_dsa::get_master_public_key`.
pub async fn verify_master_public_key_on_chain(client: &Client, interval: u64) -> Result<Vec<u8>> {
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::threshold_dsa").map_err(|e| anyhow!("{}", e))?,
        function: Identifier::from_str("get_master_public_key").map_err(|e| anyhow!("{}", e))?,
        ty_args: vec![],
        args: vec![bcs::to_bytes(&interval)?],
    };

    debug!("Querying Master Public Key for interval {}...", interval);

    let result: Vec<Option<Vec<u8>>> = client
        .view_bcs(&view_function, None)
        .await
        .map_err(|e| anyhow!("Failed to call get_master_public_key: {}", e))?
        .into_inner();

    debug!(
        "get_master_public_key response for interval {}: {:?}",
        interval,
        result.first()
    );

    result
        .first()
        .cloned()
        .flatten()
        .ok_or_else(|| anyhow!("Master Public Key not found for interval {}", interval))
}
