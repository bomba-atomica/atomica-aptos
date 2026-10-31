// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Boneh-Franklin identity-based encryption on BLS12-381 for the Atomica timelock, on arkworks.
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` (sections 5 and 6).
//!
//! - Master public key `mpk = g2^s` (G2, 96 bytes compressed). `s` is dealt by a chunky DKG
//!   session and is never reconstructed. Only shares of `s * H1(id)` are combined.
//! - Identity `id = SHA3-256(tag || chain_id || epoch || timelock_id || deadline_us)`, `Q = H1(id)`
//!   with H1 the RFC 9380 `BLS12381G1_XMD:SHA-256_SSWU_RO_` suite and a dedicated DST.
//! - Decryption key `dk = s * Q` (G1, 48 bytes compressed). A share is `s_p * Q`.
//! - KEM: `U = g2^r`, `K = e(Q, mpk)^r`. Decapsulation: `K = e(dk, U)`.
//!
//! The un-authenticated XOR stream cipher of the earlier `blstrs` code is not ported. Bids use
//! the AEAD wrapper in [`ciphertext`].
//!
//! Every group element that arrives from outside (shares, keys, `mpk`, `U`) is decoded with
//! [`decode_g1`] or [`decode_g2`]. Those check length, curve membership, the prime-order
//! subgroup, and reject the identity. No decode in this module uses `Validate::No`.

pub mod ciphertext;

#[cfg(test)]
mod golden_vectors;
#[cfg(test)]
mod tests;

use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
pub use ark_bls12_381::Fr as MasterScalar;
use ark_bls12_381::{
    g1::Config as G1Config, Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective,
};
use ark_ec::{
    hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
    pairing::{Pairing, PairingOutput},
    AffineRepr, CurveGroup, VariableBaseMSM,
};
use ark_ff::{field_hashers::DefaultFieldHasher, PrimeField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
pub use ciphertext::{BidCiphertext, MAX_CIPHERTEXT_BYTES};
use sha2_0_10_6::Sha256;
use sha3::{Digest, Sha3_256};
use std::{collections::HashSet, ops::Mul};

/// Compressed G1 length (decryption keys and key shares).
pub const G1_COMPRESSED_LEN: usize = 48;
/// Compressed G2 length (`mpk`, `U`, verification keys).
pub const G2_COMPRESSED_LEN: usize = 96;
/// Identity length in bytes.
pub const IDENTITY_LEN: usize = 32;

/// RFC 9380 domain separation tag for `H1`. Distinct from the mempool's
/// `APTOS_BATCH_ENCRYPTION_HASH_G2_ELEMENT` and from the retired `APTOS_IBE_IDENTITY_DST`.
pub const IBE_H1_DST: &[u8] = b"ATOMICA_TIMELOCK_IBE_V2_H1";
/// Prefix of the identity hash. Must equal `IDENTITY_TAG` in `ibe_config.move`.
pub const IBE_IDENTITY_TAG: &[u8] = b"ATOMICA_TIMELOCK_IBE_V2_ID";
/// HKDF salt for the bid key derivation.
pub const IBE_KDF_SALT: &[u8] = b"ATOMICA_TIMELOCK_IBE_V2_KDF";

/// Upper bound on the number of virtual players (total weight) accepted by reconstruction.
///
/// The Lagrange computation builds an FFT domain of the next power of two above the total
/// weight. This keeps that work bounded for a caller-supplied value.
pub const MAX_TOTAL_WEIGHT: usize = 1 << 16;

/// Errors of the IBE module. No function in this module panics on malformed input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IbeError {
    /// `validator_indices` and the share lists have different lengths.
    ValidatorIndicesSharesMismatch {
        indices_len: usize,
        shares_len: usize,
    },
    /// No shares were supplied.
    EmptyShares,
    /// `total_weight` differs from the sum of `weights`.
    WeightSumMismatch {
        total_weight: u64,
        computed_sum: u64,
    },
    /// A validator supplied a number of shares different from its weight.
    ShareWeightMismatch {
        validator_index: u64,
        shares_count: usize,
        expected_weight: u64,
    },
    /// Threshold or total weight is out of range.
    InvalidWeightedConfig(String),
    /// A validator index is outside the weights vector.
    PlayerOutOfBounds { index: u64, num_players: usize },
    /// The same validator index appears twice.
    DuplicatePlayer(u64),
    /// Fewer shares than the threshold weight.
    BelowThreshold { have: usize, need: usize },
    /// Wrong byte length for a group element or field.
    InvalidLength {
        what: &'static str,
        expected: usize,
        got: usize,
    },
    /// A group element failed validated decoding (not on curve, not in the subgroup, or the identity).
    InvalidPoint(&'static str),
    /// Hash to curve failed.
    HashToCurve,
    /// A ciphertext failed to parse.
    MalformedCiphertext(String),
    /// The one-time signature did not verify.
    BadSignature,
    /// AEAD authentication failed.
    DecryptionFailed,
    /// Randomness or encryption failure.
    EncryptionFailed,
    /// The master public key is not the G2 interpolation of the virtual players' verification keys.
    MpkVerificationKeyMismatch,
}

impl std::fmt::Display for IbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for IbeError {}

type G1Hasher = MapToCurveBasedHasher<G1Projective, DefaultFieldHasher<Sha256>, WBMap<G1Config>>;

/// Computes the 32 byte identity `SHA3-256(tag || chain_id || epoch || timelock_id || deadline_us)`.
///
/// Integers are little endian, matching the BCS encoding used by `ibe_config::compute_identity`.
pub fn compute_identity(
    chain_id: u8,
    epoch: u64,
    timelock_id: u64,
    deadline_us: u64,
) -> [u8; IDENTITY_LEN] {
    let mut hasher = Sha3_256::new();
    hasher.update(IBE_IDENTITY_TAG);
    hasher.update([chain_id]);
    hasher.update(epoch.to_le_bytes());
    hasher.update(timelock_id.to_le_bytes());
    hasher.update(deadline_us.to_le_bytes());
    hasher.finalize().into()
}

/// `H1`: hashes identity bytes to G1 (RFC 9380 SSWU, random oracle variant).
pub fn hash_to_g1(identity: &[u8]) -> Result<G1Affine, IbeError> {
    let hasher = G1Hasher::new(IBE_H1_DST).map_err(|_| IbeError::HashToCurve)?;
    hasher.hash(identity).map_err(|_| IbeError::HashToCurve)
}

/// Validated decode of a compressed G1 point: exact length, on curve, prime-order subgroup,
/// not the identity.
pub fn decode_g1(bytes: &[u8]) -> Result<G1Affine, IbeError> {
    if bytes.len() != G1_COMPRESSED_LEN {
        return Err(IbeError::InvalidLength {
            what: "g1",
            expected: G1_COMPRESSED_LEN,
            got: bytes.len(),
        });
    }
    let p = G1Affine::deserialize_with_mode(bytes, Compress::Yes, Validate::Yes).map_err(|_| {
        IbeError::InvalidPoint("g1 not on curve or not in the prime-order subgroup")
    })?;
    if p.is_zero() {
        return Err(IbeError::InvalidPoint("g1 is the identity"));
    }
    Ok(p)
}

/// Validated decode of a compressed G2 point. Same checks as [`decode_g1`].
pub fn decode_g2(bytes: &[u8]) -> Result<G2Affine, IbeError> {
    if bytes.len() != G2_COMPRESSED_LEN {
        return Err(IbeError::InvalidLength {
            what: "g2",
            expected: G2_COMPRESSED_LEN,
            got: bytes.len(),
        });
    }
    let p = G2Affine::deserialize_with_mode(bytes, Compress::Yes, Validate::Yes).map_err(|_| {
        IbeError::InvalidPoint("g2 not on curve or not in the prime-order subgroup")
    })?;
    if p.is_zero() {
        return Err(IbeError::InvalidPoint("g2 is the identity"));
    }
    Ok(p)
}

/// Compressed encoding of a G1 point.
pub fn encode_g1(p: &G1Affine) -> [u8; G1_COMPRESSED_LEN] {
    let mut out = [0u8; G1_COMPRESSED_LEN];
    p.serialize_compressed(&mut out[..])
        .expect("a G1 point always fits in 48 bytes");
    out
}

/// Compressed encoding of a G2 point.
pub fn encode_g2(p: &G2Affine) -> [u8; G2_COMPRESSED_LEN] {
    let mut out = [0u8; G2_COMPRESSED_LEN];
    p.serialize_compressed(&mut out[..])
        .expect("a G2 point always fits in 96 bytes");
    out
}

/// Master public key `g2^s` for a master secret (tests and tooling only: the product never
/// holds `s`).
pub fn master_public_key(secret: &Fr) -> G2Affine {
    (G2Projective::from(G2Affine::generator()) * secret).into_affine()
}

/// Boneh-Franklin Extract, or a key share when `secret` is a Shamir share: `secret * H1(id)`.
pub fn derive_decryption_key(secret: &Fr, identity: &[u8]) -> Result<G1Affine, IbeError> {
    let q = hash_to_g1(identity)?;
    Ok(q.mul(secret).into_affine())
}

/// Checks `e(dk, g2) = e(H1(id), mpk)` as one multi-pairing `e(dk, g2) * e(-Q, mpk) = 1`.
///
/// Both points must already be validated (use [`decode_g1`] and [`decode_g2`]).
pub fn verify_decryption_key(dk: &G1Affine, identity: &[u8], mpk: &G2Affine) -> bool {
    let q = match hash_to_g1(identity) {
        Ok(q) => q,
        Err(_) => return false,
    };
    Bls12_381::multi_pairing([*dk, -q], [G2Affine::generator(), *mpk]).is_zero()
}

/// Checks one virtual player's key share `share = s_p * H1(id)` against its verification key
/// `vk_p = g2^{s_p}`: `e(Q, vk_p) = e(share, g2)`, as one multi-pairing.
pub fn verify_dk_share(share: &G1Affine, identity: &[u8], vk: &G2Affine) -> bool {
    let q = match hash_to_g1(identity) {
        Ok(q) => q,
        Err(_) => return false,
    };
    Bls12_381::multi_pairing([q, -*share], [*vk, G2Affine::generator()]).is_zero()
}

/// Why [`verify_player_shares`] refused a share set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareCheckFailure {
    /// The number of shares differs from the number of verification keys of the author.
    CountMismatch { shares: usize, keys: usize },
    /// The identity does not hash to a curve point.
    Identity,
    /// Share `index` failed validated decoding.
    InvalidShare { index: usize, error: IbeError },
    /// Verification key `index` failed validated decoding.
    InvalidVerificationKey { index: usize, error: IbeError },
    /// Share `index` failed `e(H1(id), vk) = e(share, g2)`.
    PairingMismatch { index: usize },
}

/// Boneh-Franklin share check for all virtual players of one validator.
///
/// `shares[j]` and `verification_keys[j]` belong to the same virtual player `j`. Every point is decoded
/// with validation (on curve, prime-order subgroup, not the identity) and every share must satisfy
/// `e(H1(identity), vk_j) = e(share_j, g2)`. `H1(identity)` is computed once. The result depends only on
/// the arguments, so every validator reaches the same verdict. Never panics.
pub fn verify_player_shares(
    identity: &[u8],
    shares: &[Vec<u8>],
    verification_keys: &[Vec<u8>],
) -> Result<(), ShareCheckFailure> {
    if shares.is_empty() || shares.len() != verification_keys.len() {
        return Err(ShareCheckFailure::CountMismatch {
            shares: shares.len(),
            keys: verification_keys.len(),
        });
    }
    let q = hash_to_g1(identity).map_err(|_| ShareCheckFailure::Identity)?;
    let g2 = G2Affine::generator();
    for (index, (share, vk)) in shares.iter().zip(verification_keys).enumerate() {
        let share =
            decode_g1(share).map_err(|error| ShareCheckFailure::InvalidShare { index, error })?;
        let vk = decode_g2(vk)
            .map_err(|error| ShareCheckFailure::InvalidVerificationKey { index, error })?;
        if !Bls12_381::multi_pairing([q, -share], [vk, g2]).is_zero() {
            return Err(ShareCheckFailure::PairingMismatch { index });
        }
    }
    Ok(())
}

/// KEM encapsulation with an explicit randomness scalar. Returns `(U, K)`.
///
/// `U = g2^r`, `K = e(r * Q, mpk)`. `r` must be fresh and uniform in production. The scalar is
/// explicit so that vectors are reproducible.
pub fn encapsulate_with_scalar(
    mpk: &G2Affine,
    identity: &[u8],
    r: &Fr,
) -> Result<(G2Affine, PairingOutput<Bls12_381>), IbeError> {
    let q = hash_to_g1(identity)?;
    let u = (G2Projective::from(G2Affine::generator()) * r).into_affine();
    let q_r = q.mul(r).into_affine();
    Ok((u, Bls12_381::pairing(q_r, *mpk)))
}

/// KEM decapsulation: `K = e(dk, U)`.
pub fn decapsulate(dk: &G1Affine, u: &G2Affine) -> PairingOutput<Bls12_381> {
    Bls12_381::pairing(*dk, *u)
}

/// Samples a nonzero scalar from 64 uniform bytes (negligible bias).
pub fn random_scalar<R: rand::Rng>(rng: &mut R) -> Fr {
    loop {
        let mut bytes = [0u8; 64];
        rng.fill(&mut bytes[..]);
        let s = Fr::from_le_bytes_mod_order(&bytes);
        if !s.is_zero() {
            return s;
        }
    }
}

/// Weighted Lagrange interpolation at zero in G1, over validated shares.
///
/// `player_ids` are virtual player ids (indices into the roots of unity domain of the next power
/// of two above `total_weight`, the layout of the chunky DKG's `ShamirThresholdConfig`).
pub fn reconstruct_dk(
    player_ids: &[usize],
    shares: &[G1Affine],
    threshold_weight: usize,
    total_weight: usize,
) -> Result<G1Affine, IbeError> {
    if player_ids.len() != shares.len() {
        return Err(IbeError::ValidatorIndicesSharesMismatch {
            indices_len: player_ids.len(),
            shares_len: shares.len(),
        });
    }
    if shares.is_empty() {
        return Err(IbeError::EmptyShares);
    }
    if total_weight == 0 || total_weight > MAX_TOTAL_WEIGHT {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "total weight {} outside 1..={}",
            total_weight, MAX_TOTAL_WEIGHT
        )));
    }
    if threshold_weight == 0 || threshold_weight > total_weight {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "threshold {} outside 1..={}",
            threshold_weight, total_weight
        )));
    }
    if shares.len() < threshold_weight {
        return Err(IbeError::BelowThreshold {
            have: shares.len(),
            need: threshold_weight,
        });
    }
    let mut seen = HashSet::with_capacity(player_ids.len());
    for &id in player_ids {
        if id >= total_weight {
            return Err(IbeError::PlayerOutOfBounds {
                index: id as u64,
                num_players: total_weight,
            });
        }
        if !seen.insert(id) {
            return Err(IbeError::DuplicatePlayer(id as u64));
        }
    }
    let config = ShamirThresholdConfig::<Fr>::new(threshold_weight, total_weight);
    let lagrange = config.lagrange_for_subset(player_ids);
    G1Projective::msm(shares, &lagrange)
        .map(|p| p.into_affine())
        .map_err(|_| IbeError::InvalidWeightedConfig("msm length mismatch".to_string()))
}

/// Weighted Lagrange interpolation at zero in G2, over already validated points.
///
/// Same layout as [`reconstruct_dk`]: `player_ids` index the virtual players (flat, validator order).
pub fn interpolate_g2_at_zero(
    player_ids: &[usize],
    points: &[G2Affine],
    threshold_weight: usize,
    total_weight: usize,
) -> Result<G2Affine, IbeError> {
    if player_ids.len() != points.len() || points.len() < threshold_weight {
        return Err(IbeError::BelowThreshold {
            have: points.len().min(player_ids.len()),
            need: threshold_weight,
        });
    }
    if total_weight == 0 || total_weight > MAX_TOTAL_WEIGHT {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "total weight {} outside 1..={}",
            total_weight, MAX_TOTAL_WEIGHT
        )));
    }
    if threshold_weight == 0 || threshold_weight > total_weight {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "threshold {} outside 1..={}",
            threshold_weight, total_weight
        )));
    }
    let mut seen = HashSet::with_capacity(player_ids.len());
    for &id in player_ids {
        if id >= total_weight {
            return Err(IbeError::PlayerOutOfBounds {
                index: id as u64,
                num_players: total_weight,
            });
        }
        if !seen.insert(id) {
            return Err(IbeError::DuplicatePlayer(id as u64));
        }
    }
    let config = ShamirThresholdConfig::<Fr>::new(threshold_weight, total_weight);
    let lagrange = config.lagrange_for_subset(player_ids);
    G2Projective::msm(points, &lagrange)
        .map(|p| p.into_affine())
        .map_err(|_| IbeError::InvalidWeightedConfig("msm length mismatch".to_string()))
}

/// Checks that `mpk` (`V0`) equals the G2 interpolation of threshold sets of the verification keys.
///
/// Three threshold sets are interpolated: the first `t` virtual players, the last `t`, and `t`
/// players spread across the whole range. The spread set takes every `k`-th player for the largest
/// `k` that still leaves `t` players, so it mixes validators. A subtranscript whose `V0` and `Vs`
/// disagree fails at least one of them unless the shares lie on one polynomial of degree below `t`
/// that passes through `V0`. The whole-polynomial question stays open for the cryptographer (see the
/// review packet).
pub fn check_mpk_matches_verification_keys(
    mpk: &G2Affine,
    verification_keys: &[G2Affine],
    threshold_weight: usize,
) -> Result<(), IbeError> {
    let total = verification_keys.len();
    if threshold_weight == 0 || threshold_weight > total {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "threshold {} outside 1..={}",
            threshold_weight, total
        )));
    }
    let t = threshold_weight;
    let first: Vec<usize> = (0..t).collect();
    let last: Vec<usize> = (total - t..total).collect();
    let stride = (total / t).max(1);
    let mut spread: Vec<usize> = (0..total).step_by(stride).take(t).collect();
    // Fill from the front when the stride left fewer than `t` players.
    let mut next = 0usize;
    while spread.len() < t {
        if !spread.contains(&next) {
            spread.push(next);
        }
        next += 1;
    }
    for ids in [first, last, spread] {
        let points: Vec<G2Affine> = ids.iter().map(|&i| verification_keys[i]).collect();
        let v0 = interpolate_g2_at_zero(&ids, &points, t, total)?;
        if &v0 != mpk {
            return Err(IbeError::MpkVerificationKeyMismatch);
        }
    }
    Ok(())
}

/// Byte level reconstruction used by the `ibe::reconstruct_ibe_dk` native.
///
/// `dk_shares[i]` holds the compressed G1 shares of validator `validator_indices[i]`, one per
/// virtual player, in order. Every share is decoded with [`decode_g1`]. The caller (the registry)
/// verifies each share against its verification key before storing it. This function does not
/// know the verification keys, so the rebuilt key must still pass [`verify_decryption_key`].
pub fn reconstruct_ibe_dk_from_g1_shares(
    validator_indices: &[u64],
    dk_shares: &[Vec<Vec<u8>>],
    weights: &[u64],
    threshold: u64,
    total_weight: u64,
) -> Result<G1Affine, IbeError> {
    if validator_indices.len() != dk_shares.len() {
        return Err(IbeError::ValidatorIndicesSharesMismatch {
            indices_len: validator_indices.len(),
            shares_len: dk_shares.len(),
        });
    }
    if validator_indices.is_empty() {
        return Err(IbeError::EmptyShares);
    }
    let computed_sum = weights
        .iter()
        .try_fold(0u64, |acc, w| acc.checked_add(*w))
        .ok_or_else(|| IbeError::InvalidWeightedConfig("weight sum overflow".to_string()))?;
    if computed_sum != total_weight {
        return Err(IbeError::WeightSumMismatch {
            total_weight,
            computed_sum,
        });
    }
    if total_weight == 0 || total_weight > MAX_TOTAL_WEIGHT as u64 {
        return Err(IbeError::InvalidWeightedConfig(format!(
            "total weight {} outside 1..={}",
            total_weight, MAX_TOTAL_WEIGHT
        )));
    }
    let mut starting_index = Vec::with_capacity(weights.len());
    let mut acc = 0u64;
    for w in weights {
        starting_index.push(acc);
        acc += w;
    }

    let mut seen = HashSet::new();
    let mut ids: Vec<usize> = Vec::new();
    let mut points: Vec<G1Affine> = Vec::new();
    for (validator_shares, &validator_index) in dk_shares.iter().zip(validator_indices) {
        let vi = usize::try_from(validator_index)
            .ok()
            .filter(|vi| *vi < weights.len())
            .ok_or(IbeError::PlayerOutOfBounds {
                index: validator_index,
                num_players: weights.len(),
            })?;
        if !seen.insert(vi) {
            return Err(IbeError::DuplicatePlayer(validator_index));
        }
        if validator_shares.len() as u64 != weights[vi] {
            return Err(IbeError::ShareWeightMismatch {
                validator_index,
                shares_count: validator_shares.len(),
                expected_weight: weights[vi],
            });
        }
        for (j, share) in validator_shares.iter().enumerate() {
            points.push(decode_g1(share)?);
            ids.push((starting_index[vi] as usize) + j);
        }
    }
    let threshold = usize::try_from(threshold)
        .map_err(|_| IbeError::InvalidWeightedConfig("threshold too large".to_string()))?;
    reconstruct_dk(&ids, &points, threshold, total_weight as usize)
}
