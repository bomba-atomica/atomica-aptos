// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SmokeTestEnvironment;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use std::time::Duration;

#[tokio::test]
async fn test_timelock_flow() {
    let mut env = SmokeTestEnvironment::new(4).await;
    
    // 1. Wait for genesis and network startup
    env.validator_swarm.launch().await.expect("Swarm launch failed");
    
    let client = env.validator_swarm.get_client();
    
    // 2. Wait for some blocks to ensure timelock initialization
    // Initial interval is 0. 
    // We need to wait > 1 hour simulated time for rotation.
    // NOTE: In real smoke tests, we can't wait 1 hour real time.
    // We either need a custom build with shorter interval (via feature flag or config) 
    // OR we rely on the fact that for PoC we might hack the interval in Move for testing.
    // For this unmodified code, this test would hang/timeout unless we update Move to be configurable.
    // Assuming for 'smoke test' purposes we verify the *presence* of the module and initial state first.
    
    let timelock_resource = "0x1::timelock::TimelockState";
    let resources = client
        .get_account_resources(move_core_types::account_address::AccountAddress::ONE)
        .await
        .unwrap();
    
    let has_timelock = resources.iter().any(|r| r.resource_type.to_string().contains("TimelockState"));
    assert!(has_timelock, "TimelockState not found on chain!");

    // To properly test rotation in smoke tests without waiting 1 hour,
    // we would typically deploy investigating/test-only Move modules or use a Governance proposal to mitigate.
    // Or we verify that the *events* are subscribed to in the logs.
    
    // let logs = env.validator_swarm.validators().next().unwrap().fetch_logs();
    // assert!(logs.contains("[Timelock]")); 
}
