module aptos_framework::timelock {

    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::event::{Self, EventHandle, emit};
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::account;
    use aptos_framework::timelock_config;
    use aptos_framework::stake;
    use aptos_framework::validator_consensus_info;
    use aptos_std::crypto_algebra::{zero, add, serialize, deserialize};
    use aptos_std::bls12381_algebra::{G1, FormatG1Compr};
    use aptos_framework::chain_id;
    use aptos_std::bcs;

    // New modules
    use aptos_framework::threshold_dsa;
    use aptos_framework::ibe_signature; 

    friend aptos_framework::block;

    friend aptos_framework::genesis;

    /// # Atomica Timelock Service (IBE-based)
    ///
    /// This module implements the on-chain registry and orchestration for a Timelock Encryption service
    /// based on **Identity-Based Encryption (IBE)** as defined by Boneh and Franklin [BF01].
    ///
    /// ## References
    ///
    /// *   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."
    ///
    /// ## Protocol Overview
    ///
    /// The system treats time intervals as "Identities" in an IBE scheme.
    ///
    /// 1.  **Setup ($P_{pub}$)**: Validators engage in a Distributed Key Generation (DKG) to produce a shared Master Secret Key ($s$)
    ///     and publish the Master Public Key ($P_{pub} = s \cdot g_2$) on-chain.
    ///     *   See `publish_master_public_key`.
    ///
    /// 2.  **Encryption (Off-Chain)**: Users encrypt messages for a future time interval $T$ using $P_{pub}$ and identity $ID = T$.
    ///     *   $C = \text{Encrypt}(P_{pub}, ID, M)$.
    ///
    /// 3.  **Reveal / Extract ($d_{ID}$)**: When time $T$ arrives, validators compute partial private keys (signature shares) for $ID = T$.
    ///     *   Share: $\sigma_i = s_i \cdot H_1(ID)$.
    ///
    /// 4.  **Aggregation**: The contract verifies and aggregates these shares to reconstruct the full private key $d_{ID} = s \cdot H_1(ID)$.
    ///     *   This $d_{ID}$ allows anyone to decrypt $C$.
    ///     *   See `publish_decryption_key_share`.
    ///
    /// ## Architecture
    ///
    /// *   **`timelock.move`**: This module. Orchestrates the lifecycle (Intervals, Rotation, Reveal).
    /// *   **`threshold_dsa.move`**: Manages the underlying MPK storage and curve verification.
    /// *   **`ibe_signature.move`**: Defines the $H_1$ mapping from Identity to Point.

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
    /// Share verification failed against MPK/Identity.
    const ESHARE_VERIFICATION_FAILED: u64 = 6;

    struct TimelockConfig has copy, drop, store {
        threshold: u64,
        total_validators: u64,
    }

    struct IntervalConfig has store, drop, copy {
        threshold: u64,
        total_validators: u64,
        created_at: u64,  // timestamp
    }

    struct DecryptionKeyShare has store, drop {
        validator: address,
        share: vector<u8>,
    }

    struct TimelockState has key {
        current_interval: u64,
        last_rotation_time: u64,
        // master_public_keys moved to threshold_dsa
        /// Store collected key shares before aggregation
        decryption_key_shares: Table<u64, vector<DecryptionKeyShare>>,
        /// Store revealed decryption keys (DK)
        decryption_keys: Table<u64, vector<u8>>,
        /// Store historical interval configurations
        interval_configs: Table<u64, IntervalConfig>,
        /// Events
        // Events are now V2 (no handles stored)
    }

    // Event emitted to tell validators: "Please generate keys for interval X"
    #[event]
    struct StartKeyGenEvent has drop, store {
        interval: u64,
        config: TimelockConfig,
    }

    // Event emitted to tell validators: "Please reveal the secret for interval X"
    #[event]
    struct RequestRevealEvent has drop, store {
        interval: u64,
    }

    // Event emitted when a secret (DK) is fully reconstructed
    #[event]
    struct DecryptionKeyRevealedEvent has drop, store {
        interval: u64,
        decryption_key: vector<u8>,
    }

    /// Initialize the timelock system.
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        // Initialize dependency modules
        if (!exists<TimelockState>(@aptos_framework)) {
            // Ensure threshold_dsa is initialized
            threshold_dsa::initialize(framework);
            
            move_to(framework, TimelockState {
                current_interval: 0,
                last_rotation_time: 0, // Will be updated on first block
                decryption_key_shares: table::new(),
                decryption_keys: table::new(),
                interval_configs: table::new(),
            });
        }
    }

    /// Internal function to perform rotation logic
    fun perform_rotation(state: &mut TimelockState) {
        let now = timestamp::now_microseconds();
        let old_interval = state.current_interval;

        // Emit reveal event for the old interval
        emit(RequestRevealEvent {
            interval: old_interval,
        });

        state.current_interval = state.current_interval + 1;
        state.last_rotation_time = now;

        // DEBUG: Log interval rotation
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Interval rotated to"));
        aptos_std::debug::print(&state.current_interval);

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

        // NOTE: DKG (StartKeyGenEvent) is NOT emitted on interval rotation.
        // The MPK corresponds to the validator set, which changes on epoch boundaries.
        // IBE allows the same MPK to encrypt for any interval via identity derivation.
        // DKG is triggered separately via reconfiguration/epoch change events.
        let _ = config; // Suppress unused warning
    }

    /// Called by block prologue to trigger rotations.
    public(friend) fun on_new_block(vm: &signer) acquires TimelockState {
        system_addresses::assert_vm(vm);

        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));

        if (!exists<TimelockState>(@aptos_framework)) {
            aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] TimelockState does not exist - returning"));
            return
        };

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let now = timestamp::now_microseconds();

        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: current_interval="));
        aptos_std::debug::print(&state.current_interval);
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: now="));
        aptos_std::debug::print(&now);
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: last_rotation_time="));
        aptos_std::debug::print(&state.last_rotation_time);

        // Initialize last_rotation_time if it's 0 (genesis/first run)
        if (state.last_rotation_time == 0) {
            aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Initializing last_rotation_time to current time"));
            state.last_rotation_time = now;
            return
        };

        // Check if configured interval has passed (get from timelock_config)
        let interval_micros = timelock_config::get_interval_microseconds();
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] interval_micros="));
        aptos_std::debug::print(&interval_micros);

        let elapsed = now - state.last_rotation_time;
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] elapsed="));
        aptos_std::debug::print(&elapsed);
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] should_rotate="));
        aptos_std::debug::print(&(elapsed > interval_micros));

        if (now - state.last_rotation_time > interval_micros) {
            aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Calling perform_rotation"));
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
        assert!(chain_id::get() != 1, EROTATION_TOO_EARLY);

        if (!exists<TimelockState>(@aptos_framework)) {
            return
        };

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        perform_rotation(state);
    }

    /// Validators call this to publish the Master Public Key ($P_{pub}$) for a future interval.
    ///
    /// # [BF01] Setup Phase
    ///
    /// This corresponds to the **Setup** algorithm. Ideally, this runs once for the system lifetime or per epoch.
    /// The $P_{pub}$ is stored in `threshold_dsa` and allows users to derive Public Keys for any identity $ID$.
    public entry fun publish_master_public_key(
        validator: &signer,
        interval: u64,
        pk: vector<u8>
    ) {
        // Delegate to threshold_dsa module
        threshold_dsa::publish_master_public_key(validator, interval, pk);
    }

    /// Validators call this to publish their partial Decryption Key ($d_{ID}$) share for a past interval.
    ///
    /// # [BF01] Extract Phase (Distributed)
    ///
    /// When the time interval $ID$ passes, the "Private Key Generator" (PKG)—in this case, the validator set—
    /// cooperatively constructs the private key $d_{ID}$ corresponding to the identity $ID$.
    ///
    /// *   **Input**: Validator share $\sigma_i$.
    /// *   **Logic**:
    ///     1.  Verify $\sigma_i$ against $P_{pub}$ and $ID$ (using `ibe_signature::verify_private_key`).
    ///     2.  Accumulate shares until threshold is met.
    ///     3.  Aggregate to form $d_{ID} = \sum \sigma_i$.
    ///     4.  Publish $d_{ID}$.
    ///
    /// Once $d_{ID}$ is published, any ciphertext encrypted for $ID$ can be decrypted.
    public entry fun publish_decryption_key_share(
        validator: &signer,
        interval: u64,
        share: vector<u8>
    ) acquires TimelockState {
        let validator_addr = std::signer::address_of(validator);
        
        // 1. Verify validator authorization
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);

        // Security Check: Only allow revealing PAST intervals
        assert!(interval < state.current_interval, EINVALID_INTERVAL);

        // If outcome already revealed, ignore
        if (table::contains(&state.decryption_keys, interval)) {
            return
        };

        // 2. CRYPTOGRAPHIC VERIFICATION
        // Construct Identity from interval (u64 -> bytes)
        let identity = bcs::to_bytes(&interval);
        
        // Verify the share against the MPK for this interval
        // Note: verify_private_key handles MPK lookup in threshold_dsa
        let is_valid = ibe_signature::verify_private_key(interval, identity, share);
        assert!(is_valid, ESHARE_VERIFICATION_FAILED);

        // 3. Store valid share
        if (!table::contains(&state.decryption_key_shares, interval)) {
            table::add(&mut state.decryption_key_shares, interval, vector::empty());
        };
        let shares_list = table::borrow_mut(&mut state.decryption_key_shares, interval);
        
        // Dedup
        let i = 0;
        let len = vector::length(shares_list);
        while (i < len) {
            if (vector::borrow(shares_list, i).validator == validator_addr) {
                return 
            };
            i = i + 1;
        };

        // Share is already verified cryptographically, but we need to deserialize for aggregation.
        // deserialize should succeed if verify succeeded, but we check.
        let share_opt = deserialize<G1, FormatG1Compr>(&share);
        assert!(std::option::is_some(&share_opt), EINVALID_SHARE);

        vector::push_back(shares_list, DecryptionKeyShare {
            validator: validator_addr,
            share: share,
        });

        // 4. Check threshold
        assert!(table::contains(&state.interval_configs, interval), EINVALID_INTERVAL);
        let config = table::borrow(&state.interval_configs, interval);
        let threshold = config.threshold;
        let valid_count = vector::length(shares_list);

        if (valid_count >= threshold) {
            // 5. Aggregate
            let sum = zero<G1>();
            let i = 0;
            let len = vector::length(shares_list); 
            let aggregated_count = 0;
            
            while (i < len && aggregated_count < threshold) {
                let s_bytes = &vector::borrow(shares_list, i).share;
                let element_opt = deserialize<G1, FormatG1Compr>(s_bytes);
                if (std::option::is_some(&element_opt)) {
                    let element = std::option::extract(&mut element_opt);
                    sum = add(&sum, &element);
                    aggregated_count = aggregated_count + 1;
                };
                i = i + 1;
            };

            let aggregated_bytes = serialize<G1, FormatG1Compr>(&sum);
            table::add(&mut state.decryption_keys, interval, aggregated_bytes);

            // Emit event
            emit(DecryptionKeyRevealedEvent {
                interval,
                decryption_key: aggregated_bytes,
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
    public fun get_master_public_key(interval: u64): Option<vector<u8>> {
        // Delegate to threshold_dsa
        threshold_dsa::get_master_public_key(interval)
    }

    #[view]
    public fun is_decryption_key_revealed(interval: u64): bool acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return false
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        table::contains(&state.decryption_keys, interval)
    }

    #[view]
    public fun get_decryption_key(interval: u64): Option<vector<u8>> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) {
            return option::none()
        };
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.decryption_keys, interval)) {
            option::some(*table::borrow(&state.decryption_keys, interval))
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

        timestamp::update_global_time_for_test(1);
        on_new_block(&vm);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(state.last_rotation_time == 1, 99);

        timestamp::update_global_time_for_test(1 + 3600 * 1000000 + 1);
        on_new_block(&vm);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(state.current_interval == 1, 100);
    }
}
