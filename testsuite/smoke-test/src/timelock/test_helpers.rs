//! Common test helpers for timelock smoke tests

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, Swarm};
use aptos_logger::info;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use std::sync::Arc;

/// Configuration for timelock tests
pub struct TimelockTestConfig {
    pub num_validators: usize,
    pub num_fullnodes: usize,
    pub epoch_duration_secs: u64,
    pub timelock_interval_secs: Option<u64>,
}

impl Default for TimelockTestConfig {
    fn default() -> Self {
        Self {
            num_validators: 3,
            num_fullnodes: 0,
            epoch_duration_secs: 20,
            timelock_interval_secs: Some(5),
        }
    }
}

/// Creates a swarm configured for timelock testing with DKG enabled.
///
/// This ensures:
/// - Validator transactions are enabled
/// - Randomness config is enabled (required for DKG)
/// - Optional custom epoch duration
///
/// Default configuration uses 3 validators and 0 fullnodes for faster test execution.
pub async fn create_timelock_swarm(
    config: TimelockTestConfig,
) -> (
    Box<dyn Swarm>,
    aptos_rest_client::Client,
    aptos_types::chain_id::ChainId,
) {
    info!(
        "Building timelock swarm: {} validators, {} fullnodes, {}s epochs",
        config.num_validators, config.num_fullnodes, config.epoch_duration_secs
    );

    let epoch_duration_secs = config.epoch_duration_secs;
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(config.num_validators)
        .with_num_fullnodes(config.num_fullnodes)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Enable validator transactions (required for DKG and timelock)
            conf.consensus_config.enable_validator_txns();

            // Enable randomness config (required for DKG manager to start)
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let client = swarm.validators().next().unwrap().rest_client();
    let chain_id = swarm.chain_id();

    // Configure timelock interval if requested
    if let Some(interval_secs) = config.timelock_interval_secs {
        configure_timelock_interval(&swarm, &client, interval_secs)
            .await
            .expect("Failed to configure timelock interval");
    }

    (Box::new(swarm), client, chain_id)
}

/// Configures the timelock interval for testing.
///
/// This calls the `timelock_config::set_interval_for_testing` entry function
/// to override the default interval duration.
pub async fn configure_timelock_interval<S: Swarm>(
    swarm: &S,
    client: &aptos_rest_client::Client,
    interval_secs: u64,
) -> anyhow::Result<()> {
    info!("Configuring timelock interval to {} seconds", interval_secs);

    let root_account = swarm.chain_info().root_account();
    let interval_us: u64 = interval_secs * 1_000_000;

    info!(
        "Creating transaction to set interval to {} microseconds",
        interval_us
    );

    let payload = aptos_types::transaction::TransactionPayload::EntryFunction(
        aptos_types::transaction::EntryFunction::new(
            ModuleId::new(
                aptos_types::account_address::AccountAddress::ONE,
                Identifier::new("timelock_config").unwrap(),
            ),
            Identifier::new("set_interval_for_testing").unwrap(),
            vec![],
            vec![bcs::to_bytes(&interval_us)?],
        ),
    );

    let signed_txn = root_account.sign_with_transaction_builder(
        aptos_sdk::transaction_builder::TransactionFactory::new(swarm.chain_info().chain_id)
            .payload(payload)
            .max_gas_amount(2_000_000)
            .gas_unit_price(100),
    );

    info!("Submitting timelock config transaction...");
    let response = client.submit_and_wait(&signed_txn).await?;
    info!(
        "Timelock config transaction completed: success={}",
        response.inner().success()
    );

    if !response.inner().success() {
        anyhow::bail!(
            "Timelock config transaction failed: {:?}",
            response.inner().vm_status()
        );
    }

    // Give it a moment to be applied
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    info!(
        "Timelock interval configured successfully to {} seconds",
        interval_secs
    );

    Ok(())
}
