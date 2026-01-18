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
    use aptos_framework::system_addresses;

    friend aptos_framework::reconfiguration_with_dkg;

    /// MPK length must be exactly 96 bytes (G2 compressed)
    const E_INVALID_MPK_LENGTH: u64 = 1;

    /// IBE is not ready (MPK not yet set)
    const E_IBE_NOT_READY: u64 = 2;

    /// Expected length of a compressed G2 point
    const G2_COMPRESSED_LENGTH: u64 = 96;

    /// Stores IBE public parameters, updated after each successful DKG.
    struct IBEPublicParams has key {
        /// Master Public Key (G2, 96 bytes compressed)
        /// This is the dealt public key from the DKG transcript
        mpk: vector<u8>,
        /// Epoch when this MPK was generated
        epoch: u64,
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
    // PHASE 2: Timelock Registry (STUBS)
    // ================================
    //
    // The following structs and functions are stubs for Phase 2 implementation.
    // They define the interface for timelock registration and tracking.
    //
    // ## Overview
    //
    // The Timelock Registry allows users to:
    // 1. Register a timelock with a future deadline
    // 2. Query timelock status (pending, revealed)
    // 3. Retrieve the decryption key after reveal
    //
    // ## Workflow
    //
    // ```
    // User                   Registry                 Validators
    //  │                        │                         │
    //  │ register_timelock()    │                         │
    //  ├───────────────────────>│                         │
    //  │   (returns timelock_id)│                         │
    //  │                        │                         │
    //  │ [time passes...]       │                         │
    //  │                        │                         │
    //  │                        │<── submit_dk_share() ───┤
    //  │                        │         (Phase 3)       │
    //  │                        │                         │
    //  │ get_decryption_key()   │                         │
    //  ├───────────────────────>│                         │
    //  │   (returns DK)         │                         │
    // ```
    //
    // ## TODO
    //
    // - [ ] Implement TimelockInfo struct
    // - [ ] Implement TimelockRegistry struct
    // - [ ] Implement register_timelock entry function
    // - [ ] Implement get_timelock view function
    // - [ ] Implement get_decryption_key view function
    // - [ ] Add Table import for registry storage
    // - [ ] Add unit tests

    // --------------------------------
    // Phase 2 Structs (TODO)
    // --------------------------------

    // TODO: Uncomment and implement when ready
    //
    // /// Information about a registered timelock.
    // ///
    // /// Created when a user calls `register_timelock()` and updated
    // /// as validators submit DK shares after the deadline.
    // struct TimelockInfo has store {
    //     /// Unique identifier for this timelock
    //     timelock_id: u64,
    //     /// Deadline after which the decryption key can be revealed
    //     deadline_timestamp_us: u64,
    //     /// 32-byte identity hash (computed from timelock_id + deadline)
    //     /// Used as the IBE identity for encryption/decryption
    //     identity: vector<u8>,
    //     /// The aggregated decryption key (G1, 48 bytes)
    //     /// None before reveal, Some after threshold shares received
    //     decryption_key: Option<vector<u8>>,
    //     /// Number of validator shares required to reveal
    //     /// (2/3 + 1 of total weight)
    //     reveal_threshold: u64,
    //     /// Current count of received shares (weighted)
    //     share_count: u64,
    // }
    //
    // /// Registry of all active timelocks.
    // ///
    // /// Stored at @aptos_framework, initialized at genesis.
    // struct TimelockRegistry has key {
    //     /// Map from timelock_id to TimelockInfo
    //     deadlines: Table<u64, TimelockInfo>,
    //     /// Counter for generating unique timelock IDs
    //     next_timelock_id: u64,
    // }

    // --------------------------------
    // Phase 2 Functions (TODO)
    // --------------------------------

    // TODO: Implement these functions
    //
    // /// Register a new timelock with the given deadline.
    // ///
    // /// # Arguments
    // /// - `account`: The registering account (pays gas)
    // /// - `deadline_us`: Deadline timestamp in microseconds
    // ///
    // /// # Returns
    // /// The unique timelock_id for this registration.
    // ///
    // /// # Example
    // /// ```move
    // /// let deadline = timestamp::now_microseconds() + 60_000_000; // 1 minute
    // /// let timelock_id = ibe_config::register_timelock(account, deadline);
    // /// ```
    // public entry fun register_timelock(
    //     account: &signer,
    //     deadline_us: u64
    // ): u64 acquires TimelockRegistry {
    //     // TODO: Implementation
    //     // 1. Get next_timelock_id from registry
    //     // 2. Compute identity = sha3_256(timelock_id || deadline_us)
    //     // 3. Create TimelockInfo
    //     // 4. Add to registry table
    //     // 5. Increment next_timelock_id
    //     // 6. Return timelock_id
    //     abort E_IBE_NOT_READY
    // }
    //
    // #[view]
    // /// Get information about a registered timelock.
    // ///
    // /// # Arguments
    // /// - `timelock_id`: The ID returned from register_timelock
    // ///
    // /// # Returns
    // /// TimelockInfo struct (or aborts if not found)
    // public fun get_timelock(timelock_id: u64): TimelockInfo acquires TimelockRegistry {
    //     // TODO: Implementation
    //     abort E_IBE_NOT_READY
    // }
    //
    // #[view]
    // /// Get the decryption key for a timelock after reveal.
    // ///
    // /// # Arguments
    // /// - `timelock_id`: The ID returned from register_timelock
    // ///
    // /// # Returns
    // /// The decryption key (G1, 48 bytes) or empty vector if not yet revealed.
    // ///
    // /// # Usage
    // /// ```move
    // /// let dk = ibe_config::get_decryption_key(timelock_id);
    // /// if (vector::length(&dk) == 48) {
    // ///     // Decryption key is available
    // /// }
    // /// ```
    // public fun get_decryption_key(timelock_id: u64): vector<u8> acquires TimelockRegistry {
    //     // TODO: Implementation
    //     // 1. Look up timelock in registry
    //     // 2. Return decryption_key if Some, else empty vector
    //     vector::empty()
    // }
    //
    // #[view]
    // /// Check if a timelock's decryption key has been revealed.
    // ///
    // /// Returns true if the deadline has passed AND threshold shares received.
    // public fun is_revealed(timelock_id: u64): bool acquires TimelockRegistry {
    //     // TODO: Implementation
    //     false
    // }

    // ================================
    // PHASE 3: DK Share Submission (STUBS)
    // ================================
    //
    // The following functions are stubs for Phase 3 implementation.
    // They handle validator share submissions after a timelock deadline passes.
    //
    // ## Overview
    //
    // After a timelock deadline passes:
    // 1. Each validator derives their DK share: dk_share = sk_share * H(identity)
    // 2. Validators submit shares via ValidatorTransaction
    // 3. Shares are aggregated using weighted Lagrange interpolation
    // 4. When threshold reached, DK is revealed on-chain
    //
    // ## Security
    //
    // - Shares are only accepted after deadline passes
    // - Each validator can only submit once per timelock
    // - Invalid shares are rejected (verified via pairing check)
    //
    // ## TODO
    //
    // - [ ] Implement submit_dk_share friend function
    // - [ ] Implement share validation
    // - [ ] Implement weighted Lagrange aggregation
    // - [ ] Add ValidatorTransaction type in types/src/validator_txn/

    // --------------------------------
    // Phase 3 Functions (TODO)
    // --------------------------------

    // TODO: Implement these functions
    //
    // /// Submit a decryption key share for a timelock.
    // ///
    // /// Called by validator transaction handler after deadline passes.
    // /// NOT callable by users directly.
    // ///
    // /// # Arguments
    // /// - `timelock_id`: The timelock being revealed
    // /// - `share`: The DK share (G1, 48 bytes) = sk_share * H(identity)
    // /// - `validator_index`: Index of the submitting validator
    // /// - `weight`: Validator's weight (stake)
    // ///
    // /// # Errors
    // /// - Aborts if deadline not passed
    // /// - Aborts if validator already submitted
    // /// - Aborts if share is invalid
    // ///
    // /// # Side Effects
    // /// If this share reaches threshold, aggregates and stores final DK.
    // public(friend) fun submit_dk_share(
    //     timelock_id: u64,
    //     share: vector<u8>,
    //     validator_index: u64,
    //     weight: u64
    // ) acquires TimelockRegistry {
    //     // TODO: Implementation
    //     // 1. Verify deadline has passed
    //     // 2. Verify validator hasn't submitted
    //     // 3. Validate share (pairing check)
    //     // 4. Add to accumulated shares
    //     // 5. If threshold reached, aggregate to final DK
    //     abort E_IBE_NOT_READY
    // }

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
