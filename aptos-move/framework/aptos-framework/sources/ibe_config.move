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
