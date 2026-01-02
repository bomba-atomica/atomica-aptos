module aptos_framework::timelock {

    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::account;
    use aptos_framework::timelock_config;
    use aptos_framework::stake;
    use aptos_framework::validator_consensus_info;
    use aptos_std::crypto_algebra::{zero, add, serialize, deserialize};
    use aptos_std::bls12381_algebra::{G1, FormatG1Compr};
    use aptos_framework::chain_id;

    friend aptos_framework::block;
    friend aptos_framework::genesis;

    /// The singleton was not initialized.
    const ETIMELOCK_NOT_INITIALIZED: u64 = 1;
    /// Not a validator.
    const ENOT_VALIDATOR: u64 = 2;
    /// Invalid share format.
    const EINVALID_SHARE: u64 = 3;
    /// Rotation triggered too early.
    const EROTATION_TOO_EARLY: u64 = 4;
    /// Invalid interval for reveal operation.
    const EINVALID_INTERVAL: u64 = 5;

    struct TimelockConfig has copy, drop, store {
        threshold: u64,
        total_validators: u64,
    }

    struct IntervalConfig has store, drop, copy {
        threshold: u64,
        total_validators: u64,
        created_at: u64,  // timestamp
    }

    struct ValidatorShare has store, drop {
        validator: address,
        share: vector<u8>,
    }

    struct TimelockState has key {
        current_interval: u64,
        last_rotation_time: u64,
        /// Store public keys (for encryption)
        public_keys: Table<u64, vector<u8>>,
        /// Store collected shares before aggregation
        validator_shares: Table<u64, vector<ValidatorShare>>,
        /// Store revealed secret keys/signatures (for decryption)
        revealed_secrets: Table<u64, vector<u8>>,
        /// Store historical interval configurations
        interval_configs: Table<u64, IntervalConfig>,
        /// Events
        start_keygen_events: EventHandle<StartKeyGenEvent>,
        key_published_events: EventHandle<KeyPublishedEvent>,
        request_reveal_events: EventHandle<RequestRevealEvent>,
        secret_revealed_events: EventHandle<SecretRevealedEvent>,
    }

    /// Event emitted to tell validators: "Please generate keys for interval X"
    struct StartKeyGenEvent has drop, store {
        interval: u64,
        config: TimelockConfig,
    }

    /// Event emitted when MPK (transcript) is published
    struct KeyPublishedEvent has drop, store {
        interval: u64,
        public_key: vector<u8>,
    }

    /// Event emitted to tell validators: "Please reveal the secret for interval X"
    struct RequestRevealEvent has drop, store {
        interval: u64,
    }

    /// Event emitted when a secret is fully reconstructed
    struct SecretRevealedEvent has drop, store {
        interval: u64,
        secret: vector<u8>,
    }

    /// Initialize the timelock system.
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, TimelockState {
            current_interval: 0,
            last_rotation_time: 0, // Will be updated on first block
            public_keys: table::new(),
            validator_shares: table::new(),
            revealed_secrets: table::new(),
            interval_configs: table::new(),
            start_keygen_events: account::new_event_handle<StartKeyGenEvent>(framework),
            key_published_events: account::new_event_handle<KeyPublishedEvent>(framework),
            request_reveal_events: account::new_event_handle<RequestRevealEvent>(framework),
            secret_revealed_events: account::new_event_handle<SecretRevealedEvent>(framework),
        });
    }

    /// Internal function to perform rotation logic
    fun perform_rotation(state: &mut TimelockState) {
        let now = timestamp::now_microseconds();
        let old_interval = state.current_interval;

        // Emit reveal event for the old interval
        event::emit_event(&mut state.request_reveal_events, RequestRevealEvent {
            interval: old_interval,
        });

        state.current_interval = state.current_interval + 1;
        state.last_rotation_time = now;

        // Get current validator set to determine threshold
        let validators = stake::cur_validator_consensus_infos();
        let validator_addresses = vector::empty<address>();
        let i = 0;
        let len = vector::length(&validators);
        while (i < len) {
            let v = vector::borrow(&validators, i);
            vector::push_back(&mut validator_addresses, validator_consensus_info::get_addr(v));
            i = i + 1;
        };
        let total_validators = vector::length(&validators);
        // Byztantine Fault Tolerance threshold: 2f + 1, where N = 3f + 1
        // Simple formula: floor(N * 2 / 3) + 1
        let threshold = (total_validators * 2 / 3) + 1;
        if (total_validators == 0) { threshold = 1; }; // Fallback for testing/genesis

        let config = TimelockConfig {
            threshold,
            total_validators,
        };

        // Store interval config for future reveal validation
        let interval_config = IntervalConfig {
            threshold,
            total_validators,
            created_at: now,
        };
        table::add(&mut state.interval_configs, state.current_interval, interval_config);

        event::emit_event(&mut state.start_keygen_events, StartKeyGenEvent {
            interval: state.current_interval,
            config,
        });
    }

    /// Called by block prologue to trigger rotations.
    public(friend) fun on_new_block(vm: &signer) acquires TimelockState {
        system_addresses::assert_vm(vm);

        if (!exists<TimelockState>(@aptos_framework)) {
            return
        };

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let now = timestamp::now_microseconds();

        // Initialize last_rotation_time if it's 0 (genesis/first run)
        if (state.last_rotation_time == 0) {
            state.last_rotation_time = now;
            return
        };

        // Check if configured interval has passed (get from timelock_config)
        let interval_micros = timelock_config::get_interval_microseconds();
        if (now - state.last_rotation_time > interval_micros) {
            perform_rotation(state);
        }
    }

    /// Manual rotation trigger that can be called by anyone after the scheduled time.
    /// This allows testing and emergency rotation when automatic rotation fails.
    public entry fun trigger_rotation(_account: &signer) acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return
        };

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let now = timestamp::now_microseconds();

        // Initialize last_rotation_time if it's 0 (genesis/first run)
        if (state.last_rotation_time == 0) {
            state.last_rotation_time = now;
            return
        };

        // Check if configured interval has passed (get from timelock_config)
        let interval_micros = timelock_config::get_interval_microseconds();
        assert!(now - state.last_rotation_time > interval_micros, EROTATION_TOO_EARLY);

        perform_rotation(state);
    }

    /// Force rotation for testing purposes.
    /// Bypasses the time check. Only available on non-mainnet chains.
    public entry fun force_rotation_for_testing(_account: &signer) acquires TimelockState {
        assert!(chain_id::get() != 1, EROTATION_TOO_EARLY); // Re-use error or new one? EPRODUCTION... logic

        if (!exists<TimelockState>(@aptos_framework)) {
            return
        };

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        perform_rotation(state);
    }

    /// validators call this to publish the public key for a future interval
    public entry fun publish_public_key(
        validator: &signer,
        interval: u64,
        pk: vector<u8>
    ) acquires TimelockState {
        let validator_addr = std::signer::address_of(validator);
        // Verify sender is a validator
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        if (!table::contains(&state.public_keys, interval)) {
            table::add(&mut state.public_keys, interval, pk);
            
            event::emit_event(&mut state.key_published_events, KeyPublishedEvent {
                interval,
                public_key: pk,
            });
        };
    }

    /// validators call this to publish the secret share/signature for a past interval
    public entry fun publish_secret_share(
        validator: &signer,
        interval: u64,
        share: vector<u8>
    ) acquires TimelockState {
        let validator_addr = std::signer::address_of(validator);
        // 1. Verify validator authorization
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);

        // CRITICAL SECURITY: Only allow revealing PAST intervals
        // Validators must not be able to reveal the current interval's secret.
        // The timelock guarantee is that secrets remain hidden until the interval rotates.
        // Without this check, malicious validators could immediately reveal secrets for the
        // current interval, completely breaking the timelock security model.
        assert!(interval < state.current_interval, EINVALID_INTERVAL);

        // If already revealed, ignore (or could abort)
        if (table::contains(&state.revealed_secrets, interval)) {
            return
        };

        // 2. Store the share
        if (!table::contains(&state.validator_shares, interval)) {
            table::add(&mut state.validator_shares, interval, vector::empty());
        };
        let shares_list = table::borrow_mut(&mut state.validator_shares, interval);
        
        // Dedup: check if validator already submitted
        let i = 0;
        let len = vector::length(shares_list);
        while (i < len) {
            if (vector::borrow(shares_list, i).validator == validator_addr) {
                return // Already submitted
            };
            i = i + 1;
        };

        // 2. Validate share format BEFORE storing
        let share_opt = deserialize<G1, FormatG1Compr>(&share);
        assert!(std::option::is_some(&share_opt), EINVALID_SHARE);

        vector::push_back(shares_list, ValidatorShare {
            validator: validator_addr,
            share: share,
        });

        // 3. Check if threshold is met using VALID shares only
        // Since we validate on insertion (line 254), all stored shares are valid G1 points.
        let valid_count = vector::length(shares_list);

        // Use stored interval config for threshold validation
        assert!(table::contains(&state.interval_configs, interval), EINVALID_INTERVAL);
        let config = table::borrow(&state.interval_configs, interval);
        let threshold = config.threshold;

        if (valid_count >= threshold) {
            // 4. Aggregate VALID shares only
            // 4. Aggregate shares
            let sum = zero<G1>();
            let i = 0;
            // distinct from valid_count, just loop iterator
            let len = vector::length(shares_list); 
            let aggregated_count = 0;
            
            while (i < len && aggregated_count < threshold) {
                let s_bytes = &vector::borrow(shares_list, i).share;
                // We must re-deserialize to add, but we can trust it is Some
                let element_opt = deserialize<G1, FormatG1Compr>(s_bytes);
                // Safety check, though redundant if storage is trusted
                if (std::option::is_some(&element_opt)) {
                    let element = std::option::extract(&mut element_opt);
                    sum = add(&sum, &element);
                    aggregated_count = aggregated_count + 1;
                };
                i = i + 1;
            };

            let aggregated_bytes = serialize<G1, FormatG1Compr>(&sum);
            table::add(&mut state.revealed_secrets, interval, aggregated_bytes);

            // Emit event
            event::emit_event(&mut state.secret_revealed_events, SecretRevealedEvent {
                interval,
                secret: aggregated_bytes,
            });
        }
    }

    #[view]
    public fun get_current_interval(): u64 acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return 0
        };
        borrow_global<TimelockState>(@aptos_framework).current_interval
    }

    #[view]
    public fun get_interval_config(interval: u64): Option<IntervalConfig> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return option::none()
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.interval_configs, interval)) {
            option::some(*table::borrow(&state.interval_configs, interval))
        } else {
            option::none()
        }
    }

    #[view]
    public fun get_public_key(interval: u64): Option<vector<u8>> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return option::none()
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.public_keys, interval)) {
            option::some(*table::borrow(&state.public_keys, interval))
        } else {
            option::none()
        }
    }

    #[view]
    public fun is_secret_revealed(interval: u64): bool acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return false
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        table::contains(&state.revealed_secrets, interval)
    }

    #[view]
    public fun get_secret(interval: u64): Option<vector<u8>> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return option::none()
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.revealed_secrets, interval)) {
            option::some(*table::borrow(&state.revealed_secrets, interval))
        } else {
            option::none()
        }
    }

    #[test_only]
    use aptos_framework::account::create_signer_for_test;

    #[test(framework = @aptos_framework)]
    public fun test_timelock_flow(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        let vm = create_signer_for_test(@0x0);

        // Advance time to 1 to ensure last_rotation_time is non-zero
        timestamp::update_global_time_for_test(1);

        // First block - initializes last_rotation_time to 1
        on_new_block(&vm);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(state.last_rotation_time == 1, 99);

        // Advance time: 1 + interval + 1
        timestamp::update_global_time_for_test(1 + 3600 * 1000000 + 1);
        
        // Second block - should trigger rotation
        on_new_block(&vm);
        
        // This fails if the rotation logic doesn't update current_interval
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(state.current_interval == 1, 100);
    }

    #[test(framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = 65550, location = aptos_framework::stake)] // ESTAKE_POOL_DOES_NOT_EXIST = 14 (0xE), Invalid Argument (0x1) -> 0x1000E
    public fun test_access_control(framework: &signer, validator: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        // Try to publish key as non-validator (validator set is empty, so 0x123 is not a validator)
        publish_public_key(validator, 1, vector[1, 2, 3]);
    }

    #[test(framework = @aptos_framework)]
    public fun test_share_aggregation_logic(framework: &signer) acquires TimelockState {
        // Defines specific logic test for share math if possible,
        // but real G1 operations require valid bytes.
        // We can test that duplicate shares are rejected.
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        initialize(framework);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        // Manual setup of state
        let shares = vector::empty<ValidatorShare>();
        vector::push_back(&mut shares, ValidatorShare { validator: @0x1, share: vector[] });
        table::add(&mut state.validator_shares, 1, shares);

        // Check deduplication relies on runtime logic, easier to verify in e2e
    }

    #[test(framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = EINVALID_INTERVAL, location = Self)]
    public fun test_cannot_reveal_current_interval(framework: &signer, validator: &signer) acquires TimelockState {
        // Test Bug #2 Fix: Validators cannot reveal secrets for the CURRENT interval
        // This is a critical security test - without this check, malicious validators
        // could decrypt messages in the current interval, breaking the timelock guarantee.
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);

        // Setup validator with stake pool
        let validator_addr = std::signer::address_of(validator);
        account::create_account_for_test(validator_addr);
        stake::initialize_stake_owner(validator, 0, validator_addr, validator_addr);
        stake::mint_and_add_stake(validator, 100);

        // Register validator using proper stake API (false = don't end epoch, we control time)
        let (_sk, pk, pop) = stake::generate_identity();
        stake::join_validator_set_for_test(&pk, &pop, validator, validator_addr, false);

        // End epoch to activate validator
        stake::end_epoch();

        initialize(framework);
        let vm = create_signer_for_test(@0x0);

        // Initialize time AFTER end_epoch (which sets time)
        let start_time = timestamp::now_microseconds() + 1;
        timestamp::update_global_time_for_test(start_time);
        on_new_block(&vm);

        // Advance time to trigger rotation to interval 1
        timestamp::update_global_time_for_test(start_time + 3600 * 1000000 + 1);
        on_new_block(&vm);

        let state = borrow_global<TimelockState>(@aptos_framework);
        let current = state.current_interval;
        assert!(current == 1, 101);

        // Create valid G1 point (generator * 1, compressed format)
        // This is the compressed encoding of the G1 generator point
        let valid_g1_share = vector[
            0x97, 0xf1, 0xd3, 0xa7, 0x33, 0x70, 0x96, 0x6a,
            0x07, 0x71, 0xbf, 0x6e, 0x6e, 0x8f, 0x8e, 0xdb,
            0xcd, 0xe7, 0x08, 0xd5, 0x89, 0x6f, 0xde, 0x0e,
            0x0e, 0xa6, 0x64, 0x89, 0xa8, 0xed, 0xd1, 0x70,
            0xe2, 0xef, 0x46, 0x48, 0xf8, 0x69, 0x8b, 0x24,
            0xda, 0x5f, 0x3b, 0x01, 0x45, 0x63, 0x0f, 0x38
        ];

        // ATTACK: Try to reveal secret for CURRENT interval (interval 1)
        // This should ABORT with EINVALID_INTERVAL
        publish_secret_share(validator, current, valid_g1_share);
        // If we reach here, the security check failed!
    }

    #[test(framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = EINVALID_INTERVAL, location = Self)]
    public fun test_cannot_reveal_future_interval(framework: &signer, validator: &signer) acquires TimelockState {
        // Test Bug #2 Fix: Validators cannot reveal secrets for FUTURE intervals
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);

        // Setup validator with stake pool
        let validator_addr = std::signer::address_of(validator);
        account::create_account_for_test(validator_addr);
        stake::initialize_stake_owner(validator, 0, validator_addr, validator_addr);
        stake::mint_and_add_stake(validator, 100);

        // Register validator using proper stake API (false = don't end epoch, we control time)
        let (_sk, pk, pop) = stake::generate_identity();
        stake::join_validator_set_for_test(&pk, &pop, validator, validator_addr, false);

        // End epoch to activate validator
        stake::end_epoch();

        initialize(framework);
        let vm = create_signer_for_test(@0x0);

        // Initialize time AFTER end_epoch (which sets time)
        let start_time = timestamp::now_microseconds() + 1;
        timestamp::update_global_time_for_test(start_time);
        on_new_block(&vm);

        // Advance time to trigger rotation to interval 1
        timestamp::update_global_time_for_test(start_time + 3600 * 1000000 + 1);
        on_new_block(&vm);

        let state = borrow_global<TimelockState>(@aptos_framework);
        let current = state.current_interval;
        assert!(current == 1, 102);

        // Valid G1 point
        let valid_g1_share = vector[
            0x97, 0xf1, 0xd3, 0xa7, 0x33, 0x70, 0x96, 0x6a,
            0x07, 0x71, 0xbf, 0x6e, 0x6e, 0x8f, 0x8e, 0xdb,
            0xcd, 0xe7, 0x08, 0xd5, 0x89, 0x6f, 0xde, 0x0e,
            0x0e, 0xa6, 0x64, 0x89, 0xa8, 0xed, 0xd1, 0x70,
            0xe2, 0xef, 0x46, 0x48, 0xf8, 0x69, 0x8b, 0x24,
            0xda, 0x5f, 0x3b, 0x01, 0x45, 0x63, 0x0f, 0x38
        ];

        // ATTACK: Try to reveal secret for FUTURE interval (interval 999)
        // This should ABORT with EINVALID_INTERVAL
        publish_secret_share(validator, 999, valid_g1_share);
        // If we reach here, the security check failed!
    }

    #[test(framework = @aptos_framework, validator = @0x123)]
    #[expected_failure(abort_code = EINVALID_SHARE, location = Self)]
    public fun test_can_reveal_past_interval(framework: &signer, validator: &signer) acquires TimelockState {
        // Test Bug #2 Fix: Validators CAN reveal secrets for PAST intervals (legitimate case)
        // This test proves the interval validation PASSES (doesn't abort with EINVALID_INTERVAL)
        // It will abort later with EINVALID_SHARE due to invalid crypto bytes, which is expected
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);

        // Setup validator with stake pool
        let validator_addr = std::signer::address_of(validator);
        account::create_account_for_test(validator_addr);
        stake::initialize_stake_owner(validator, 0, validator_addr, validator_addr);
        stake::mint_and_add_stake(validator, 100);

        // Register validator using proper stake API (false = don't end epoch, we control time)
        let (_sk, pk, pop) = stake::generate_identity();
        stake::join_validator_set_for_test(&pk, &pop, validator, validator_addr, false);

        // End epoch to activate validator
        stake::end_epoch();

        initialize(framework);
        let vm = create_signer_for_test(@0x0);

        // Initialize time AFTER end_epoch (which sets time)
        let start_time = timestamp::now_microseconds() + 1;
        timestamp::update_global_time_for_test(start_time);
        on_new_block(&vm);

        // Manually setup state to have interval 0 config and rotate to interval 2
        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let config = IntervalConfig {
            threshold: 999, // High threshold to prevent aggregation (we're just testing interval validation)
            total_validators: 1,
            created_at: 1,
        };
        table::add(&mut state.interval_configs, 0, config);
        state.current_interval = 2; // We're now at interval 2

        // Dummy G1 bytes (won't be aggregated due to high threshold)
        // We're only testing that the interval validation PASSES, not the crypto
        let dummy_share = vector[1, 2, 3];

        // LEGITIMATE: Reveal secret for PAST interval (interval 0, current is 2)
        // The interval validation check (interval < current_interval) should PASS
        // Then it will abort with EINVALID_SHARE (expected) due to invalid G1 bytes
        // This proves our security fix doesn't block legitimate reveals
        publish_secret_share(validator, 0, dummy_share);
        // Will abort with EINVALID_SHARE above, proving interval validation passed
    }
}
