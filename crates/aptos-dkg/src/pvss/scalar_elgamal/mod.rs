//! # Scalar ElGamal PVSS Module
//!
//! This module implements a Publicly Verifiable Secret Sharing (PVSS) scheme that outputs
//! **scalar shares** for use with Identity-Based Encryption (IBE).
//!
//! ## Motivation
//!
//! The existing DAS PVSS outputs G1 group elements, which work for WVUF (randomness) but
//! are incompatible with standard Boneh-Franklin IBE that requires scalar secrets.
//!
//! This module provides a second PVSS variant that:
//! - Uses the same `InputSecret` (a scalar) as DAS
//! - Outputs scalar shares directly (encrypted with ElGamal)
//! - Produces the same MPK (`g2^secret`) as DAS
//!
//! ## Architecture
//!
//! ```text
//! InputSecret (scalar a)
//!        │
//!        ├──► DAS PVSS      → G1 = g1^a  → WVUF (randomness)
//!        │
//!        └──► Scalar PVSS   → scalar a   → IBE (timelock)
//!
//! Both produce MPK = g2^a (identical)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use aptos_dkg::pvss::scalar_elgamal::{Transcript, WeightedTranscript};
//!
//! // For unweighted threshold
//! let transcript = Transcript::deal(sc, pp, sk, eks, input_secret, aux, dealer, rng);
//!
//! // For weighted threshold (production use)
//! let weighted_transcript = WeightedTranscript::deal(...);
//! ```
//!
//! ## Security
//!
//! - Shares are ElGamal-encrypted under each validator's public key
//! - DLEQ proofs ensure share consistency without revealing values
//! - Standard discrete-log security on BLS12-381
//!
//! ## See Also
//!
//! - [`crate::pvss::das`] - DAS PVSS (G1 output) for randomness
//! - [`crate::pvss::insecure_field`] - Reference implementation (unencrypted shares)
//! - [`crate::ibe`] - IBE module that consumes scalar shares

pub mod transcript;
pub mod weighted_protocol;

// Re-export main types for convenience
pub use transcript::Transcript;
pub use weighted_protocol::WeightedTranscript;

use crate::pvss::{
    das::PublicParameters, dealt_secret_key::scalar::DealtSecretKey, input_secret::InputSecret,
    traits::Convert,
};

impl Convert<DealtSecretKey, PublicParameters> for InputSecret {
    fn to(&self, _pp: &PublicParameters) -> DealtSecretKey {
        DealtSecretKey::new(*self.get_secret_a())
    }
}

/// Domain separation tag for Fiat-Shamir in this PVSS scheme.
///
/// Used in DLEQ proof generation to prevent cross-protocol attacks.
pub const SCALAR_ELGAMAL_DST: &[u8] = b"APTOS_SCALAR_ELGAMAL_PVSS_DST";

/// Scheme identifier for logging and debugging.
pub const SCHEME_NAME: &str = "scalar_elgamal_pvss";

#[cfg(test)]
mod tests {
    //! # Test Plan for Scalar ElGamal PVSS
    //!
    //! ## Unit Tests (this module)
    //!
    //! | Test | Description | Status |
    //! |------|-------------|--------|
    //! | `test_deal_and_verify` | Deal transcript, verify it passes | TODO |
    //! | `test_decrypt_own_share` | Validator can decrypt their share | TODO |
    //! | `test_wrong_key_fails` | Wrong decryption key fails | TODO |
    //! | `test_aggregate_transcripts` | Multiple transcripts aggregate correctly | TODO |
    //! | `test_reconstruct_secret` | Threshold shares reconstruct original | TODO |
    //! | `test_share_consistency` | Shares match commitments | TODO |
    //! | `test_dleq_proof_valid` | DLEQ proofs verify correctly | TODO |
    //! | `test_dleq_proof_tampered` | Tampered DLEQ proofs fail | TODO |
    //!
    //! ## Integration Tests
    //!
    //! | Test | Description | Status |
    //! |------|-------------|--------|
    //! | `test_same_input_secret_same_mpk` | DAS and Scalar produce same MPK | TODO |
    //! | `test_weighted_threshold` | Weighted config works correctly | TODO |
    //!
    //! ## Run Tests
    //!
    //! ```bash
    //! cargo test -p aptos-dkg --lib pvss::scalar_elgamal:: -- --nocapture
    //! ```

    use super::*;

    /// Placeholder test to ensure module compiles.
    /// TODO: Replace with actual tests once transcript.rs is implemented.
    #[test]
    fn test_module_compiles() {
        // This test just verifies the module structure is correct
        assert_eq!(SCHEME_NAME, "scalar_elgamal_pvss");
    }
}
