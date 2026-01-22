#[test_only]
/// IBE Native Function Paranoid Tests (Move)
///
/// This module contains comprehensive tests that verify the correctness of the IBE
/// native function implementation using golden vectors.
///
/// # Test Philosophy (From Audit Document)
///
/// These tests verify the G1-based reconstruction path:
/// 1. Validators submit 48-byte G1 DK shares (one per virtual player)
/// 2. Native function performs weighted Lagrange interpolation on G1 points
/// 3. Result matches the expected reconstructed DK from golden vectors
module aptos_framework::ibe_native_test {
    use std::vector;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_std::bls12381_algebra::G1;
    use aptos_std::ibe;

    // ============================================================================
    // NATIVE FUNCTION RECONSTRUCTION TESTS (PARANOID)
    // ============================================================================

    /// Tests DK reconstruction with roundtrip vector 1 (5 validators, equal weights).
    #[test]
    fun test_native_reconstruction_5_validators_equal_weights() {
        // Load test data from golden vectors
        let validator_indices = fixtures::roundtrip_1_validator_indices();
        let weights = fixtures::roundtrip_1_validator_weights();
        let dk_shares = fixtures::roundtrip_1_dk_shares();
        let identity = fixtures::roundtrip_1_identity();
        let threshold = fixtures::roundtrip_1_threshold();
        let total_weight = fixtures::roundtrip_1_total_weight();
        let expected_dk = fixtures::roundtrip_1_reconstructed_dk();

        // Verify inputs are correct format before calling native function
        assert!(vector::length(&validator_indices) == 5, 1);
        assert!(vector::length(&weights) == 5, 2);
        assert!(vector::length(&dk_shares) == 5, 3);

        // Call the native function with nested G1 DK shares
        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        // Verify the reconstructed DK matches expected value
        assert!(reconstructed_dk == expected_dk, 0);

        // Verify result is 48 bytes (compressed G1)
        assert!(vector::length(&reconstructed_dk) == 48, 100);
    }

    /// Tests DK reconstruction with roundtrip vector 2 (4 validators, threshold 2).
    #[test]
    fun test_native_reconstruction_4_validators_threshold_2() {
        let validator_indices = fixtures::roundtrip_2_validator_indices();
        let weights = fixtures::roundtrip_2_validator_weights();
        let dk_shares = fixtures::roundtrip_2_dk_shares();
        let identity = fixtures::roundtrip_2_identity();
        let threshold = fixtures::roundtrip_2_threshold();
        let total_weight = fixtures::roundtrip_2_total_weight();
        let expected_dk = fixtures::roundtrip_2_reconstructed_dk();

        // Verify format
        assert!(vector::length(&dk_shares) == 4, 3);

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        assert!(reconstructed_dk == expected_dk, 0);
        assert!(vector::length(&reconstructed_dk) == 48, 100);
    }

    /// Tests DK reconstruction with unequal weights [2, 1, 2].
    #[test]
    fun test_native_reconstruction_unequal_weights_215() {
        let validator_indices = fixtures::roundtrip_3_validator_indices();
        let weights = fixtures::roundtrip_3_validator_weights();
        let dk_shares = fixtures::roundtrip_3_dk_shares();
        let identity = fixtures::roundtrip_3_identity();
        let threshold = fixtures::roundtrip_3_threshold();
        let total_weight = fixtures::roundtrip_3_total_weight();
        let expected_dk = fixtures::roundtrip_3_reconstructed_dk();

        // Verify weights are [2, 1, 2]
        assert!(*vector::borrow(&weights, 0) == 2, 1);
        assert!(*vector::borrow(&weights, 1) == 1, 2);
        assert!(*vector::borrow(&weights, 2) == 2, 3);

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        assert!(reconstructed_dk == expected_dk, 0);
        assert!(vector::length(&reconstructed_dk) == 48, 100);
    }

    /// Tests DK reconstruction with unequal weights [2, 3, 2, 1].
    #[test]
    fun test_native_reconstruction_unequal_weights_2321() {
        let validator_indices = fixtures::roundtrip_4_validator_indices();
        let weights = fixtures::roundtrip_4_validator_weights();
        let dk_shares = fixtures::roundtrip_4_dk_shares();
        let identity = fixtures::roundtrip_4_identity();
        let threshold = fixtures::roundtrip_4_threshold();
        let total_weight = fixtures::roundtrip_4_total_weight();
        let expected_dk = fixtures::roundtrip_4_reconstructed_dk();

        // Verify weights are [2, 3, 2, 1]
        assert!(*vector::borrow(&weights, 0) == 2, 1);
        assert!(*vector::borrow(&weights, 1) == 3, 2);
        assert!(*vector::borrow(&weights, 2) == 2, 3);
        assert!(*vector::borrow(&weights, 3) == 1, 4);

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        assert!(reconstructed_dk == expected_dk, 0);
        assert!(vector::length(&reconstructed_dk) == 48, 100);
    }

    /// Tests that different identities produce different DKs from the same shares.
    #[test]
    fun test_different_identities_produce_different_dks() {
        // Get shares from roundtrip 1
        let indices1 = fixtures::roundtrip_1_validator_indices();
        let weights1 = fixtures::roundtrip_1_validator_weights();
        let shares1 = fixtures::roundtrip_1_dk_shares();
        let total1 = fixtures::roundtrip_1_total_weight();

        // Use identity from roundtrip 1
        let identity1 = fixtures::roundtrip_1_identity();
        let dk1 = ibe::reconstruct_ibe_dk<G1>(
            indices1,
            shares1,
            weights1,
            3,
            total1,
            identity1
        );

        // Use identity from roundtrip 2 (different)
        let identity2 = fixtures::roundtrip_2_identity();
        let shares2 = fixtures::roundtrip_2_dk_shares();
        let indices2 = fixtures::roundtrip_2_validator_indices();
        let weights2 = fixtures::roundtrip_2_validator_weights();
        let total2 = fixtures::roundtrip_2_total_weight();
        let dk2 = ibe::reconstruct_ibe_dk<G1>(
            indices2,
            shares2,
            weights2,
            2,
            total2,
            identity2
        );

        // Different identities should produce different DKs
        assert!(dk1 != dk2, 0);
    }

    // ============================================================================
    // ERROR HANDLING TESTS
    // ============================================================================

    /// Tests that empty shares returns an error.
    #[test]
    #[expected_failure]
    fun test_empty_shares_should_abort() {
        let validator_indices = vector<u64>[];
        let dk_shares = vector<vector<vector<u8>>>[];
        let weights = vector[1, 1, 1];
        let identity = fixtures::identity_0_1000000000000();

        let _dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            1,
            3,
            identity
        );
    }

    /// Tests that mismatched indices and shares counts returns an error.
    #[test]
    #[expected_failure]
    fun test_mismatched_indices_and_shares_should_abort() {
        let validator_indices = vector[0, 1, 2];  // 3 indices
        let validator1_shares = vector[x"010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"];
        let validator2_shares = vector[x"020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"];
        let dk_shares = vector[validator1_shares, validator2_shares];   // Only 2 validators
        let weights = vector[1, 1, 1];
        let identity = fixtures::identity_0_1000000000000();

        let _dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            dk_shares,
            weights,
            2,
            3,
            identity
        );
    }

    // ============================================================================
    // SPARSE PARTICIPATION TESTS
    // ============================================================================

    /// Tests reconstruction with only subset of validators (meets threshold).
    #[test]
    fun test_sparse_validator_participation() {
        let validator_indices = vector[0, 2, 4];  // Sparse indices
        let weights = fixtures::roundtrip_1_validator_weights();
        let dk_shares = fixtures::roundtrip_1_dk_shares();

        // Get shares for validators 0, 2, 4
        let share_0 = *vector::borrow(&dk_shares, 0);
        let share_2 = *vector::borrow(&dk_shares, 2);
        let share_4 = *vector::borrow(&dk_shares, 4);
        let sparse_shares = vector[share_0, share_2, share_4];

        let identity = fixtures::roundtrip_1_identity();
        let threshold = fixtures::roundtrip_1_threshold();
        let total_weight = fixtures::roundtrip_1_total_weight();

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            validator_indices,
            sparse_shares,
            weights,
            threshold,
            total_weight,
            identity
        );

        assert!(vector::length(&reconstructed_dk) == 48, 0);
    }

    // ============================================================================
    // DATA STRUCTURE VALIDATION TESTS
    // ============================================================================

    /// Tests that DK shares are 48 bytes (compressed G1).
    #[test]
    fun test_dk_shares_are_48_bytes() {
        let vectors = vector[
            fixtures::roundtrip_1_dk_shares(),
            fixtures::roundtrip_2_dk_shares(),
            fixtures::roundtrip_3_dk_shares(),
            fixtures::roundtrip_4_dk_shares()
        ];

        let v = 0;
        while (v < 4) {
            let validator_group = *vector::borrow(&vectors, v);
            let i = 0;
            let num_validators = vector::length(&validator_group);
            while (i < num_validators) {
                let validator_shares = *vector::borrow(&validator_group, i);
                let j = 0;
                let num_shares = vector::length(&validator_shares);
                while (j < num_shares) {
                    let share = *vector::borrow(&validator_shares, j);
                    assert!(vector::length(&share) == 48, (v * 100) + (i * 10) + j);
                    j = j + 1;
                };
                i = i + 1;
            };
            v = v + 1;
        };
    }

    /// Tests that reconstructed DK is 48 bytes.
    #[test]
    fun test_reconstructed_dk_is_48_bytes() {
        assert!(vector::length(&fixtures::roundtrip_1_reconstructed_dk()) == 48, 0);
        assert!(vector::length(&fixtures::roundtrip_2_reconstructed_dk()) == 48, 1);
        assert!(vector::length(&fixtures::roundtrip_3_reconstructed_dk()) == 48, 2);
        assert!(vector::length(&fixtures::roundtrip_4_reconstructed_dk()) == 48, 3);
    }

    /// Tests weight configurations match total weight.
    #[test]
    fun test_weight_configurations() {
        // Roundtrip 1: [1, 1, 1, 1, 1] = 5
        let w1 = fixtures::roundtrip_1_validator_weights();
        let sum1 = *vector::borrow(&w1, 0) + *vector::borrow(&w1, 1) + *vector::borrow(&w1, 2) + *vector::borrow(&w1, 3) + *vector::borrow(&w1, 4);
        assert!(sum1 == fixtures::roundtrip_1_total_weight(), 1);

        // Roundtrip 3: [2, 1, 2] = 5
        let w3 = fixtures::roundtrip_3_validator_weights();
        let sum3 = *vector::borrow(&w3, 0) + *vector::borrow(&w3, 1) + *vector::borrow(&w3, 2);
        assert!(sum3 == fixtures::roundtrip_3_total_weight(), 5);

        // Roundtrip 4: [2, 3, 2, 1] = 8
        let w4 = fixtures::roundtrip_4_validator_weights();
        let sum4 = *vector::borrow(&w4, 0) + *vector::borrow(&w4, 1) + *vector::borrow(&w4, 2) + *vector::borrow(&w4, 3);
        assert!(sum4 == fixtures::roundtrip_4_total_weight(), 8);
    }
}
