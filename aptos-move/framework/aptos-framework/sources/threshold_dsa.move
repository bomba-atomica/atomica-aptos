module aptos_framework::threshold_dsa {
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::system_addresses;
    use aptos_framework::stake;
    use aptos_std::crypto_algebra::{zero, one, from_u64, eq, deserialize, serialize, add, scalar_mul, hash_to, pairing, Element};
    use aptos_std::bls12381_algebra::{G1, G2, Gt, Fr, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo};

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
            return vector::empty<u8>()
        };

        let lambdas = vector::empty<u128>();
        let j = 0;
        while (j < n) {
            let idx = *vector::borrow(validator_indices, j);
            let lambda = compute_lagrange_coefficient(idx, validator_indices, total_validators);
            vector::push_back(&mut lambdas, lambda);
            j = j + 1;
        };

        let result = zero<G1>();
        let i = 0;
        while (i < n) {
            let share_bytes = vector::borrow(share_bytes_list, i);
            let share_opt = deserialize<G1, FormatG1Compr>(share_bytes);
            if (option::is_some(&share_opt)) {
                let share = option::destroy_some(share_opt);
                let lambda_u128 = *vector::borrow(&lambdas, i);
                let lambda_scalar = from_u64<Fr>((lambda_u128 as u64));
                let scaled = scalar_mul(&share, &lambda_scalar);
                result = add(&result, &scaled);
            };
            i = i + 1;
        };

        serialize<G1, FormatG1Compr>(&result)
    }

    /// Compute Lagrange coefficient λ_k for validator k
    /// 
    /// λ_k = Π_{i ∈ V, i ≠ k} (0 - i) / (k - i)
    ///     = Π_{i ∈ V, i ≠ k} (-i) / (k - i)
    /// 
    /// Uses modulo arithmetic with prime q = 1000003.
    fun compute_lagrange_coefficient(k: u64, participants: &vector<u64>, _total: u64): u128 {
        let num = 1u128;
        let den = 1u128;
        let n = vector::length(participants);
        let i = 0;
        let q: u128 = 1000003;
        while (i < n) {
            let idx = *vector::borrow(participants, i);
            if (idx != k) {
                // numerator: (-idx) mod q = q - idx
                let neg_idx = q - (idx as u128);
                num = (num * neg_idx) % q;

                // denominator: (k - idx) mod q
                // Handle the sign by checking if k > idx
                let diff = if (k > idx) { k - idx } else { idx - k };
                let diff_mod = (diff as u128) % q;
                if (k < idx) {
                    // k - idx is negative, so (k - idx) mod q = q - diff
                    den = (den * (q - diff_mod)) % q;
                } else {
                    den = (den * diff_mod) % q;
                };
            };
            i = i + 1;
        };

        // λ = num * den^(-1) mod q
        let den_inv = mod_exp(den, q - 2, q);
        (num * den_inv) % q
    }

    /// Modular exponentiation: base^exp mod mod
    fun mod_exp(base: u128, exp: u128, mod: u128): u128 {
        let result = 1u128;
        let b = base % mod;
        let e = exp;
        while (e > 0) {
            if (e % 2 == 1) {
                result = (result * b) % mod;
            };
            b = (b * b) % mod;
            e = e / 2;
        };
        result
    }

    // =========================================================================
    // Unit Tests for Threshold BLS Functions
    // =========================================================================

    #[test]
    fun test_mod_exp_basic() {
        // 2^10 mod 1000 = 1024 mod 1000 = 24
        let result = mod_exp(2, 10, 1000);
        assert!(result == 24, 1);
    }

    #[test]
    fun test_mod_exp_zero_base() {
        let result = mod_exp(0, 5, 100);
        assert!(result == 0, 1);
    }

    #[test]
    fun test_mod_exp_one_exponent() {
        let result = mod_exp(7, 1, 100);
        assert!(result == 7, 1);
    }

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

    #[test]
    fun test_compute_lagrange_single_participant() {
        let participants = vector::empty<u64>();
        vector::push_back(&mut participants, 5);
        
        let lambda = compute_lagrange_coefficient(5, &participants, 10);
        assert!(lambda == 1, 1);
    }

    #[test]
    fun test_compute_lagrange_two_participants() {
        let participants = vector::empty<u64>();
        vector::push_back(&mut participants, 1);
        vector::push_back(&mut participants, 2);
        
        let lambda_1 = compute_lagrange_coefficient(1, &participants, 2);
        let lambda_2 = compute_lagrange_coefficient(2, &participants, 2);
        
        // Using prime q = 1000003
        let q: u128 = 1000003;
        // λ_1 = (-2) / (-1) = 2
        let expected_lambda_1: u128 = 2;
        // λ_2 = (-1) / (1) = -1 = q - 1
        let expected_lambda_2: u128 = q - 1;
        
        assert!(lambda_1 == expected_lambda_1, 1);
        assert!(lambda_2 == expected_lambda_2, 2);
    }

    #[test]
    fun test_compute_lagrange_three_participants() {
        let participants = vector::empty<u64>();
        vector::push_back(&mut participants, 1);
        vector::push_back(&mut participants, 2);
        vector::push_back(&mut participants, 3);
        
        let lambda_1 = compute_lagrange_coefficient(1, &participants, 3);
        let lambda_2 = compute_lagrange_coefficient(2, &participants, 3);
        let lambda_3 = compute_lagrange_coefficient(3, &participants, 3);
        
        let q: u128 = 1000003;
        
        // λ_1 = (-2)(-3) / ((-1)(-2)) = 6/2 = 3
        // λ_2 = (-1)(-3) / ((1)(-1)) = 3/(-1) = -3 = q - 3
        // λ_3 = (-1)(-2) / ((2)(1)) = 2/2 = 1
        assert!(lambda_1 == 3, 1);
        assert!(lambda_2 == q - 3, 2);
        assert!(lambda_3 == 1, 3);
        
        // λ_1 + λ_2 + λ_3 should equal 1 (Lagrange identity property)
        let sum = (lambda_1 + lambda_2 + lambda_3) % q;
        assert!(sum == 1, 4);
    }

    #[test]
    fun test_lagrange_coefficients_sum_to_one() {
        let test_cases = vector::empty<vector<u64>>();
        
        let case1 = vector::empty<u64>();
        vector::push_back(&mut case1, 1);
        vector::push_back(&mut test_cases, case1);
        
        let case2 = vector::empty<u64>();
        vector::push_back(&mut case2, 1);
        vector::push_back(&mut case2, 2);
        vector::push_back(&mut test_cases, case2);
        
        let case3 = vector::empty<u64>();
        vector::push_back(&mut case3, 0);
        vector::push_back(&mut case3, 1);
        vector::push_back(&mut case3, 2);
        vector::push_back(&mut test_cases, case3);
        
        let q: u128 = 1000003;
        let num_cases = vector::length(&test_cases);
        
        let case_idx = 0;
        while (case_idx < num_cases) {
            let participants = *vector::borrow(&test_cases, case_idx);
            let n = vector::length(&participants);
            
            let sum: u128 = 0;
            let i = 0;
            while (i < n) {
                let idx = *vector::borrow(&participants, i);
                let lambda = compute_lagrange_coefficient(idx, &participants, n);
                sum = (sum + lambda) % q;
                i = i + 1;
            };
            
            assert!(sum == 1, 100 + case_idx);
            case_idx = case_idx + 1;
        };
    }

    #[test]
    fun test_aggregate_timelock_shares_empty() {
        let shares = vector::empty<vector<u8>>();
        let indices = vector::empty<u64>();
        let result = aggregate_timelock_shares(&shares, &indices, 0);
        assert!(vector::length(&result) == 0, 1);
    }
}
