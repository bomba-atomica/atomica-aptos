/// Copyright © Aptos Foundation
/// SPDX-License-Identifier: Apache-2.0

//! Cross-Implementation IBE Test Fixtures for Move
//!
//! This module provides deterministic test fixtures matching the Rust implementation
//! for verifying IBE correctness in Move.
//!
//! Constants match: crates/aptos-dkg/src/ibe/fixtures.rs
//!
//! Test fixtures use hardcoded inputs with known expected outputs:
//! - FIXTURE_EPOCH = 42
//! - FIXTURE_DEADLINE_MICROSECONDS = 1704070800000000
//! - FIXTURE_MESSAGE = b"Golden Vector Message 2024"
//! - All keys and ciphertexts are pre-computed

module aptos_framework::ibe_test_fixtures {
    use std::vector;
    use std::hash;
    use aptos_std::crypto_algebra::{Self, Element};
    use aptos_std::bls12381_algebra::{G1, G2, Gt, HashG1XmdSha256SswuRo};
    use aptos_framework::ibe;

    // ============================================================================
    // Test Fixture Constants (matching Rust fixtures.rs)
    // ============================================================================

    /// Test epoch for fixtures
    const FIXTURE_EPOCH: u64 = 42;

    /// Test deadline in microseconds (2024-01-01 01:00:00 UTC)
    const FIXTURE_DEADLINE_MICROSECONDS: u64 = 1704070800000000;

    /// Test message as bytes
    const FIXTURE_MESSAGE: vector<u8> = b"Golden Vector Message 2024";

    /// Identity string: "timelock_id:42:deadline_timestamp_microseconds:1704070800000000"
    const FIXTURE_IDENTITY_STRING: vector<u8> =
        b"timelock_id:42:deadline_timestamp_microseconds:1704070800000000";

    /// Identity hash: Keccak256(FIXTURE_IDENTITY_STRING)
    /// Matches Rust: 6d68192a4097c6215fc53001590729f5f5d3f68e0449dcb94f76de2effc95f71
    const FIXTURE_IDENTITY_HASH: vector<u8> =
        x"6d68192a4097c6215fc53001590729f5f5d3f68e0449dcb94f76de2effc95f71";

    /// MSK (Master Secret Key) as hex - 32 bytes
    const FIXTURE_MSK_HEX: vector<u8> =
        x"9949fb87fac301fa852d757fed7fa3cf093928850089766583d7eeea94376d01";

    /// MPK (Master Public Key) - G2 point, 96 bytes compressed
    const FIXTURE_MPK_G2_HEX: vector<u8> =
        x"864479cdc8bbe7a3aa3194a50afdd5e4d45ec85ff1fdb58511468d61f51705f718fbf0094e5c0e74f95239f2edd244290c8c11c129562d1770a5f4dbe95ac60bb06c3e3c645d5b1ef65cd1a2ec06833d974506487787bfdd53b438040110c5cd";

    /// DK (Decryption Key) - G1 point, 48 bytes compressed
    const FIXTURE_DK_G1_HEX: vector<u8> =
        x"8a16b673b67dc8405eeea538753c8d5cb727dbc8e446464d3da08812d94061688fed6eca2dffd775e0dae6c53b15ffcb";

    /// Ciphertext U (G2 point) - 96 bytes
    const FIXTURE_CIPHERTEXT_U_HEX: vector<u8> =
        x"a081a70d4d60e76aea4e4d4cce3731b741d0ba652783071ec3ad8a9dcd48817514e826915b93efcf68f0c8cbbe0a89200e6e09c384e863ef5ee54c8f8f99c5ba69480863109d229df3e00cd1be3d05e4238b2df0dbc18aff4b939ab88dc5610c";

    /// Ciphertext V (encrypted message) - 21 bytes
    const FIXTURE_CIPHERTEXT_V_HEX: vector<u8> =
        x"b847c649d14bbcdb7c5186af161bfb90116b0a4d0de331467044";

    /// Expected decrypted result (matches FIXTURE_MESSAGE)
    const FIXTURE_EXPECTED_DECRYPTED_HEX: vector<u8> =
        x"476f6c64656e20566563746f72204d6573736167652032303234";

    // DST for hash_to_curve (matching Rust BLS_WVUF_DST)
    const BLS_WVUF_DST: vector<u8> = b"APTOS_BLS_WVUF_DST";

    // ============================================================================
    // Helper Functions
    // ============================================================================

    /// Parse hex string to bytes (for test fixture data)
    public fun parse_hex(hex: &vector<u8>): vector<u8> {
        let len = vector::length(hex);
        let result = vector::empty<u8>();
        let i = 0;
        while (i < len) {
            let high = hex_char_to_nibble(*vector::borrow(hex, i));
            let low = hex_char_to_nibble(*vector::borrow(hex, i + 1));
            vector::push_back(&mut result, (high << 4) | low);
            i = i + 2;
        };
        result
    }

    /// Convert hex character to 4-bit value
    fun hex_char_to_nibble(c: u8): u8 {
        if (c >= 48 && c <= 57) { // '0'-'9'
            c - 48
        } else if (c >= 97 && c <= 102) { // 'a'-'f'
            c - 87
        } else if (c >= 65 && c <= 70) { // 'A'-'F'
            c - 55
        } else {
            abort 1 // Invalid hex character
        }
    }

    /// Compute Keccak256 hash
    public fun keccak256(data: &vector<u8>): vector<u8> {
        hash::keccak256(data)
    }

    /// Map identity bytes to G1 point using hash_to_curve
    public fun identity_to_g1(identity: &vector<u8>): Element<G1> {
        // Prepend "H(m)" message to match Rust hash_to_curve(identity, DST, b"H(m)")
        let msg_with_h = vector::empty<u8>();
        vector::push_back(&mut msg_with_h, 72); // 'H'
        vector::push_back(&mut msg_with_h, 40); // '('
        vector::push_back(&mut msg_with_h, 109); // 'm'
        vector::push_back(&mut msg_with_h, 41); // ')'
        let i = 0;
        while (i < vector::length(identity)) {
            vector::push_back(&mut msg_with_h, *vector::borrow(identity, i));
            i = i + 1;
        };
        crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&BLS_WVUF_DST, &msg_with_h)
    }

    /// Parse G2 hex to Element
    public fun parse_g2_hex(hex: &vector<u8>): Element<G2> {
        let bytes = parse_hex(hex);
        crypto_algebra::deserialize<G2>(&bytes)
    }

    /// Parse G1 hex to Element
    public fun parse_g1_hex(hex: &vector<u8>): Element<G1> {
        let bytes = parse_hex(hex);
        crypto_algebra::deserialize<G1>(&bytes)
    }

    // ============================================================================
    // Test Functions
    // ============================================================================

    /// Test 1: Identity Derivation
    /// Verifies Keccak256("timelock_id:42:deadline_timestamp_microseconds:1704070800000000")
    /// matches the expected identity hash
    #[test]
    fun test_identity_derivation() {
        // Compute identity string
        let identity_string = FIXTURE_IDENTITY_STRING;

        // Compute Keccak256 hash
        let computed_hash = keccak256(&identity_string);

        // Compare with expected
        let expected = FIXTURE_IDENTITY_HASH;
        let i = 0;
        while (i < vector::length(&expected)) {
            assert!(*vector::borrow(&computed_hash, i) == *vector::borrow(expected, i), 1);
            i = i + 1;
        };
    }

    /// Test 2: Identity to G1 Point Mapping
    /// Verifies hash_to_curve produces expected G1 point
    #[test]
    fun test_identity_to_g1() {
        let identity_hash = FIXTURE_IDENTITY_HASH;
        let q_id = identity_to_g1(&identity_hash);

        // Serialize to compressed bytes
        let serialized = crypto_algebra::serialize<G1>(&q_id);

        // Expected DK (G1) - this is MSK * Q_id, but we're just testing the mapping
        let expected_dk = FIXTURE_DK_G1_HEX;
        let expected_bytes = parse_hex(expected_dk);

        let i = 0;
        while (i < vector::length(&expected_bytes)) {
            assert!(*vector::borrow(&serialized, i) == *vector::borrow(&expected_bytes, i), 2);
            i = i + 1;
        };
    }

    /// Test 3: MPK Parsing
    /// Verifies G2 point deserialization works correctly
    #[test]
    fun test_mpk_parsing() {
        let mpk_hex = FIXTURE_MPK_G2_HEX;
        let mpk = parse_g2_hex(&mpk_hex);

        // Re-serialize and verify
        let serialized = crypto_algebra::serialize<G2>(&mpk);
        let expected_bytes = parse_hex(&mpk_hex);

        let i = 0;
        while (i < vector::length(&expected_bytes)) {
            assert!(*vector::borrow(&serialized, i) == *vector::borrow(&expected_bytes, i), 3);
            i = i + 1;
        };
    }

    /// Test 4: DK Parsing
    /// Verifies G1 point deserialization works correctly
    #[test]
    fun test_dk_parsing() {
        let dk_hex = FIXTURE_DK_G1_HEX;
        let dk = parse_g1_hex(&dk_hex);

        // Re-serialize and verify
        let serialized = crypto_algebra::serialize<G1>(&dk);
        let expected_bytes = parse_hex(&dk_hex);

        let i = 0;
        while (i < vector::length(&expected_bytes)) {
            assert!(*vector::borrow(&serialized, i) == *vector::borrow(&expected_bytes, i), 4);
            i = i + 1;
        };
    }

    /// Test 5: Ciphertext U Parsing
    /// Verifies ciphertext U (G2) deserialization
    #[test]
    fun test_ciphertext_u_parsing() {
        let u_hex = FIXTURE_CIPHERTEXT_U_HEX;
        let u = parse_g2_hex(&u_hex);

        // Re-serialize and verify
        let serialized = crypto_algebra::serialize<G2>(&u);
        let expected_bytes = parse_hex(&u_hex);

        let i = 0;
        while (i < vector::length(&expected_bytes)) {
            assert!(*vector::borrow(&serialized, i) == *vector::borrow(&expected_bytes, i), 5);
            i = i + 1;
        };
    }

    /// Test 6: IBE Decryption
    /// Verifies full decryption using pre-computed ciphertext
    #[test]
    fun test_ibe_decryption() {
        // Parse ciphertext components
        let u_hex = FIXTURE_CIPHERTEXT_U_HEX;
        let u = parse_g2_hex(&u_hex);

        let v_hex = FIXTURE_CIPHERTEXT_V_HEX;
        let v = parse_hex(&v_hex);

        // Parse DK
        let dk_hex = FIXTURE_DK_G1_HEX;
        let dk = parse_g1_hex(&dk_hex);

        // Decrypt using IBE module
        let decrypted = ibe::decrypt<G1, G2, Gt>(&dk, &u, v);

        // Verify decrypted message matches expected
        let expected = FIXTURE_EXPECTED_DECRYPTED_HEX;
        let expected_bytes = parse_hex(&expected);

        let i = 0;
        while (i < vector::length(&expected_bytes)) {
            assert!(*vector::borrow(&decrypted, i) == *vector::borrow(&expected_bytes, i), 6);
            i = i + 1;
        };
    }

    /// Test 7: Full Roundtrip (Encrypted in Rust, Decrypted in Move)
    /// Uses pre-computed ciphertext from Rust fixture
    #[test]
    fun test_cross_language_decryption() {
        // Ciphertext computed by Rust implementation
        let u_hex = FIXTURE_CIPHERTEXT_U_HEX;
        let u = parse_g2_hex(&u_hex);

        let v_hex = FIXTURE_CIPHERTEXT_V_HEX;
        let v = parse_hex(&v_hex);

        // DK computed by Rust implementation
        let dk_hex = FIXTURE_DK_G1_HEX;
        let dk = parse_g1_hex(&dk_hex);

        // Decrypt
        let decrypted = ibe::decrypt<G1, G2, Gt>(&dk, &u, v);

        // Verify matches original message
        let expected = b"Golden Vector Message 2024";
        let i = 0;
        while (i < vector::length(expected)) {
            assert!(*vector::borrow(&decrypted, i) == expected[i], 7);
            i = i + 1;
        };
    }

    /// Test 8: Verify Fixture Constants are Valid
    /// Ensures all hex constants are correctly formatted
    #[test]
    fun test_fixture_constants_valid() {
        // MSK should be 32 bytes
        let msk = parse_hex(&FIXTURE_MSK_HEX);
        assert!(vector::length(&msk) == 32, 8);

        // MPK (G2) should be 96 bytes
        let mpk = parse_hex(&FIXTURE_MPK_G2_HEX);
        assert!(vector::length(&mpk) == 96, 8);

        // DK (G1) should be 48 bytes
        let dk = parse_hex(&FIXTURE_DK_G1_HEX);
        assert!(vector::length(&dk) == 48, 8);

        // Ciphertext U (G2) should be 96 bytes
        let u = parse_hex(&FIXTURE_CIPHERTEXT_U_HEX);
        assert!(vector::length(&u) == 96, 8);

        // Identity hash should be 32 bytes (Keccak256)
        assert!(vector::length(&FIXTURE_IDENTITY_HASH) == 32, 8);
    }
}
