// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bid ciphertext: Boneh-Franklin KEM, HKDF key binding, AES-128-GCM, one-time Ed25519 signature.
//!
//! Spec section 6 of `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md`.
//!
//! ```text
//! (sk, vk) = fresh Ed25519 key pair      r = fresh scalar      U = g2^r      K = e(Q, mpk)^r
//! key = HKDF-SHA256(salt, K || U || id || vk || SHA3-256(AD))[0..16]
//! C   = AES-128-GCM(key, nonce, m, aad = AD)
//! sig = Ed25519.sign(sk, BCS(U, nonce, C, AD))
//! ct  = (vk, U, nonce, C, sig)
//! ```
//!
//! The associated data `AD` is built by the contract and is not part of the wire format. It is
//! bound by the signature, by the KDF input and by the AEAD tag. The wire format is the BCS of
//! `(vk, U, nonce, C, sig)`, each a byte vector, with exact length checks on parse.
//!
//! The signature message `BCS(U, nonce, C, AD)` encodes each of the four items as a BCS byte
//! vector (ULEB128 length then bytes).

use super::{
    decapsulate, decode_g2, encapsulate_with_scalar, encode_g2, random_scalar, IbeError,
    IBE_KDF_SALT, IDENTITY_LEN,
};
use aes_gcm::{
    aead::{Aead, Payload},
    Aes128Gcm, Nonce,
};
use ark_bls12_381::{Bls12_381, G1Affine, G2Affine};
use ark_ec::pairing::PairingOutput;
use ark_serialize::CanonicalSerialize;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2_0_10_6::Sha256;
use sha3::{Digest, Sha3_256};

/// Ed25519 verifying key length.
pub const VK_LEN: usize = 32;
/// AES-GCM nonce length.
pub const NONCE_LEN: usize = 12;
/// AES-GCM tag length. A body shorter than this cannot be valid.
pub const TAG_LEN: usize = 16;
/// Ed25519 signature length.
pub const SIG_LEN: usize = 64;
/// Upper bound on the serialized ciphertext accepted by [`BidCiphertext::from_bytes`].
pub const MAX_CIPHERTEXT_BYTES: usize = 1 << 20;

const KDF_INFO: &[u8] = b"ATOMICA_TIMELOCK_BID_V2_AES128GCM";
const CT_ID_TAG: &[u8] = b"ATOMICA_TIMELOCK_BID_V2_CTID";

/// A signed, encrypted bid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BidCiphertext {
    /// One-time Ed25519 verifying key.
    pub vk: [u8; VK_LEN],
    /// KEM element `U = g2^r`, compressed G2.
    pub u: [u8; super::G2_COMPRESSED_LEN],
    /// AES-GCM nonce.
    pub nonce: [u8; NONCE_LEN],
    /// AES-GCM ciphertext with tag.
    pub c: Vec<u8>,
    /// Signature over `BCS(U, nonce, C, AD)`.
    pub sig: [u8; SIG_LEN],
}

#[derive(Serialize, Deserialize)]
struct Wire {
    #[serde(with = "serde_bytes")]
    vk: Vec<u8>,
    #[serde(with = "serde_bytes")]
    u: Vec<u8>,
    #[serde(with = "serde_bytes")]
    nonce: Vec<u8>,
    #[serde(with = "serde_bytes")]
    c: Vec<u8>,
    #[serde(with = "serde_bytes")]
    sig: Vec<u8>,
}

fn fixed<const N: usize>(what: &'static str, v: &[u8]) -> Result<[u8; N], IbeError> {
    v.try_into().map_err(|_| IbeError::InvalidLength {
        what,
        expected: N,
        got: v.len(),
    })
}

impl BidCiphertext {
    /// Serializes to the wire format.
    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&Wire {
            vk: self.vk.to_vec(),
            u: self.u.to_vec(),
            nonce: self.nonce.to_vec(),
            c: self.c.clone(),
            sig: self.sig.to_vec(),
        })
        .expect("bcs of byte vectors cannot fail")
    }

    /// Parses the wire format. Checks every length. Does not check the group element.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IbeError> {
        if bytes.len() > MAX_CIPHERTEXT_BYTES {
            return Err(IbeError::MalformedCiphertext("ciphertext too large".into()));
        }
        let w: Wire =
            bcs::from_bytes(bytes).map_err(|e| IbeError::MalformedCiphertext(e.to_string()))?;
        if w.c.len() < TAG_LEN {
            return Err(IbeError::MalformedCiphertext(
                "body shorter than the AEAD tag".into(),
            ));
        }
        Ok(Self {
            vk: fixed("vk", &w.vk)?,
            u: fixed("u", &w.u)?,
            nonce: fixed("nonce", &w.nonce)?,
            c: w.c,
            sig: fixed("sig", &w.sig)?,
        })
    }

    /// Identifier of this ciphertext for duplicate rejection: `SHA3-256(tag || vk || SHA3-256(AD))`.
    pub fn id(&self, ad: &[u8]) -> [u8; 32] {
        let mut h = Sha3_256::new();
        h.update(CT_ID_TAG);
        h.update(self.vk);
        h.update(Sha3_256::digest(ad));
        h.finalize().into()
    }

    pub(crate) fn signed_message(&self, ad: &[u8]) -> Vec<u8> {
        bcs::to_bytes(&(&self.u[..], &self.nonce[..], &self.c[..], ad))
            .expect("bcs of byte slices cannot fail")
    }

    /// Submission check (`ibe::verify_bid_ct`): `U` is a valid prime-order non-identity G2 point,
    /// and the one-time signature over `BCS(U, nonce, C, AD)` verifies for `vk`.
    ///
    /// `ad` is the associated data the contract built. Uses strict Ed25519 verification, which
    /// also rejects small-order keys and non-canonical signatures.
    pub fn verify(&self, ad: &[u8]) -> Result<(), IbeError> {
        let _validated_u = decode_g2(&self.u)?;
        let vk = VerifyingKey::from_bytes(&self.vk).map_err(|_| IbeError::BadSignature)?;
        let sig = Signature::from_bytes(&self.sig);
        vk.verify_strict(&self.signed_message(ad), &sig)
            .map_err(|_| IbeError::BadSignature)
    }
}

fn derive_aead_key(
    k: &PairingOutput<Bls12_381>,
    u: &[u8],
    id: &[u8; IDENTITY_LEN],
    vk: &[u8],
    ad: &[u8],
) -> [u8; 16] {
    let mut ikm = Vec::with_capacity(576 + u.len() + IDENTITY_LEN + vk.len() + 32);
    k.0.serialize_compressed(&mut ikm)
        .expect("writing to a vec cannot fail");
    ikm.extend_from_slice(u);
    ikm.extend_from_slice(id);
    ikm.extend_from_slice(vk);
    ikm.extend_from_slice(&Sha3_256::digest(ad));

    // HKDF-SHA256 (RFC 5869). Extract, then one expand block.
    let mut extract: Hmac<Sha256> =
        <Hmac<Sha256> as Mac>::new_from_slice(IBE_KDF_SALT).expect("HMAC takes a key of any size");
    extract.update(&ikm);
    let prk = extract.finalize().into_bytes();
    let mut expand: Hmac<Sha256> =
        <Hmac<Sha256> as Mac>::new_from_slice(&prk).expect("HMAC takes a key of any size");
    expand.update(KDF_INFO);
    expand.update(&[0x01]);
    let okm = expand.finalize().into_bytes();
    let mut key = [0u8; 16];
    key.copy_from_slice(&okm[..16]);
    key
}

/// Encrypts a bid to identity `id` under `mpk`. `mpk` must come from [`decode_g2`].
pub fn encrypt_bid<R: rand::Rng>(
    mpk: &G2Affine,
    id: &[u8; IDENTITY_LEN],
    ad: &[u8],
    msg: &[u8],
    rng: &mut R,
) -> Result<BidCiphertext, IbeError> {
    let mut sk_bytes = [0u8; 32];
    rng.fill(&mut sk_bytes[..]);
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    sk_bytes.fill(0);
    let vk = signing_key.verifying_key().to_bytes();

    let r = random_scalar(rng);
    let (u_point, k) = encapsulate_with_scalar(mpk, id, &r)?;
    let u = encode_g2(&u_point);
    let key = derive_aead_key(&k, &u, id, &vk, ad);

    let mut nonce = [0u8; NONCE_LEN];
    rng.fill(&mut nonce[..]);
    let cipher = <Aes128Gcm as aes_gcm::KeyInit>::new((&key).into());
    let c = cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg, aad: ad })
        .map_err(|_| IbeError::EncryptionFailed)?;

    let mut ct = BidCiphertext {
        vk,
        u,
        nonce,
        c,
        sig: [0u8; SIG_LEN],
    };
    ct.sig = signing_key.sign(&ct.signed_message(ad)).to_bytes();
    Ok(ct)
}

/// Opens a bid with the revealed key `dk` for identity `id`.
///
/// Checks the submission conditions first ([`BidCiphertext::verify`]), then decapsulates and
/// decrypts. Fails alone: it touches no other bid.
pub fn decrypt_bid(
    dk: &G1Affine,
    id: &[u8; IDENTITY_LEN],
    ad: &[u8],
    ct: &BidCiphertext,
) -> Result<Vec<u8>, IbeError> {
    ct.verify(ad)?;
    let u = decode_g2(&ct.u)?;
    let key = derive_aead_key(&decapsulate(dk, &u), &ct.u, id, &ct.vk, ad);
    <Aes128Gcm as aes_gcm::KeyInit>::new((&key).into())
        .decrypt(Nonce::from_slice(&ct.nonce), Payload {
            msg: &ct.c,
            aad: ad,
        })
        .map_err(|_| IbeError::DecryptionFailed)
}

/// Native entry: parse and verify serialized `ct` against `ad`.
pub fn verify_bid_ct(ct: &[u8], ad: &[u8]) -> Result<(), IbeError> {
    BidCiphertext::from_bytes(ct)?.verify(ad)
}
