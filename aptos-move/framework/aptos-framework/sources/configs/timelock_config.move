// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Configuration for timelock encryption intervals.
///
/// This module manages the interval duration for timelock key rotation. The interval
/// determines how frequently new timelock keys are generated via DKG, and when old
/// keys are revealed for decryption.
///
/// Default: 1 hour (production)
/// Test: Configurable via `set_interval_for_testing()` on non-mainnet chains
module aptos_framework::timelock_config {
    use std::error;
    use aptos_framework::chain_id;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;
    friend aptos_framework::timelock;

    /// Timelock interval configuration is not initialized
    const ETIMELOCK_CONFIG_NOT_FOUND: u64 = 1;
    /// Cannot override interval in production (mainnet)
    const EPRODUCTION_OVERRIDE_FORBIDDEN: u64 = 2;

    /// Global configuration for timelock intervals.
    struct TimelockConfig has key {
        /// Interval duration in microseconds.
        /// Default: 1 hour = 3600 * 1_000_000 microseconds
        /// Test: 5 seconds = 5 * 1_000_000 microseconds (for fast testing)
        interval_microseconds: u64,
    }

    /// Initialize with default 1-hour interval.
    /// Called during genesis to set up the timelock configuration.
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<TimelockConfig>(@aptos_framework)) {
            move_to(framework, TimelockConfig {
                interval_microseconds: 3600 * 1000000, // 1 hour default
            });
        }
    }

    /// Set interval for testing (devnet/testnet only).
    ///
    /// This function allows overriding the default interval on test networks
    /// to speed up testing (e.g., 5 seconds instead of 1 hour).
    ///
    /// # Security
    /// This function is blocked on mainnet (chain_id == 1) to prevent
    /// production misconfigurations.
    ///
    /// # Arguments
    /// - framework: Must be @aptos_framework signer
    /// - interval_us: New interval in microseconds
    public entry fun set_interval_for_testing(
        framework: &signer,
        interval_us: u64
    ) acquires TimelockConfig {
        system_addresses::assert_aptos_framework(framework);

        // Prevent production override - mainnet has chain_id == 1
        let current_chain_id = chain_id::get();
        assert!(
            current_chain_id != 1,
            error::permission_denied(EPRODUCTION_OVERRIDE_FORBIDDEN)
        );

        if (!exists<TimelockConfig>(@aptos_framework)) {
            move_to(framework, TimelockConfig {
                interval_microseconds: interval_us,
            });
        } else {
            let config = borrow_global_mut<TimelockConfig>(@aptos_framework);
            config.interval_microseconds = interval_us;
        }
    }

    #[view]
    /// Get the current interval duration in microseconds. Returns the configured interval, or the default (1 hour) if not initialized. Used by the timelock module to determine rotation timing.
    public fun get_interval_microseconds(): u64 acquires TimelockConfig {
        if (!exists<TimelockConfig>(@aptos_framework)) {
            return 3600 * 1000000 // Default 1 hour
        };
        borrow_global<TimelockConfig>(@aptos_framework).interval_microseconds
    }

    #[test_only]
    use aptos_framework::account::create_signer_for_test;

    #[test(framework = @aptos_framework)]
    fun test_initialize_and_get(framework: &signer) acquires TimelockConfig {
        chain_id::initialize_for_test(framework, 4); // testnet
        initialize(framework);
        assert!(get_interval_microseconds() == 3600 * 1000000, 0);
    }

    #[test(framework = @aptos_framework)]
    fun test_set_for_testing(framework: &signer) acquires TimelockConfig {
        chain_id::initialize_for_test(framework, 4); // testnet
        initialize(framework);

        // Set to 5 seconds for testing
        set_interval_for_testing(framework, 5 * 1000000);
        assert!(get_interval_microseconds() == 5 * 1000000, 0);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x50002, location = Self)] // EPRODUCTION_OVERRIDE_FORBIDDEN
    fun test_cannot_override_on_mainnet(framework: &signer) acquires TimelockConfig {
        chain_id::initialize_for_test(framework, 1); // mainnet
        initialize(framework);

        // This should abort
        set_interval_for_testing(framework, 5 * 1000000);
    }

    #[test]
    fun test_get_default_when_not_initialized() acquires TimelockConfig {
        // Should return default even if not initialized
        assert!(get_interval_microseconds() == 3600 * 1000000, 0);
    }
}
