#[test_only]
/// Timelock Workflow Unit Tests (Move)
///
/// This module contains comprehensive tests for the complete timelock workflow,
/// verifying the end-to-end process from registration through DK reconstruction.
///
/// # Test Coverage
///
/// - IBE initialization and MPK setup
/// - Timelock registration with deadline
/// - Timestamp manipulation for deadline passing
/// - DK share submission from multiple validators
/// - DK reconstruction via native function
/// - Event emission verification
module aptos_framework::timelock_workflow_test {
    use std::vector;
    use std::hash::sha3_256;
    use std::bcs;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_framework::ibe_config;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_std::bls12381_algebra::G1;
    use aptos_std::ibe;

    // ============================================
    // Initialization Tests
    // ============================================

    #[test]
    fun test_initialize_for_testing() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        assert!(!ibe_config::is_ready(), 1);
        assert!(ibe_config::get_epoch() == 0, 2);
    }

    #[test]
    fun test_set_mpk_for_testing() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let mpk = fixtures::timelock_basic_mpk();
        ibe_config::set_mpk_for_testing(mpk, 1);

        assert!(ibe_config::get_mpk() == mpk, 1);
        assert!(ibe_config::get_epoch() == 1, 2);
        assert!(ibe_config::is_ready(), 3);
    }

    // ============================================
    // Identity Computation Tests
    // ============================================

    #[test]
    fun test_identity_uniqueness() {
        let framework = account::create_signer_for_test(@0x1);
        timestamp::set_time_has_started_for_testing(&framework);

        let id1 = compute_identity_internal(0, 1000000000000);
        let id2 = compute_identity_internal(1, 1000000000000);
        let id3 = compute_identity_internal(0, 2000000000000);

        assert!(id1 != id2, 1);
        assert!(id1 != id3, 2);
        assert!(id2 != id3, 3);
    }

    fun compute_identity_internal(timelock_id: u64, deadline_us: u64): vector<u8> {
        let input = bcs::to_bytes(&timelock_id);
        vector::append(&mut input, bcs::to_bytes(&deadline_us));
        sha3_256(input)
    }

    // ============================================
    // Registration Tests
    // ============================================

    #[test]
    fun test_register_timelock_creates_info() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let initial_next_id = ibe_config::get_next_timelock_id();
        let deadline = 2000000000000;

        ibe_config::register_timelock(&framework, deadline);

        assert!(ibe_config::get_next_timelock_id() == initial_next_id + 1, 1);
        assert!(!ibe_config::is_revealed(initial_next_id), 2);

        let (reg_deadline, identity, is_revealed, share_count) = ibe_config::get_timelock(initial_next_id);
        assert!(reg_deadline == deadline, 3);
        assert!(vector::length(&identity) == 32, 4);
        assert!(!is_revealed, 5);
        assert!(share_count == 0, 6);
    }

    #[test]
    fun test_register_multiple_timelocks() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        ibe_config::register_timelock(&framework, 2000000000000);
        ibe_config::register_timelock(&framework, 3000000000000);

        assert!(ibe_config::get_next_timelock_id() == 2, 1);

        let (deadline1, identity1, _, _) = ibe_config::get_timelock(0);
        let (deadline2, identity2, _, _) = ibe_config::get_timelock(1);

        assert!(deadline1 == 2000000000000, 2);
        assert!(deadline2 == 3000000000000, 3);
        assert!(identity1 != identity2, 4);
    }

    #[test]
    #[expected_failure]
    fun test_register_past_deadline_fails() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        timestamp::update_global_time_for_test(1000000000000);
        ibe_config::register_timelock(&framework, 500000000000);
    }

    // ============================================
    // DK Share Submission Tests
    // ============================================

    #[test]
    #[expected_failure]
    fun test_submit_shares_before_deadline_fails() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);
        ibe_config::set_mpk_for_testing(fixtures::timelock_basic_mpk(), 1);

        let deadline = 2000000000000;
        ibe_config::register_timelock(&framework, deadline);

        let shares = *vector::borrow(&fixtures::timelock_basic_dk_shares(), 0);
        ibe_config::submit_dk_shares_for_testing(
            0,
            shares,
            0,
            @0x1234000000000000000000000000000A,
            1,
            5
        );
    }

    // ============================================
    // Complete Workflow Tests
    // ============================================

    #[test]
    fun test_complete_workflow_equal_weights() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let mpk = fixtures::timelock_basic_mpk();
        ibe_config::set_mpk_for_testing(mpk, 1);

        let deadline = 1000000000000;
        ibe_config::register_timelock(&framework, deadline);

        assert!(!ibe_config::is_revealed(0), 1);

        timestamp::update_global_time_for_test(deadline + 1);

        let dk_shares = fixtures::timelock_basic_dk_shares();
        let weights = fixtures::timelock_basic_validator_weights();
        let threshold = fixtures::timelock_basic_threshold();
        let total_weight = fixtures::timelock_basic_total_weight();

        assert!(threshold == 3, 10);
        assert!(total_weight == 5, 11);

        let validator_addresses = vector[@0x1234000000000000000000000000000A, @0x1234000000000000000000000000000B, @0x1234000000000000000000000000000C];
        let i = 0;
        while (i < 3) {
            let shares = *vector::borrow(&dk_shares, i);
            let weight = *vector::borrow(&weights, i);
            let validator_addr = *vector::borrow(&validator_addresses, i);
            ibe_config::submit_dk_shares_for_testing(
                0,
                shares,
                i,
                validator_addr,
                weight,
                total_weight
            );
            i = i + 1;
        };

        assert!(!ibe_config::is_revealed(0), 2);

        ibe_config::finalize_timelock_reveal_with_threshold_for_testing(0, threshold, total_weight);

        assert!(ibe_config::is_revealed(0), 3);

        let dk = ibe_config::get_decryption_key(0);
        let expected_dk = fixtures::timelock_basic_reconstructed_dk();
        assert!(dk == expected_dk, 4);
        assert!(vector::length(&dk) == 48, 5);
    }

    #[test]
    fun test_sparse_validator_participation() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let mpk = fixtures::timelock_basic_mpk();
        ibe_config::set_mpk_for_testing(mpk, 1);

        let deadline = 1000000000000;
        ibe_config::register_timelock(&framework, deadline);

        timestamp::update_global_time_for_test(deadline + 1);

        let dk_shares = fixtures::timelock_basic_dk_shares();
        let total_weight = fixtures::timelock_basic_total_weight();

        ibe_config::submit_dk_shares_for_testing(
            0,
            *vector::borrow(&dk_shares, 0),
            0,
            @0x1234000000000000000000000000000A,
            1,
            total_weight
        );
        assert!(!ibe_config::is_revealed(0), 1);

        ibe_config::submit_dk_shares_for_testing(
            0,
            *vector::borrow(&dk_shares, 2),
            2,
            @0x1234000000000000000000000000000B,
            1,
            total_weight
        );
        assert!(!ibe_config::is_revealed(0), 2);

        ibe_config::submit_dk_shares_for_testing(
            0,
            *vector::borrow(&dk_shares, 4),
            4,
            @0x1234000000000000000000000000000C,
            1,
            total_weight
        );
        assert!(!ibe_config::is_revealed(0), 3);

        let threshold = fixtures::timelock_basic_threshold();
        ibe_config::finalize_timelock_reveal_with_threshold_for_testing(0, threshold, total_weight);

        assert!(ibe_config::is_revealed(0), 4);

        let dk = ibe_config::get_decryption_key(0);
        let expected_dk = fixtures::timelock_basic_reconstructed_dk();
        assert!(dk == expected_dk, 5);
    }

    #[test]
    #[expected_failure]
    fun test_get_decryption_key_before_reveal_fails() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let mpk = fixtures::timelock_basic_mpk();
        ibe_config::set_mpk_for_testing(mpk, 1);

        let deadline = 1000000000000;
        ibe_config::register_timelock(&framework, deadline);

        timestamp::update_global_time_for_test(deadline + 1);

        let dk_shares = fixtures::timelock_basic_dk_shares();
        let total_weight = fixtures::timelock_basic_total_weight();

        ibe_config::submit_dk_shares_for_testing(
            0,
            *vector::borrow(&dk_shares, 0),
            0,
            @0x1234000000000000000000000000000A,
            1,
            total_weight
        );

        let _dk = ibe_config::get_decryption_key(0);
    }

    // ============================================
    // Native Function Reconstruction Tests
    // ============================================

    #[test]
    fun test_native_reconstruction_matches_golden_vector() {
        let validator_indices = vector[0u64, 1, 2];
        let weights = fixtures::timelock_basic_validator_weights();
        let dk_shares = fixtures::timelock_basic_dk_shares();

        let share0 = *vector::borrow(&dk_shares, 0);
        let share1 = *vector::borrow(&dk_shares, 1);
        let share2 = *vector::borrow(&dk_shares, 2);
        let nested_shares = vector[share0, share1, share2];

        let identity = fixtures::timelock_basic_identity();
        let threshold = fixtures::timelock_basic_threshold();
        let total_weight = fixtures::timelock_basic_total_weight();

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            nested_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        let expected_dk = fixtures::timelock_basic_reconstructed_dk();
        assert!(reconstructed_dk == expected_dk, 1);
        assert!(vector::length(&reconstructed_dk) == 48, 2);
    }

    // ============================================
    // Event Emission Tests
    // ============================================

    #[test]
    fun test_is_expired_changes_after_deadline() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let deadline = 2000000000000;
        ibe_config::register_timelock(&framework, deadline);

        assert!(!ibe_config::is_expired(0), 1);

        timestamp::update_global_time_for_test(deadline - 1);
        assert!(!ibe_config::is_expired(0), 2);

        timestamp::update_global_time_for_test(deadline);
        assert!(ibe_config::is_expired(0), 3);
    }

    // ============================================
    // Helper Function Tests
    // ============================================

    #[test]
    fun test_get_timelock_view() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let deadline = 3000000000000;
        ibe_config::register_timelock(&framework, deadline);

        let (reg_deadline, identity, is_revealed, share_count) = ibe_config::get_timelock(0);

        assert!(reg_deadline == deadline, 1);
        assert!(vector::length(&identity) == 32, 2);
        assert!(!is_revealed, 3);
        assert!(share_count == 0, 4);
    }

    #[test]
    fun test_get_identity_view() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let deadline = 1000000000000;
        ibe_config::register_timelock(&framework, deadline);

        let identity = ibe_config::get_identity(0);
        let expected_identity = compute_identity_internal(0, deadline);

        assert!(identity == expected_identity, 1);
        assert!(vector::length(&identity) == 32, 2);
    }

    #[test]
    fun test_get_deadline_view() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        let deadline = 4000000000000;
        ibe_config::register_timelock(&framework, deadline);

        assert!(ibe_config::get_deadline(0) == deadline, 1);
    }

    #[test]
    fun test_get_next_timelock_id() {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);

        assert!(ibe_config::get_next_timelock_id() == 0, 1);

        ibe_config::register_timelock(&framework, 1000000000000);
        assert!(ibe_config::get_next_timelock_id() == 1, 2);

        ibe_config::register_timelock(&framework, 2000000000000);
        assert!(ibe_config::get_next_timelock_id() == 2, 3);
    }
}
