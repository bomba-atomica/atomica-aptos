/// IBE (Identity-Based Encryption) native function wrappers.
///
/// This module provides Move wrappers for IBE native functions:
/// - `reconstruct_ibe_dk_internal`: Reconstructs DK from G1 DK shares
///
/// The native functions are implemented in Rust in the algebra natives
/// and registered via the `natives::cryptography::algebra::ibe` module.
///
/// # Data Flow (From Audit)
///
/// ```text
/// Validator holds: s_i (32-byte scalars, private)
/// Validator computes: dk_share_i = sum_j(s_i[j]) × H(identity) → 48-byte G1 point
/// Validator submits: dk_share_i (48 bytes) → stored in submitted_shares
/// Reconstruction: DK = Σ(Lagrange_i × dk_share_i) → 48-byte G1 point
/// ```
///
/// # Data Format
///
/// The native function accepts and returns byte vectors:
///
/// | Parameter | Move Type | Description |
/// |-----------|-----------|-------------|
/// | `validator_indices` | `vector<u64>` | Validator indices (0-based) |
/// | `dk_shares` | `vector<vector<u8>>` | 48-byte compressed G1 points |
/// | `weights` | `vector<u64>` | Full weights for ALL validators |
/// | `total_weight` | `u64` | Sum of all validator weights |
/// | `identity` | `vector<u8>` | 32-byte IBE identity |
/// | **Returns** | `vector<u8>` | 48-byte compressed G1 (DK) |
///
/// # Security Note
///
/// **CRITICAL**: The system accepts DK shares without cryptographic verification.
/// A malicious validator can submit arbitrary G1 points. Consider adding:
/// - Storage of public key shares from PVSS transcript
/// - Verification: e(dk_share, g₂) == e(H(identity), aggregate(pk_shares))
module aptos_std::ibe {
    use std::vector;

    /// Reconstructs an IBE decryption key from G1 DK shares.
    ///
    /// This function performs weighted Lagrange interpolation directly on G1 points.
    /// Each validator's DK share is a vector of contributions, one per virtual player:
    /// `dk_share_i_j = s_i[j] × H(identity)`
    ///
    /// # Arguments
    /// * `validator_indices` - Vector of validator indices (0-indexed, from DKG)
    /// * `dk_shares` - Nested vector of 48-byte compressed G1 points.
    ///   `dk_shares[i]` contains the shares for validator `validator_indices[i]`.
    /// * `weights` - Full vector of validator weights (for ALL validators, not just participating)
    /// * `threshold` - Minimum weight required for reconstruction (as defined during DKG)
    /// * `total_weight` - Sum of all validator weights
    /// * `identity` - 32-byte identity hash (from compute_identity)
    ///
    /// # Returns
    /// THE reconstructed decryption key as 48-byte compressed G1
    ///
    /// # Aborts
    /// - If `validator_indices` and `dk_shares` have different lengths
    /// - If any dk_share is not exactly 48 bytes
    public fun reconstruct_ibe_dk<G1>(
        validator_indices: vector<u64>,
        dk_shares: vector<vector<vector<u8>>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8> {
        reconstruct_ibe_dk_internal<G1>(
            validator_indices,
            dk_shares,
            weights,
            threshold,
            total_weight,
            identity
        )
    }

    /// Internal native function. Accepts 48-byte G1 DK shares, returns 48-byte compressed G1.
    native fun reconstruct_ibe_dk_internal<G1>(
        validator_indices: vector<u64>,
        dk_shares: vector<vector<vector<u8>>>,
        weights: vector<u64>,
        threshold: u64,
        total_weight: u64,
        identity: vector<u8>,
    ): vector<u8>;

}
