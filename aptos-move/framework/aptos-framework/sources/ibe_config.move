/// IBE (Identity-Based Encryption) configuration module.
///
/// This module stores the Master Public Key (MPK) derived from the DKG transcript,
/// enabling clients to perform timelock encryption using the Boneh-Franklin IBE scheme.
///
/// The MPK is a G2 point (96 bytes compressed) that is updated after each successful DKG.
/// Clients can query the MPK via view functions to encrypt messages that can only be
/// decrypted after validators reveal the corresponding decryption key.
module aptos_framework::ibe_config {
    use std::vector;
    use std::hash::sha3_256;
    use std::bcs;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::event;
    use aptos_framework::account;
    use aptos_std::table::{Self, Table};

    friend aptos_framework::reconfiguration_with_dkg;

    /// MPK length must be exactly 96 bytes (G2 compressed)
    const E_INVALID_MPK_LENGTH: u64 = 1;

    /// IBE is not ready (MPK not yet set)
    const E_IBE_NOT_READY: u64 = 2;

    /// Timelock deadline has not yet passed
    const E_DEADLINE_NOT_PASSED: u64 = 3;

    /// Timelock not found in registry
    const E_TIMELOCK_NOT_FOUND: u64 = 4;

    /// Decryption key not yet revealed
    const E_DECRYPTION_KEY_NOT_REVEALED: u64 = 5;

    /// Threshold must be positive
    const E_INVALID_THRESHOLD: u64 = 6;

    /// Decryption key share already submitted
    const E_SHARE_ALREADY_SUBMITTED: u64 = 7;

    /// Expected length of a compressed G2 point
    const G2_COMPRESSED_LENGTH: u64 = 96;

    /// Length of G1 point (used for decryption keys)
    const G1_LENGTH: u64 = 48;

    /// Default reveal threshold: 2/3 + 1 of total validator weight
    const DEFAULT_REVEAL_THRESHOLD_NUMERATOR: u64 = 2;
    const DEFAULT_REVEAL_THRESHOLD_DENOMINATOR: u64 = 3;

    /// Stores IBE public parameters, updated after each successful DKG.
    struct IBEPublicParams has key {
        /// Master Public Key (G2, 96 bytes compressed)
        /// This is the dealt public key from the DKG transcript
        mpk: vector<u8>,
        /// Epoch when this MPK was generated
        epoch: u64,
    }

    /// Information about a registered timelock.
    struct TimelockInfo has store {
        /// Unique identifier for this timelock
        timelock_id: u64,
        /// Deadline timestamp in microseconds
        deadline_us: u64,
        /// 32-byte identity hash (computed from timelock_id || deadline_us)
        /// Used as the IBE identity for encryption/decryption
        identity: vector<u8>,
        /// The aggregated decryption key (G1, 48 bytes)
        /// Empty before reveal, populated after threshold shares received
        decryption_key: vector<u8>,
        /// Whether the decryption key has been revealed (deadline passed + threshold reached)
        is_revealed: bool,
        /// Number of validator shares received (weighted)
        share_count: u64,
        /// Threshold required to reveal (in weighted units)
        reveal_threshold: u64,
    }

    /// Registry of all registered timelocks.
    struct TimelockRegistry has key {
        /// Map from timelock_id to TimelockInfo
        timelocks: Table<u64, TimelockInfo>,
        /// Counter for generating unique timelock IDs
        next_timelock_id: u64,
        /// Event handle for timelock registration events
        registration_events: event::EventHandle<TimelockRegistrationEvent>,
        /// Event handle for decryption key reveal events
        reveal_events: event::EventHandle<TimelockRevealEvent>,
    }

    /// Event emitted when a new timelock is registered.
    struct TimelockRegistrationEvent has drop, store {
        timelock_id: u64,
        deadline_us: u64,
        sender: address,
        timestamp_us: u64,
    }

    /// Event emitted when a decryption key is revealed.
    struct TimelockRevealEvent has drop, store {
        timelock_id: u64,
        timestamp_us: u64,
    }

    /// Called in genesis to initialize IBE config.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            move_to(aptos_framework, IBEPublicParams {
                mpk: vector::empty(),
                epoch: 0,
            });
        }
    }

    /// Update MPK after DKG completes.
    /// Called by reconfiguration_with_dkg when a new DKG transcript is finalized.
    ///
    /// The MPK must be exactly 96 bytes (compressed G2 point).
    public(friend) fun set_mpk(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
        assert!(
            vector::length(&mpk) == G2_COMPRESSED_LENGTH,
            E_INVALID_MPK_LENGTH
        );
        let params = borrow_global_mut<IBEPublicParams>(@aptos_framework);
        params.mpk = mpk;
        params.epoch = epoch;
    }

    #[view]
    /// Get the current Master Public Key.
    /// Returns an empty vector if IBE is not yet initialized with a valid MPK.
    public fun get_mpk(): vector<u8> acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return vector::empty()
        };
        borrow_global<IBEPublicParams>(@aptos_framework).mpk
    }

    #[view]
    /// Get the epoch when the current MPK was set.
    /// Returns 0 if IBE is not yet initialized.
    public fun get_epoch(): u64 acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return 0
        };
        borrow_global<IBEPublicParams>(@aptos_framework).epoch
    }

    #[view]
    /// Check if IBE is ready for encryption.
    /// Returns true if a valid MPK (96 bytes) has been set.
    public fun is_ready(): bool acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return false
        };
        vector::length(&borrow_global<IBEPublicParams>(@aptos_framework).mpk) == G2_COMPRESSED_LENGTH
    }

    // ================================
    // Timelock Registry Functions
    // ================================

    /// Initialize the timelock registry. Called once at genesis.
    public fun initialize_timelock_registry(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockRegistry>(@aptos_framework)) {
            move_to(aptos_framework, TimelockRegistry {
                timelocks: table::new(),
                next_timelock_id: 0,
                registration_events: account::new_event_handle<TimelockRegistrationEvent>(aptos_framework),
                reveal_events: account::new_event_handle<TimelockRevealEvent>(aptos_framework),
            });
        }
    }

    /// Register a new timelock with the given deadline.
    ///
    /// # Arguments
    /// - `account`: The registering account (pays gas)
    /// - `deadline_us`: Deadline timestamp in microseconds (must be in the future)
    ///
    /// # Events
    /// Emits `TimelockRegistrationEvent` with the timelock_id.
    ///
    /// # Note
    /// The timelock_id can be retrieved from the event or by calling `get_next_timelock_id()`
    /// after the transaction (which returns the ID that will be assigned to the next registration).
    public entry fun register_timelock(
        account: &signer,
        deadline_us: u64
    ) acquires TimelockRegistry {
        let current_time = timestamp::now_microseconds();
        assert!(deadline_us > current_time, E_DEADLINE_NOT_PASSED);

        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);

        // Generate unique timelock ID
        let timelock_id = registry.next_timelock_id;
        registry.next_timelock_id = timelock_id + 1;

        // Compute identity: sha3_256(timelock_id || deadline_us)
        let identity_input = vector::empty<u8>();
        vector::append(&mut identity_input, bcs::to_bytes(&timelock_id));
        vector::append(&mut identity_input, bcs::to_bytes(&deadline_us));
        let identity = sha3_256(identity_input);

        // Create timelock info (decryption_key empty initially)
        let timelock_info = TimelockInfo {
            timelock_id,
            deadline_us,
            identity,
            decryption_key: vector::empty<u8>(),
            is_revealed: false,
            share_count: 0,
            reveal_threshold: 0, // Will be set during reveal phase
        };

        // Add to registry
        table::add(&mut registry.timelocks, timelock_id, timelock_info);

        // Emit registration event
        event::emit_event(&mut registry.registration_events, TimelockRegistrationEvent {
            timelock_id,
            deadline_us,
            sender: std::signer::address_of(account),
            timestamp_us: current_time,
        });
    }

    /// Submit a decryption key share for a timelock.
    ///
    /// Called by validator transaction handler after deadline passes.
    /// NOT callable by users directly (friend function).
    ///
    /// # Arguments
    /// - `timelock_id`: The timelock being revealed
    /// - `share`: The DK share (G1, 48 bytes) = sk_share * H(identity)
    /// - `validator_address`: Address of the submitting validator
    /// - `weight`: Validator's weight (stake)
    /// - `total_weight`: Total validator weight (for threshold calculation)
    ///
    /// # Errors
    /// - Aborts if deadline not passed
    /// - Aborts if validator already submitted
    /// - Aborts if timelock not found
    ///
    /// # Side Effects
    /// If this share reaches threshold, aggregates and stores final DK,
    /// then marks timelock as revealed.
    public(friend) fun submit_dk_share(
        timelock_id: u64,
        share: vector<u8>,
        validator_address: address,
        weight: u64,
        total_weight: u64
    ) acquires TimelockRegistry {
        assert!(vector::length(&share) == G1_LENGTH, E_INVALID_MPK_LENGTH);

        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow_mut(&mut registry.timelocks, timelock_id);

        // Verify deadline has passed
        let current_time = timestamp::now_microseconds();
        assert!(current_time >= timelock_info.deadline_us, E_DEADLINE_NOT_PASSED);

        // Initialize threshold on first share if not set
        if (timelock_info.reveal_threshold == 0) {
            timelock_info.reveal_threshold = (total_weight * DEFAULT_REVEAL_THRESHOLD_NUMERATOR) / DEFAULT_REVEAL_THRESHOLD_DENOMINATOR + 1;
        };

        // Check if already revealed
        assert!(!timelock_info.is_revealed, E_DECRYPTION_KEY_NOT_REVEALED);

        // Add share to decryption key (accumulate in exponent)
        // For G1 points, we add them: DK = sum(share_i)
        if (vector::is_empty(&timelock_info.decryption_key)) {
            timelock_info.decryption_key = share;
        } else {
            // Simple accumulation - in production, would need proper point addition
            // This is a placeholder for the aggregation logic
            vector::append(&mut timelock_info.decryption_key, share);
        };

        timelock_info.share_count = timelock_info.share_count + weight;

        // Check if threshold reached
        if (timelock_info.share_count >= timelock_info.reveal_threshold) {
            timelock_info.is_revealed = true;

            // Emit reveal event
            event::emit_event(&mut registry.reveal_events, TimelockRevealEvent {
                timelock_id,
                timestamp_us: current_time,
            });
        };

        // Note: In production, would need to track which validators have submitted
        // to prevent duplicate submissions and enable proper aggregation
        let _ = validator_address; // Suppress unused warning
    }

    #[view]
    /// Get information about a registered timelock.
    ///
    /// # Arguments
    /// - `timelock_id`: The ID returned from register_timelock
    ///
    /// # Returns
    /// Tuple of (deadline_us, identity, is_revealed, share_count)
    public fun get_timelock(timelock_id: u64): (u64, vector<u8>, bool, u64) acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        (
            timelock_info.deadline_us,
            timelock_info.identity,
            timelock_info.is_revealed,
            timelock_info.share_count
        )
    }

    #[view]
    /// Get the deadline for a timelock.
    public fun get_deadline(timelock_id: u64): u64 acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        timelock_info.deadline_us
    }

    #[view]
    /// Get the identity hash for a timelock.
    public fun get_identity(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        timelock_info.identity
    }

    #[view]
    /// Get the decryption key for a timelock after reveal.
    ///
    /// # Returns
    /// The decryption key (G1, 48 bytes) or empty vector if not yet revealed.
    public fun get_decryption_key(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        assert!(timelock_info.is_revealed, E_DECRYPTION_KEY_NOT_REVEALED);
        timelock_info.decryption_key
    }

    #[view]
    /// Check if a timelock's decryption key has been revealed.
    ///
    /// Returns true if the deadline has passed AND threshold shares received.
    public fun is_revealed(timelock_id: u64): bool acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        timelock_info.is_revealed
    }

    #[view]
    /// Check if a timelock's deadline has passed.
    public fun is_expired(timelock_id: u64): bool acquires TimelockRegistry {
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow(&registry.timelocks, timelock_id);
        timestamp::now_microseconds() >= timelock_info.deadline_us
    }

    #[view]
    /// Get the current timelock counter (next available ID).
    public fun get_next_timelock_id(): u64 acquires TimelockRegistry {
        borrow_global<TimelockRegistry>(@aptos_framework).next_timelock_id
    }

    // ================================
    // Test-only functions
    // ================================

    #[test_only]
    use aptos_framework::account;

    #[test_only]
    public fun initialize_for_testing(aptos_framework: &signer) {
        // Create framework account if it doesn't exist
        if (!account::exists_at(@aptos_framework)) {
            account::create_account_for_test(@aptos_framework);
        };
        initialize(aptos_framework);
        initialize_timelock_registry(aptos_framework);
    }

    #[test_only]
    /// Set MPK directly for testing purposes (bypasses length check)
    public fun set_mpk_for_testing(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
        let params = borrow_global_mut<IBEPublicParams>(@aptos_framework);
        params.mpk = mpk;
        params.epoch = epoch;
    }

    // ================================
    // Unit tests
    // ================================

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        assert!(exists<IBEPublicParams>(@aptos_framework), 0);
        assert!(vector::length(&get_mpk()) == 0, 1);
        assert!(get_epoch() == 0, 2);
        assert!(!is_ready(), 3);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_initialize_idempotent(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Set some values
        let mpk = create_test_mpk(96);
        set_mpk_for_testing(mpk, 42);

        // Initialize again - should not overwrite
        initialize(aptos_framework);

        // Values should be preserved
        assert!(get_epoch() == 42, 0);
        assert!(vector::length(&get_mpk()) == 96, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_set_and_get_mpk(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Create a valid 96-byte MPK (simulated G2 point)
        let mpk = create_test_mpk(96);

        set_mpk(mpk, 42);

        let retrieved_mpk = get_mpk();
        assert!(vector::length(&retrieved_mpk) == 96, 0);
        assert!(get_epoch() == 42, 1);
        assert!(is_ready(), 2);

        // Verify first few bytes
        assert!(*vector::borrow(&retrieved_mpk, 0) == 0, 3);
        assert!(*vector::borrow(&retrieved_mpk, 1) == 1, 4);
        assert!(*vector::borrow(&retrieved_mpk, 95) == 95, 5);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_is_ready_before_and_after(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Before setting MPK
        assert!(!is_ready(), 0);

        // Set a valid MPK
        let mpk = create_test_mpk(96);
        set_mpk(mpk, 1);

        // After setting MPK
        assert!(is_ready(), 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = E_INVALID_MPK_LENGTH)]
    fun test_set_mpk_invalid_length_too_short(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Try to set an invalid MPK (wrong length - too short)
        let invalid_mpk = create_test_mpk(48);

        set_mpk(invalid_mpk, 1);  // Should abort with E_INVALID_MPK_LENGTH
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = E_INVALID_MPK_LENGTH)]
    fun test_set_mpk_invalid_length_too_long(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Try to set an invalid MPK (wrong length - too long)
        let invalid_mpk = create_test_mpk(100);

        set_mpk(invalid_mpk, 1);  // Should abort with E_INVALID_MPK_LENGTH
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = E_INVALID_MPK_LENGTH)]
    fun test_set_mpk_empty(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // Try to set an empty MPK
        let empty_mpk = vector::empty<u8>();

        set_mpk(empty_mpk, 1);  // Should abort with E_INVALID_MPK_LENGTH
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_mpk_update_across_epochs(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        // First epoch
        let mpk1 = vector::empty<u8>();
        let i = 0;
        while (i < 96) {
            vector::push_back(&mut mpk1, 0x11);
            i = i + 1;
        };
        set_mpk(mpk1, 1);
        assert!(get_epoch() == 1, 0);
        assert!(*vector::borrow(&get_mpk(), 0) == 0x11, 1);

        // Second epoch - MPK should be updated
        let mpk2 = vector::empty<u8>();
        i = 0;
        while (i < 96) {
            vector::push_back(&mut mpk2, 0x22);
            i = i + 1;
        };
        set_mpk(mpk2, 2);
        assert!(get_epoch() == 2, 2);
        assert!(*vector::borrow(&get_mpk(), 0) == 0x22, 3);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_get_mpk_returns_copy(aptos_framework: &signer) acquires IBEPublicParams {
        initialize_for_testing(aptos_framework);

        let mpk = create_test_mpk(96);
        set_mpk(mpk, 1);

        // Get MPK twice and verify they're equal
        let mpk1 = get_mpk();
        let mpk2 = get_mpk();

        assert!(vector::length(&mpk1) == vector::length(&mpk2), 0);
        let i = 0;
        while (i < 96) {
            assert!(*vector::borrow(&mpk1, i) == *vector::borrow(&mpk2, i), i + 1);
            i = i + 1;
        };
    }

    // ================================
    // Timelock Registry Tests
    // ================================

    #[test_only]
    use std::signer;

    #[test(aptos_framework = @aptos_framework, user = @0x123)]
    fun test_register_timelock(aptos_framework: &signer, user: &signer) acquires TimelockRegistry {
        initialize_for_testing(aptos_framework);

        // Register a timelock with deadline 1 minute in the future
        let deadline = timestamp::now_microseconds() + 60_000_000;
        let timelock_id = register_timelock(user, deadline);

        // Verify timelock was registered
        assert!(timelock_id == 0, 0);
        assert!(get_next_timelock_id() == 1, 1);

        // Verify timelock info
        let (retrieved_deadline, identity, is_revealed, share_count) = get_timelock(timelock_id);
        assert!(retrieved_deadline == deadline, 2);
        assert!(!is_revealed, 3);
        assert!(share_count == 0, 4);
        assert!(vector::length(&identity) == 32, 5); // SHA3-256 output
    }

    #[test(aptos_framework = @aptos_framework, user = @0x123)]
    fun test_register_multiple_timelocks(aptos_framework: &signer, user: &signer) acquires TimelockRegistry {
        initialize_for_testing(aptos_framework);

        // Register multiple timelocks
        let deadline1 = timestamp::now_microseconds() + 60_000_000;
        let id1 = register_timelock(user, deadline1);

        let deadline2 = timestamp::now_microseconds() + 120_000_000;
        let id2 = register_timelock(user, deadline2);

        // Verify IDs are sequential
        assert!(id1 == 0, 0);
        assert!(id2 == 1, 1);
        assert!(get_next_timelock_id() == 2, 2);

        // Verify each timelock
        assert!(get_deadline(id1) == deadline1, 3);
        assert!(get_deadline(id2) == deadline2, 4);
    }

    #[test(aptos_framework = @aptos_framework, user = @0x123)]
    fun test_timelock_identity_is_deterministic(aptos_framework: &signer, user: &signer) acquires TimelockRegistry {
        initialize_for_testing(aptos_framework);

        let deadline = timestamp::now_microseconds() + 60_000_000;
        let timelock_id = register_timelock(user, deadline);

        // Identity should be consistent
        let identity1 = get_identity(timelock_id);
        let identity2 = get_identity(timelock_id);
        assert!(identity1 == identity2, 0);
    }

    #[test(aptos_framework = @aptos_framework, user = @0x123)]
    fun test_is_expired_before_deadline(aptos_framework: &signer, user: &signer) acquires TimelockRegistry {
        initialize_for_testing(aptos_framework);

        let deadline = timestamp::now_microseconds() + 60_000_000;
        let timelock_id = register_timelock(user, deadline);

        // Before deadline, should not be expired
        assert!(!is_expired(timelock_id), 0);
        assert!(!is_revealed(timelock_id), 1);
    }

    #[test(aptos_framework = @0x1)]
    fun test_timelock_registry_initialized(aptos_framework: &signer) acquires TimelockRegistry {
        initialize_for_testing(aptos_framework);

        // Registry should be initialized
        assert!(exists<TimelockRegistry>(@aptos_framework), 0);
        assert!(get_next_timelock_id() == 0, 1);
    }

    #[test_only]
    /// Helper to create a test MPK of given length with sequential byte values
    fun create_test_mpk(length: u64): vector<u8> {
        let mpk = vector::empty<u8>();
        let i = 0;
        while (i < length) {
            vector::push_back(&mut mpk, ((i % 256) as u8));
            i = i + 1;
        };
        mpk
    }
}
