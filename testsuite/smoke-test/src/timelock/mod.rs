// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Timelock E2E smoke tests
//!
//! This module contains utilities and tests for the timelock encryption feature,
//! which uses distributed key generation (DKG) to enable time-based encryption
//! for sealed bid auctions.

pub mod basic_flow;
pub mod dkg_startup;
pub mod ibe_e2e;
pub mod test_dkg;
pub mod test_helpers;
pub mod test_ibe;
pub mod test_timelock;

use anyhow::{anyhow, Result};
use aptos_api_types::ViewFunction;
use aptos_logger::info;
use aptos_rest_client::Client;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::{str::FromStr, time::Duration};
use tokio::time::{sleep, Instant};

/// Represents the on-chain timelock state.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TimelockState {
    pub current_interval: u64,
    pub last_rotation_time: u64,
}

/// Get current interval number from on-chain state.
///
/// Calls the timelock::get_current_interval() view function.
pub async fn get_current_interval(client: &Client) -> Result<u64> {
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::timelock").map_err(|e| anyhow!("{}", e))?,
        function: Identifier::from_str("get_current_interval").map_err(|e| anyhow!("{}", e))?,
        ty_args: vec![],
        args: vec![],
    };

    let result: Vec<u64> = client
        .view_bcs(&view_function, None)
        .await
        .map_err(|e| anyhow!("Failed to call get_current_interval: {}", e))?
        .into_inner();

    result
        .first()
        .copied()
        .ok_or_else(|| anyhow!("get_current_interval returned empty result"))
}

/// Check if timelock is initialized on-chain.
///
/// Queries the get_current_interval view function - if it returns successfully,
/// the timelock is initialized. If it fails, it's not initialized.
pub async fn is_timelock_initialized(client: &Client) -> Result<bool> {
    match get_current_interval(client).await {
        Ok(_) => Ok(true),
        Err(e) => {
            info!("[Timelock Test] Timelock not initialized: {}", e);
            Ok(false)
        },
    }
}

/// Wait for timelock interval to rotate to target interval.
///
/// Polls the on-chain TimelockState until current_interval >= target_interval
/// or timeout is reached.
///
/// # Arguments
/// - client: REST client to query blockchain state
/// - target_interval: Wait until current_interval reaches this value
/// - timeout_secs: Maximum time to wait in seconds
///
/// # Returns
/// TimelockState when target interval is reached
///
/// # Errors
/// Returns error if timeout is reached before rotation
pub async fn wait_for_interval_rotation(
    client: &Client,
    target_interval: u64,
    timeout_secs: u64,
) -> Result<TimelockState> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        let current = get_current_interval(client).await?;

        if current >= target_interval {
            info!(
                "[Timelock Test] Reached interval {} (target: {})",
                current, target_interval
            );
            return Ok(TimelockState {
                current_interval: current,
                last_rotation_time: 0, // Not tracked via view function
            });
        }

        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timeout waiting for interval rotation: current={}, target={}",
                current,
                target_interval
            ));
        }

        info!(
            "[Timelock Test] Waiting for rotation: current={}, target={}, elapsed={:.1}s",
            current,
            target_interval,
            start.elapsed().as_secs_f64()
        );

        sleep(Duration::from_secs(1)).await;
    }
}

/// Verify public key is published for interval.
///
/// Queries the timelock module to check if a public key (MPK) has been
/// published for the specified interval. This is used by bidders to
/// encrypt their bids.
///
/// # Arguments
/// - client: REST client to query blockchain state
/// - interval: Interval number to check
///
/// # Returns
/// Public key bytes if published
///
/// # Errors
/// Returns error if public key is not published
pub async fn verify_public_key_published(client: &Client, interval: u64) -> Result<Vec<u8>> {
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::timelock").map_err(|e| anyhow!("{}", e))?,
        function: Identifier::from_str("get_public_key").map_err(|e| anyhow!("{}", e))?,
        ty_args: vec![],
        args: vec![bcs::to_bytes(&interval)?],
    };

    // Result is Option<vector<u8>> which BCS-deserializes as Vec<Option<Vec<u8>>>
    let result: Vec<Option<Vec<u8>>> = client
        .view_bcs(&view_function, None)
        .await
        .map_err(|e| anyhow!("Failed to call get_public_key: {}", e))?
        .into_inner();

    result
        .first()
        .cloned()
        .flatten()
        .ok_or_else(|| anyhow!("Public key not published for interval {}", interval))
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
    interval: u64,
    _expected_threshold: u64,
) -> Result<Vec<u8>> {
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::timelock").map_err(|e| anyhow!("{}", e))?,
        function: Identifier::from_str("get_secret").map_err(|e| anyhow!("{}", e))?,
        ty_args: vec![],
        args: vec![bcs::to_bytes(&interval)?],
    };

    // Result is Option<vector<u8>>
    let result: Vec<Option<Vec<u8>>> = client
        .view_bcs(&view_function, None)
        .await
        .map_err(|e| anyhow!("Failed to call get_secret: {}", e))?
        .into_inner();

    result
        .first()
        .cloned()
        .flatten()
        .ok_or_else(|| anyhow!("Secret not aggregated for interval {}", interval))
}
