/// IBE (Identity-Based Encryption) native function wrappers.
///
/// This module provides Move wrappers for IBE native functions:
/// - `reconstruct_ibe_dk_internal`: Reconstructs DK from PVSS threshold scalar shares
///
/// The native functions are implemented in Rust in the algebra natives
/// and registered via the `natives::cryptography::algebra::ibe` module.
///
/// # Data Format
///
/// The native function accepts and returns byte vectors:
/// - `scalar_shares`: `vector<vector<u8>>` where each inner vector is a 32-byte little-endian Scalar
/// - Return value: `vector<u8>` which is a 48-byte compressed G1 (the reconstructed DK)
///
/// This matches `aptos_dkg::ibe::reconstruct_ibe_dk()` requirements.
module aptos_std::ibe {
    use std::vector;

    /// Reconstruct an IBE decryption key from PVSS threshold scalar shares.
    ///
    /// # Arguments
    /// * `validator_indices` - Vector of validator indices (0-indexed, from DKG)
    /// * `scalar_shares` - Nested vector of 32-byte little-endian scalar shares
    /// * `weights` - Full vector of validator weights (for ALL validators, not just participating)
    /// * `threshold` - Minimum number of shares required for reconstruction
    /// * `total_weight` - Sum of all validator weights
    /// * `identity` - 32-byte identity hash (from compute_identity)
    ///
    /// # Returns
    /// The reconstructed decryption key as 48-byte compressed G1
    ///
    /// # Aborts
    /// - If `validator_indices` and `scalar_shares` have different lengths
    /// - If fewer than `threshold` weight units are provided
    public fun reconstruct_ibe_dk<G1>(
        validator_indices: vector<u64>,
        scalar_shares: vector<vector<u8>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8> {
        reconstruct_ibe_dk_internal<G1>(
            validator_indices,
            scalar_shares,
            weights,
            threshold,
            total_weight,
            identity
        )
    }

    /// Internal native function. Returns 48-byte compressed G1.
    native fun reconstruct_ibe_dk_internal<G1>(
        validator_indices: vector<u64>,
        scalar_shares: vector<vector<u8>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8>;
}
