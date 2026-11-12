/// Identity-based encryption (Boneh-Franklin on BLS12-381) native function declarations.
///
/// Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md`.
///
/// The Rust implementations live in the natives crate (feature natives-port-timelock).
/// This module only declares the Move signatures that the registry (`ibe_config`) and the
/// auction contract call.
///
/// | Native | Spec section | Purpose |
/// |---|---|---|
/// | `reconstruct_ibe_dk` | 5 | Weighted Lagrange interpolation of G1 key shares |
/// | `verify_decryption_key` | 5 | Pairing check `e(dk, g2) = e(H1(id), mpk)` |
/// | `verify_bid_ct` | 6 | Submission check of a bid ciphertext against its associated data |
///
/// Abort codes of `reconstruct_ibe_dk` (from the native): 1 when the type argument is not
/// `bls12381_algebra::G1`, 2 for malformed input (shape, bounds, weights, below threshold, or a
/// share that is not a valid prime-order non-identity G1 point).
module aptos_std::ibe {
    // SCOUT(#227) DOC: the module table above gains a `decrypt_bid` row (spec section 6) and the stale Cargo feature name `natives-port-timelock` goes.
    // SCOUT(#240) DOC: the module table and the abort code paragraph above lose `reconstruct_ibe_dk`.
    // SCOUT(#240) REMOVE: `reconstruct_ibe_dk` has no caller outside tests; the `TimelockShare` handler rebuilds the key in Rust.
    /// Reconstructs an IBE decryption key from G1 key shares.
    ///
    /// Performs weighted Lagrange interpolation directly on G1 points. Each validator's entry
    /// holds one 48-byte compressed G1 point per virtual player.
    ///
    /// * `validator_indices` - validator indices (0-based, from the DKG).
    /// * `dk_shares` - `dk_shares[i]` holds the shares of validator `validator_indices[i]`.
    /// * `weights` - weights of every validator of the epoch, indexed by validator index (the full
    ///   per-validator list, not only the submitters). Validator `i` owns the virtual players that
    ///   start at `weights[0] + ... + weights[i - 1]`, and the weights must sum to `total_weight`.
    /// * `threshold` - minimum weight required for reconstruction.
    /// * `total_weight` - sum of all validator weights.
    /// * `identity` - 32-byte identity (see `ibe_config::compute_identity`).
    ///
    /// Returns the 48-byte compressed G1 decryption key. Aborts on malformed input.
    public fun reconstruct_ibe_dk<G1>(
        validator_indices: vector<u64>,
        dk_shares: vector<vector<vector<u8>>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8> {
        reconstruct_ibe_dk_internal<G1>(
            validator_indices, dk_shares, weights, threshold, total_weight, identity
        )
    }

    /// Checks `e(dk, g2) = e(H1(identity), mpk)`.
    ///
    /// * `dk` - 48-byte compressed G1 key. Decoded with subgroup validation.
    /// * `identity` - 32-byte identity. The native derives `H1(identity)` with the V2 domain tag.
    /// * `mpk` - 96-byte compressed G2 master public key.
    ///
    /// Returns false for any key that fails to decode or fails the pairing check.
    public fun verify_decryption_key(
        dk: vector<u8>, identity: vector<u8>, mpk: vector<u8>
    ): bool {
        verify_decryption_key_internal(dk, identity, mpk)
    }

    /// Verifies a bid ciphertext at submission time.
    ///
    /// * `ct` - BCS of the five byte vectors `(vk, U, nonce, C, sig)`: 32, 96, 12, at least 16 and
    ///   64 bytes. Trailing bytes and any other length are rejected.
    /// * `ad` - associated data the contract built on chain (BCS of tag, chain id, epoch,
    ///   timelock id, auction id, bidder, deadline). It is not part of `ct`. It is bound by the
    ///   one-time signature, so a caller cannot supply a different bidder or auction.
    ///
    /// Checks: `U` is a valid prime-order non-identity G2 point, and the one-time Ed25519
    /// signature (strict verification) over `BCS(U, nonce, C, AD)` verifies for `vk`.
    /// Returns false on any failure.
    public fun verify_bid_ct(ct: vector<u8>, ad: vector<u8>): bool {
        verify_bid_ct_internal(ct, ad)
    }

    // SCOUT(#227) ADD: `public fun decrypt_bid(dk: vector<u8>, identity: vector<u8>, ad: vector<u8>, ct: vector<u8>): Option<vector<u8>>` that calls the native `decrypt_bid_internal` and never aborts on bad input. A public stdlib function cannot be removed after release, so it is not stubbed here.
    // SCOUT(#228) EDIT: the product `auction::open_bids` calls `decrypt_bid` once per bid with the key and identity from the timelock registry.

    // SCOUT(#240) REMOVE: native declaration of the unused reconstruction native.
    native fun reconstruct_ibe_dk_internal<G1>(
        validator_indices: vector<u64>,
        dk_shares: vector<vector<vector<u8>>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8>;

    native fun verify_decryption_key_internal(
        dk: vector<u8>, identity: vector<u8>, mpk: vector<u8>
    ): bool;

    native fun verify_bid_ct_internal(ct: vector<u8>, ad: vector<u8>): bool;

    // SCOUT(#227) ADD: `native fun decrypt_bid_internal(dk: vector<u8>, identity: vector<u8>, ad: vector<u8>, ct: vector<u8>): Option<vector<u8>>;` goes here, in the same commit that registers `native_decrypt_bid` in `timelock::make_all`. A native declared without its Rust side breaks loading of this module.
}
