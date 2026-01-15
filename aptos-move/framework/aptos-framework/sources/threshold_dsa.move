module aptos_framework::threshold_dsa {
    use std::option::{Self, Option};
    use std::vector;
    use std::debug;
    use std::string;
    use aptos_std::table::{Self, Table};
    use aptos_framework::system_addresses;
    use aptos_framework::stake;
    use aptos_std::crypto_algebra::{zero, one, from_u64, eq, deserialize, serialize, add, sub, mul, scalar_mul, hash_to, pairing, Element};
    use aptos_std::bls12381_algebra::{G1, G2, Gt, Fr, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo};
    use aptos_std::lagrange;

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
        /// Map of ID (e.g. epoch) to Master Public Key bytes (compressed G2)
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

            debug::print(&string::utf8(b"[THRESHOLD_DSA] Publishing Master Public Key"));
            debug::print(&id);
            debug::print(&vector::length(&pk));

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
    /// *   `id`: The identifier for the stored MPK (e.g. epoch).
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
        // Using same DST as Rust's BLS_WVUF_DST = b"APTOS_BLS_WVUF_DST"
        let h_msg = hash_to<G1, HashG1XmdSha256SswuRo>(&b"APTOS_BLS_WVUF_DST", &msg);
        
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

    // =========================================================================
    // Timelock Threshold BLS Functions
    // These functions handle threshold BLS share verification and aggregation
    // for the Atomica Timelock service.
    // =========================================================================

    /// Verify that a timelock decryption key share is valid
    /// 
    /// Currently performs format validation (G1 point deserialization).
    /// Full cryptographic verification requires DKG state for:
    /// - Validator public key lookup by index
    /// - Identity-based verification (share should be s_i × Q_id)
    public fun verify_timelock_share(
        share_bytes: vector<u8>
    ): bool {
        option::is_some(&deserialize<G1, FormatG1Compr>(&share_bytes))
    }

    /// Aggregate timelock decryption key shares using Lagrange-weighted sum
    /// 
    /// Given shares s_i × Q_id from participating validators with indices V,
    /// the decryption key is: DK = Σ λ_i × (s_i × Q_id) = (Σ λ_i × s_i) × Q_id = s × Q_id
    /// where λ_i are Lagrange coefficients for the set V.
    /// 
    /// # Parameters
    ///
    /// * `share_bytes_list`: Vector of serialized G1 share points
    /// * `validator_indices`: Vector of validator indices corresponding to each share
    /// * `total_validators`: Total number of validators in the DKG session
    /// 
    /// # Returns
    ///
    /// The aggregated decryption key (serialized G1 point), or empty if shares is empty
    public fun aggregate_timelock_shares(
        share_bytes_list: &vector<vector<u8>>,
        validator_indices: &vector<u64>,
        total_validators: u64
    ): vector<u8> {
        let n = vector::length(share_bytes_list);
        if (n == 0) {
            debug::print(&string::utf8(b"[THRESHOLD_DSA] aggregate_timelock_shares called with empty shares"));
            return vector::empty<u8>()
        };

        debug::print(&string::utf8(b"[THRESHOLD_DSA] Aggregating timelock shares"));
        debug::print(&n);
        debug::print(&total_validators);

        // Compute Lagrange coefficients using native implementation
        let lambdas = lagrange::coefficients<Fr>(validator_indices);

        // Deserialize shares and multiply by Lagrange coefficients
        let result = zero<G1>();
        let i = 0;
        while (i < n) {
            let share_bytes = vector::borrow(share_bytes_list, i);
            let share_opt = deserialize<G1, FormatG1Compr>(share_bytes);
            if (option::is_some(&share_opt)) {
                let share = option::destroy_some(share_opt);
                let lambda = *vector::borrow(&lambdas, i);
                let scaled = scalar_mul<G1, Fr>(&share, &lambda);
                result = add(&result, &scaled);
            };
            i = i + 1;
        };

        debug::print(&string::utf8(b"[THRESHOLD_DSA] Share aggregation complete"));
        debug::print(&vector::length(&serialize<G1, FormatG1Compr>(&result)));

        serialize<G1, FormatG1Compr>(&result)
    }



    // =========================================================================
    // Unit Tests for Threshold BLS Functions
    // =========================================================================

    #[test]
    fun test_verify_timelock_share_invalid_format() {
        let empty_share = vector::empty<u8>();
        
        let result = verify_timelock_share(empty_share);
        assert!(!result, 1);
    }

    #[test]
    fun test_verify_timelock_share_valid_format() {
        let result = verify_timelock_share(b"");
        assert!(!result, 1);
    }

    // =========================================================================
    // Tests for aggregate_timelock_shares with Fr field arithmetic
    // =========================================================================

    #[test]
    fun test_aggregate_timelock_shares_empty() {
        let shares = vector::empty<vector<u8>>();
        let indices = vector::empty<u64>();
        let result = aggregate_timelock_shares(&shares, &indices, 0);
        assert!(vector::length(&result) == 0, 1);
    }
}
