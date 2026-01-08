module aptos_framework::ibe_signature {
    use std::vector; 

    use aptos_std::bls12381_algebra::{G1, HashG1XmdSha256SswuRo};
    use aptos_std::crypto_algebra::{hash_to, Element};
    use aptos_framework::threshold_dsa;

    /// This module implements the Identity-Based Encryption (IBE) primitive semantics,
    /// specifically mapping Identities to Group Elements.
    ///
    /// # Reference
    ///
    /// *   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."
    ///     Section 4.1 "BasicIdent: A basic IBE system" -> **Extract** algorithm.
    ///
    /// # The Map-To-Point Function ($H_1$)
    ///
    /// [BF01] requires a hash function $H_1: \{0,1\}^* \to G_1^*$.
    /// We implement this using the IETF standard `hash_to_curve` (XMD:SHA-256_SSWU_RO).
    ///
    /// # Protocol Role
    ///
    /// This module connects the abstract concept of an "Identity" (e.g. a time interval)
    /// to the cryptographic verification logic in `threshold_dsa`.

    // DST for mapping identity to point.
    // This matches Rust's BLS_WVUF_DST = b"APTOS_BLS_WVUF_DST"
    const DST: vector<u8> = b"APTOS_BLS_WVUF_DST";

    // Message prefix for hash_to_curve, matching Rust's b"H(m)"
    const H_M_MSG: vector<u8> = b"H(m)";

    /// Map an Identity string to a $G_1$ group element.
    ///
    /// # Mathematical Definition
    ///
    /// $$ Q_{ID} = H_1(ID) \in G_1^* $$
    ///
    /// This corresponds to the first step of the **Extract** algorithm in [BF01].
    /// In our Timelock system, `identity_bytes` is the Keccak256 hash of the identity string.
    ///
    /// Note: This function prepends "H(m)" to the identity before hashing to curve,
    /// matching the Rust implementation's hash_to_curve(identity, DST, b"H(m)") signature.
    public fun identity_to_point(identity_bytes: vector<u8>): Element<G1> {
        // Prepend "H(m)" message to match Rust hash_to_curve(identity, DST, b"H(m)")
        // This is specific to the blstrs crate's hash_to_curve implementation
        let msg_with_h = vector::empty<u8>();
        vector::push_back(&mut msg_with_h, 72); // 'H'
        vector::push_back(&mut msg_with_h, 40); // '('
        vector::push_back(&mut msg_with_h, 109); // 'm'
        vector::push_back(&mut msg_with_h, 41); // ')'
        let i = 0;
        while (i < vector::length(&identity_bytes)) {
            vector::push_back(&mut msg_with_h, *vector::borrow(&identity_bytes, i));
            i = i + 1;
        };
        hash_to<G1, HashG1XmdSha256SswuRo>(&DST, &msg_with_h)
    }

    /// Verify that a given Private Key ($d_{ID}$) corresponds to the Identity ($ID$)
    /// under the Master Public Key ($P_{pub}$) for the given system ID.
    ///
    /// # Verification Logic
    ///
    /// This function verifies that the provided `private_key_bytes` constitute a valid IBE Private Key for the identity.
    ///
    /// $$ \text{Verify}(P_{pub}, ID, d_{ID}) \iff e(d_{ID}, g_2) = e(H_1(ID), P_{pub}) $$
    ///
    /// In the context of the Timelock Service:
    /// *   **Identity**: The time interval number (serialized).
    /// *   **Private Key**: The "decryption key" revealed by the validator set.
    /// *   **Verification**: Ensures that the revealed key is cryptographically valid and bound to the interval.
    ///
    /// This wraps `threshold_dsa::verify_signature_point`.
    public fun verify_private_key(mpk_id: u64, identity: vector<u8>, private_key_bytes: vector<u8>): bool {
        let point = identity_to_point(identity);
        threshold_dsa::verify_signature_point(mpk_id, point, private_key_bytes)
    }
}
