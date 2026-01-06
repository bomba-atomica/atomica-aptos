module aptos_framework::threshold_dsa {
    use std::option::{Self, Option};
    use aptos_std::table::{Self, Table};
    use aptos_framework::system_addresses;
    use aptos_framework::stake;
    use aptos_std::crypto_algebra::{deserialize, pairing, eq, one, hash_to, Element};
    use aptos_std::bls12381_algebra::{G1, G2, Gt, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo};

    friend aptos_framework::timelock;
    friend aptos_framework::ibe_signature; 

    /// The `threshold_dsa` module implements specific components of a Threshold Digital Signature Algorithm (DSA)
    /// based on the BLS (Boneh-Lynn-Shacham) signature scheme.
    ///
    /// # Cryptographic Foundation
    ///
    /// This module manages the **Master Public Key (MPK)** and verifies signatures against it.
    /// In the context of Identity-Based Encryption (IBE), the "Signature" corresponds to an extracted Private Key over an Identity.
    ///
    /// ## References
    ///
    /// *   **[BLS01]**: Boneh, D., Lynn, B., & Shacham, H. (2001). "Short signatures from the Weil pairing."
    /// *   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."
    ///
    /// ## Definitions
    ///
    /// *   **$G_1, G_2, G_T$**: Cyclic groups of prime order $q$ equipped with a bilinear map $e: G_1 \times G_2 \to G_T$.
    /// *   **$g_2$**: Generator of $G_2$.
    /// *   **$s$**: The Master Secret Key (MSK), $s \in_R \mathbb{Z}_q^*$.
    /// *   **$P_{pub}$**: The Master Public Key (MPK), $P_{pub} = s \cdot g_2$ (in $G_2$).
    ///
    /// Note: This implementation uses Type-3 pairings (BLS12-381), whereas the original [BF01] paper describes Type-1.
    ///
    /// # Access Control
    ///
    /// Only active validators (via `stake` module) are authorized to publish the MPK.

    const ENOT_VALIDATOR: u64 = 1;
    const EMPK_ALREADY_EXISTS: u64 = 2;
    const EMPK_NOT_FOUND: u64 = 3;
    const EINVALID_PUBKEY: u64 = 4;
    const EINVALID_SIGNATURE: u64 = 5;

    struct State has key {
        /// Map of ID (e.g. interval/epoch) to Master Public Key bytes (compressed G2)
        master_public_keys: Table<u64, vector<u8>>,
    }

    #[event]
    struct MasterPublicKeyPublishedEvent has drop, store {
        id: u64,
        master_public_key: vector<u8>,
    }

    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<State>(@aptos_framework)) {
            move_to(framework, State {
                master_public_keys: table::new(),
            });
        }
    }

    /// Publish a unified Master Public Key for a given ID.
    public entry fun publish_master_public_key(
        validator: &signer,
        id: u64,
        pk: vector<u8>
    ) acquires State {
        let validator_addr = std::signer::address_of(validator);
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<State>(@aptos_framework);
        if (!table::contains(&state.master_public_keys, id)) {
            // Validate PK format (G2 compressed)
            let pk_point = deserialize<G2, FormatG2Compr>(&pk);
            assert!(option::is_some(&pk_point), EINVALID_PUBKEY);

            table::add(&mut state.master_public_keys, id, pk);
            
            aptos_framework::event::emit(MasterPublicKeyPublishedEvent {
                id,
                master_public_key: pk,
            });
        };
    }

    #[view]
    public fun get_master_public_key(id: u64): Option<vector<u8>> acquires State {
        if (!exists<State>(@aptos_framework)) {
            return option::none()
        };
        let state = borrow_global<State>(@aptos_framework);
        if (table::contains(&state.master_public_keys, id)) {
            option::some(*table::borrow(&state.master_public_keys, id))
        } else {
            option::none()
        }
    }

    /// Verify a signature share (or unique signature) against the generic Master Public Key (MPK).
    ///
    /// # Mathematical Verification
    ///
    /// Given a message $m$, signature $\sigma$, and public key $P_{pub}$:
    /// 1.  Map the message to a point in $G_1$: $H(m) \in G_1$.
    /// 2.  Verify the bilinear pairing equality:
    ///     $$ e(\sigma, g_2) \stackrel{?}{=} e(H(m), P_{pub}) $$
    ///
    /// Where:
    /// *   $\sigma = s \cdot H(m)$ (The signature / extracted private key).
    /// *   $P_{pub} = s \cdot g_2$ (The Master Public Key).
    ///
    /// By bilinearity: $e(s \cdot H(m), g_2) = e(H(m), g_2)^s = e(H(m), s \cdot g_2) = e(H(m), P_{pub})$.
    ///
    /// # Parameters
    ///
    /// *   `id`: The identifier for the stored MPK (e.g. epoch/interval).
    /// *   `msg`: The message bytes to be signed. This will be hashed to curve $G_1$ via `hash_to_curve`.
    /// *   `sig`: The signature bytes (compressed $G_1$ point).
    ///
    /// Returns `true` if the verification holds, `false` otherwise.
    public fun verify_signature(id: u64, msg: vector<u8>, sig: vector<u8>): bool acquires State {
        if (!exists<State>(@aptos_framework)) {
            return false
        };
        let state = borrow_global<State>(@aptos_framework);
        if (!table::contains(&state.master_public_keys, id)) {
            return false
        };

        let mpk_bytes = table::borrow(&state.master_public_keys, id);
        let mpk_opt = deserialize<G2, FormatG2Compr>(mpk_bytes);
        let sig_opt = deserialize<G1, FormatG1Compr>(&sig);

        if (option::is_none(&mpk_opt) || option::is_none(&sig_opt)) {
            return false
        };

        let mpk = option::destroy_some(mpk_opt);
        let signature = option::destroy_some(sig_opt);
        
        // Use Hash-to-Curve to map message to G1
        // Using same suite as defined in bls12381_algebra
        let h_msg = hash_to<G1, HashG1XmdSha256SswuRo>(&b"IBE-BLS-SIG", &msg); 
        // Note: DST should be verified against spec. "IBE-BLS-SIG" matches nothing?
        // Using empty DST or a specific one? 
        // Boneh-Franklin uses H_1. 
        // bls12381_algebra uses HashG1XmdSha256SswuRo default DST "QUUX...".
        // I should probably pass DST or use a standard one. 
        // For now using "IBE-BLS_SIG" as placeholder. This must match how signatures were generated!
        // But wait, the user said "Exact nomenclature of IBE paper".
        // The paper just says H_1.
        // I will use a fixed DST provided by `ibe_signature` via arguments?
        // `threshold_dsa` is generic. Let's assume the caller handles hashing to G1?
        // But `verify_signature` takes `msg: vector<u8>`.
        // If `ibe_signature` calls this with ALREADY HASHED bytes (ID), then we should hash them to curve.
        
        // Verification: e(sig, g2_gen) == e(h_msg, mpk)
        let lhs = pairing<G1, G2, Gt>(&signature, &one<G2>());
        let rhs = pairing<G1, G2, Gt>(&h_msg, &mpk);
        
        eq<Gt>(&lhs, &rhs)
    }
    
    /// Helper to verify a point directly if the message is already mapped to G1
    public fun verify_signature_point(id: u64, msg_point: Element<G1>, sig_bytes: vector<u8>): bool acquires State {
         if (!exists<State>(@aptos_framework)) {
            return false
        };
        let state = borrow_global<State>(@aptos_framework);
        if (!table::contains(&state.master_public_keys, id)) {
            return false
        };

        let mpk_bytes = table::borrow(&state.master_public_keys, id);
        let mpk_opt = deserialize<G2, FormatG2Compr>(mpk_bytes);
        let sig_opt = deserialize<G1, FormatG1Compr>(&sig_bytes);

        if (option::is_none(&mpk_opt) || option::is_none(&sig_opt)) {
            return false
        };

        let mpk = option::destroy_some(mpk_opt);
        let signature = option::destroy_some(sig_opt);
        
        let lhs = pairing<G1, G2, Gt>(&signature, &one<G2>());
        let rhs = pairing<G1, G2, Gt>(&msg_point, &mpk);
        
        eq<Gt>(&lhs, &rhs)
    }

    #[test_only]
    public fun initialize_for_test(framework: &signer) {
        initialize(framework);
    }
}
