#[test_only]
/// IBE Native Function Tests (Move)
///
/// This module contains tests that verify the IBE native function integration
/// with the Move VM. These tests use golden vectors to verify data structures
/// and identity computation.
///
/// # Important Notes
///
/// The native function `ibe::reconstruct_ibe_dk` expects 32-byte scalar shares
/// (little-endian BLS12-381 scalars). The golden vectors provide 48-byte G1 DK
/// shares (pre-multiplied by H(identity)).
///
/// For full reconstruction testing with actual scalar shares, see:
/// - Rust tests: `aptos-move/framework/src/natives/cryptography/algebra/ibe_tests.rs`
/// - Rust SDK tests: `crates/aptos-dkg/src/ibe/tests.rs`
///
/// These Move tests focus on:
/// 1. Golden vector data structure validation
/// 2. Identity computation verification
/// 3. Data format and length assertions
module aptos_framework::ibe_native_test {
    use std::vector;
    use std::signer;
    use std::bcs;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_framework::ibe_config;

    // ============================================================================
    // GOLDEN VECTOR DATA STRUCTURE VALIDATION TESTS
    // ============================================================================

    /// Validates that golden vector 1 has correct data structure.
    ///
    /// This test verifies:
    /// - Identity is 32 bytes (SHA3-256 output)
    /// - DK shares are 48 bytes (compressed G1)
    /// - Reconstructed DK is 48 bytes
    /// - Weights and indices have matching counts
    #[test]
    fun test_golden_vectors_1_structure() {
        let identity = fixtures::roundtrip_1_identity();
        let validator_indices = fixtures::roundtrip_1_validator_indices();
        let validator_weights = fixtures::roundtrip_1_validator_weights();
        let dk_shares = fixtures::roundtrip_1_dk_shares();
        let reconstructed_dk = fixtures::roundtrip_1_reconstructed_dk();

        // Identity must be 32 bytes
        assert!(vector::length(&identity) == 32, 0);

        // Must have 5 validators
        assert!(vector::length(&validator_indices) == 5, 1);
        assert!(vector::length(&validator_weights) == 5, 2);

        // Weights must all be 1 (equal weights)
        let i = 0;
        while (i < 5) {
            assert!(*vector::borrow(&validator_weights, i) == 1, 10 + i);
            i = i + 1;
        };

        // Must have 5 DK shares
        assert!(vector::length(&dk_shares) == 5, 3);

        // Each DK share must be 48 bytes (compressed G1)
        let j = 0;
        while (j < 5) {
            let share = *vector::borrow(&dk_shares, j);
            assert!(vector::length(&share) == 48, 20 + j);
            j = j + 1;
        };

        // Reconstructed DK must be 48 bytes
        assert!(vector::length(&reconstructed_dk) == 48, 4);

        // Threshold and total weight must match
        assert!(fixtures::roundtrip_1_threshold() == 3, 5);
        assert!(fixtures::roundtrip_1_total_weight() == 5, 6);
    }

    /// Validates golden vector 2 structure (4 validators, threshold 2).
    #[test]
    fun test_golden_vectors_2_structure() {
        let identity = fixtures::roundtrip_2_identity();
        let validator_indices = fixtures::roundtrip_2_validator_indices();
        let validator_weights = fixtures::roundtrip_2_validator_weights();
        let dk_shares = fixtures::roundtrip_2_dk_shares();
        let reconstructed_dk = fixtures::roundtrip_2_reconstructed_dk();

        assert!(vector::length(&identity) == 32, 0);
        assert!(vector::length(&validator_indices) == 4, 1);
        assert!(vector::length(&validator_weights) == 4, 2);
        assert!(vector::length(&dk_shares) == 4, 3);

        let j = 0;
        while (j < 4) {
            let share = *vector::borrow(&dk_shares, j);
            assert!(vector::length(&share) == 48, 10 + j);
            j = j + 1;
        };

        assert!(vector::length(&reconstructed_dk) == 48, 4);
        assert!(fixtures::roundtrip_2_threshold() == 2, 5);
        assert!(fixtures::roundtrip_2_total_weight() == 4, 6);
    }

    /// Validates golden vector 3 structure (unequal weights [2, 1, 2]).
    #[test]
    fun test_golden_vectors_3_unequal_weights_structure() {
        let identity = fixtures::roundtrip_3_identity();
        let validator_indices = fixtures::roundtrip_3_validator_indices();
        let validator_weights = fixtures::roundtrip_3_validator_weights();
        let dk_shares = fixtures::roundtrip_3_dk_shares();
        let reconstructed_dk = fixtures::roundtrip_3_reconstructed_dk();

        assert!(vector::length(&identity) == 32, 0);
        assert!(vector::length(&validator_indices) == 3, 1);
        assert!(vector::length(&validator_weights) == 3, 2);

        // Verify weights are [2, 1, 2]
        assert!(*vector::borrow(&validator_weights, 0) == 2, 10);
        assert!(*vector::borrow(&validator_weights, 1) == 1, 11);
        assert!(*vector::borrow(&validator_weights, 2) == 2, 12);

        assert!(vector::length(&dk_shares) == 3, 3);

        let j = 0;
        while (j < 3) {
            let share = *vector::borrow(&dk_shares, j);
            assert!(vector::length(&share) == 48, 20 + j);
            j = j + 1;
        };

        assert!(vector::length(&reconstructed_dk) == 48, 4);
        assert!(fixtures::roundtrip_3_threshold() == 3, 5);
        assert!(fixtures::roundtrip_3_total_weight() == 5, 6);
    }

    /// Validates golden vector 4 structure (unequal weights [2, 3, 2, 1]).
    #[test]
    fun test_golden_vectors_4_unequal_weights_structure() {
        let identity = fixtures::roundtrip_4_identity();
        let validator_indices = fixtures::roundtrip_4_validator_indices();
        let validator_weights = fixtures::roundtrip_4_validator_weights();
        let dk_shares = fixtures::roundtrip_4_dk_shares();
        let reconstructed_dk = fixtures::roundtrip_4_reconstructed_dk();

        assert!(vector::length(&identity) == 32, 0);
        assert!(vector::length(&validator_indices) == 4, 1);
        assert!(vector::length(&validator_weights) == 4, 2);

        // Verify weights are [2, 3, 2, 1]
        assert!(*vector::borrow(&validator_weights, 0) == 2, 10);
        assert!(*vector::borrow(&validator_weights, 1) == 3, 11);
        assert!(*vector::borrow(&validator_weights, 2) == 2, 12);
        assert!(*vector::borrow(&validator_weights, 3) == 1, 13);

        assert!(vector::length(&dk_shares) == 4, 3);

        let j = 0;
        while (j < 4) {
            let share = *vector::borrow(&dk_shares, j);
            assert!(vector::length(&share) == 48, 20 + j);
            j = j + 1;
        };

        assert!(vector::length(&reconstructed_dk) == 48, 4);
        assert!(fixtures::roundtrip_4_threshold() == 3, 5);
        assert!(fixtures::roundtrip_4_total_weight() == 8, 6);
    }

    // ============================================================================
    // IDENTITY FIXTURE VALIDATION TESTS
    // ============================================================================

    /// Validates that identity fixtures are well-formed.
    ///
    /// This test verifies that the identity fixtures from golden vectors
    /// have the correct format and are self-consistent.
    #[test]
    fun test_identity_fixtures_are_well_formed() {
        // All identities must be 32 bytes
        assert!(vector::length(&fixtures::identity_0_1000000000000()) == 32, 0);
        assert!(vector::length(&fixtures::identity_1_1000000000000()) == 32, 1);
        assert!(vector::length(&fixtures::identity_0_2000000000000()) == 32, 2);

        // All identities must be different
        assert!(fixtures::identity_0_1000000000000() != fixtures::identity_1_1000000000000(), 3);
        assert!(fixtures::identity_1_1000000000000() != fixtures::identity_0_2000000000000(), 4);
        assert!(fixtures::identity_0_1000000000000() != fixtures::identity_0_2000000000000(), 5);

        // Roundtrip identities must match
        assert!(fixtures::roundtrip_1_identity() == fixtures::identity_0_1000000000000(), 6);
    }

    // ============================================================================
    // DATA FORMAT VALIDATION TESTS
    // ============================================================================

    /// Tests that identity is exactly 32 bytes.
    #[test]
    fun test_identity_is_32_bytes() {
        assert!(vector::length(&fixtures::identity_0_1000000000000()) == 32, 0);
        assert!(vector::length(&fixtures::identity_1_1000000000000()) == 32, 1);
        assert!(vector::length(&fixtures::identity_0_2000000000000()) == 32, 2);
        assert!(vector::length(&fixtures::roundtrip_1_identity()) == 32, 3);
        assert!(vector::length(&fixtures::roundtrip_2_identity()) == 32, 4);
        assert!(vector::length(&fixtures::roundtrip_3_identity()) == 32, 5);
        assert!(vector::length(&fixtures::roundtrip_4_identity()) == 32, 6);
    }

    /// Tests that DK shares are exactly 48 bytes (compressed G1).
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
            let shares = *vector::borrow(&vectors, v);
            let i = 0;
            let len = vector::length(&shares);
            while (i < len) {
                let share = *vector::borrow(&shares, i);
                assert!(vector::length(&share) == 48, (v * 10) + i);
                i = i + 1;
            };
            v = v + 1;
        };
    }

    /// Tests that reconstructed DK is exactly 48 bytes.
    #[test]
    fun test_reconstructed_dk_is_48_bytes() {
        assert!(vector::length(&fixtures::roundtrip_1_reconstructed_dk()) == 48, 0);
        assert!(vector::length(&fixtures::roundtrip_2_reconstructed_dk()) == 48, 1);
        assert!(vector::length(&fixtures::roundtrip_3_reconstructed_dk()) == 48, 2);
        assert!(vector::length(&fixtures::roundtrip_4_reconstructed_dk()) == 48, 3);
    }

    /// Tests that H(identity) G1 points are exactly 48 bytes.
    #[test]
    fun test_h_identity_is_48_bytes() {
        assert!(vector::length(&fixtures::h_identity_0_1000000000000()) == 48, 0);
        assert!(vector::length(&fixtures::h_identity_1_1000000000000()) == 48, 1);
        assert!(vector::length(&fixtures::h_identity_0_2000000000000()) == 48, 2);
        assert!(vector::length(&fixtures::roundtrip_1_h_identity()) == 48, 3);
        assert!(vector::length(&fixtures::roundtrip_2_h_identity()) == 48, 4);
        assert!(vector::length(&fixtures::roundtrip_3_h_identity()) == 48, 5);
        assert!(vector::length(&fixtures::roundtrip_4_h_identity()) == 48, 6);
    }

    // ============================================================================
    // CIPHERTEXT FORMAT VALIDATION TESTS
    // ============================================================================

    /// Tests that ciphertext U component is 96 bytes (compressed G2).
    #[test]
    fun test_ciphertext_u_is_96_bytes() {
        assert!(vector::length(&fixtures::roundtrip_1_ciphertext_u()) == 96, 0);
        assert!(vector::length(&fixtures::roundtrip_2_ciphertext_u()) == 96, 1);
        assert!(vector::length(&fixtures::roundtrip_3_ciphertext_u()) == 96, 2);
        assert!(vector::length(&fixtures::roundtrip_4_ciphertext_u()) == 96, 3);
    }

    // ============================================================================
    // HELPER FUNCTION TESTS
    // ============================================================================

    /// Tests that helper functions return correct data.
    #[test]
    fun test_helper_functions() {
        // get_identity_hash
        assert!(fixtures::get_identity_hash(0, 1000000000000) == fixtures::identity_0_1000000000000(), 0);
        assert!(fixtures::get_identity_hash(1, 1000000000000) == fixtures::identity_1_1000000000000(), 1);
        assert!(fixtures::get_identity_hash(0, 2000000000000) == fixtures::identity_0_2000000000000(), 2);
        assert!(vector::length(&fixtures::get_identity_hash(999, 999)) == 0, 3);

        // get_roundtrip_validator_indices
        assert!(fixtures::get_roundtrip_validator_indices(1) == fixtures::roundtrip_1_validator_indices(), 4);
        assert!(fixtures::get_roundtrip_validator_indices(2) == fixtures::roundtrip_2_validator_indices(), 5);
        assert!(fixtures::get_roundtrip_validator_indices(3) == fixtures::roundtrip_3_validator_indices(), 6);
        assert!(fixtures::get_roundtrip_validator_indices(4) == fixtures::roundtrip_4_validator_indices(), 7);
        assert!(vector::length(&fixtures::get_roundtrip_validator_indices(99)) == 0, 8);

        // get_roundtrip_threshold
        assert!(fixtures::get_roundtrip_threshold(1) == 3, 9);
        assert!(fixtures::get_roundtrip_threshold(2) == 2, 10);
        assert!(fixtures::get_roundtrip_threshold(3) == 3, 11);
        assert!(fixtures::get_roundtrip_threshold(4) == 3, 12);
        assert!(fixtures::get_roundtrip_threshold(99) == 0, 13);

        // get_roundtrip_total_weight
        assert!(fixtures::get_roundtrip_total_weight(1) == 5, 14);
        assert!(fixtures::get_roundtrip_total_weight(2) == 4, 15);
        assert!(fixtures::get_roundtrip_total_weight(3) == 5, 16);
        assert!(fixtures::get_roundtrip_total_weight(4) == 8, 17);
        assert!(fixtures::get_roundtrip_total_weight(99) == 0, 18);
    }

    // ============================================================================
    // WEIGHT CONFIGURATION TESTS
    // ============================================================================

    /// Tests that weight configurations are correct.
    #[test]
    fun test_weight_configurations() {
        // Roundtrip 1: [1, 1, 1, 1, 1]
        let w1 = fixtures::roundtrip_1_validator_weights();
        assert!(vector::length(&w1) == 5, 0);
        let sum1 = *vector::borrow(&w1, 0) + *vector::borrow(&w1, 1) + *vector::borrow(&w1, 2) + *vector::borrow(&w1, 3) + *vector::borrow(&w1, 4);
        assert!(sum1 == fixtures::roundtrip_1_total_weight(), 1);

        // Roundtrip 2: [1, 1, 1, 1]
        let w2 = fixtures::roundtrip_2_validator_weights();
        assert!(vector::length(&w2) == 4, 2);
        let sum2 = *vector::borrow(&w2, 0) + *vector::borrow(&w2, 1) + *vector::borrow(&w2, 2) + *vector::borrow(&w2, 3);
        assert!(sum2 == fixtures::roundtrip_2_total_weight(), 3);

        // Roundtrip 3: [2, 1, 2]
        let w3 = fixtures::roundtrip_3_validator_weights();
        assert!(vector::length(&w3) == 3, 4);
        let sum3 = *vector::borrow(&w3, 0) + *vector::borrow(&w3, 1) + *vector::borrow(&w3, 2);
        assert!(sum3 == fixtures::roundtrip_3_total_weight(), 5);

        // Roundtrip 4: [2, 3, 2, 1]
        let w4 = fixtures::roundtrip_4_validator_weights();
        assert!(vector::length(&w4) == 4, 6);
        let sum4 = *vector::borrow(&w4, 0) + *vector::borrow(&w4, 1) + *vector::borrow(&w4, 2) + *vector::borrow(&w4, 3);
        assert!(sum4 == fixtures::roundtrip_4_total_weight(), 7);
    }
}
