//! Option C prototype: FPTX batch threshold encryption for auction bids.
//!
//! Spec: `atomica/docs/decisions/aptos-upgrade-option-c-spec.md`. Everything here
//! runs outside consensus. It wraps `aptos-batch-encryption` (`FPTXWeighted`,
//! upstream 1.49.6) in the flow a timelock reveal uses:
//!
//! 1. A bidder encrypts a [`Bid`] under the epoch key, bound to [`BidAd`].
//! 2. The auction contract calls [`verify_bid`] and enforces the batch cap.
//! 3. At the deadline the batch is frozen and [`Reveal::new`] derives the digest.
//! 4. Each validator derives a share. The handler runs [`Reveal::verify_share`].
//! 5. After a threshold of verified shares, [`Reveal::combine`] rebuilds the key
//!    and checks it against the epoch public key. [`Reveal::decrypt_all`] opens bids.

use anyhow::{anyhow, Result};
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted,
    traits::{AssociatedData, BatchThresholdEncryption, DecryptionKeyShare, Plaintext},
};
use aptos_crypto::weighted_config::WeightedConfigArkworks;
use ark_std::rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

pub use aptos_batch_encryption::group::Fr;

/// Spec section 5: bids per timelock. A larger auction is split into shards.
pub const MAX_BATCH_SIZE: usize = 256;
/// Spec section 4: rounds in one epoch's digest key. One round per reveal.
pub const NUM_ROUNDS: usize = 256;
/// Spec section 7: domain tag inside the associated data.
pub const AD_TAG: &str = "atomica/timelock-bid/v1";

pub type Scheme = FPTXWeighted;
pub type Ciphertext = <Scheme as BatchThresholdEncryption>::Ciphertext;
pub type Digest = <Scheme as BatchThresholdEncryption>::Digest;
pub type Share = <Scheme as BatchThresholdEncryption>::DecryptionKeyShare;
pub type DecryptionKey = <Scheme as BatchThresholdEncryption>::DecryptionKey;

#[derive(Debug, thiserror::Error)]
pub enum TimelockError {
    #[error("batch is full: {0} bids (cap {MAX_BATCH_SIZE})")]
    BatchFull(usize),
    #[error("duplicate ciphertext id in batch")]
    DuplicateBid,
    #[error("ciphertext failed verification: {0}")]
    BadCiphertext(String),
    #[error("round {0} is not below num_rounds")]
    RoundExhausted(u64),
    #[error("decryption key share rejected: {0}")]
    BadShare(String),
    #[error("share from player {0} does not come from a unique validator")]
    DuplicateShare(usize),
    #[error("reconstruction failed: {0}")]
    Reconstruct(String),
    #[error("reconstructed key does not match the epoch public key: {0}")]
    BadKey(String),
}

/// Associated data of a bid (spec section 7). BCS encoded by the library.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BidAd {
    pub tag: String,
    pub chain_id: u8,
    pub epoch: u64,
    pub auction_id: u64,
    pub shard: u16,
    pub bidder: [u8; 32],
    pub deadline_us: u64,
}

impl BidAd {
    pub fn new(
        chain_id: u8,
        epoch: u64,
        auction_id: u64,
        shard: u16,
        bidder: [u8; 32],
        deadline_us: u64,
    ) -> Self {
        Self {
            tag: AD_TAG.to_string(),
            chain_id,
            epoch,
            auction_id,
            shard,
            bidder,
            deadline_us,
        }
    }
}
impl AssociatedData for BidAd {}

/// The sealed content of a bid.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bid {
    pub price: u64,
    pub quantity: u64,
    pub salt: [u8; 16],
}
impl Plaintext for Bid {}

/// Test-only setup (single dealer, seeded). Production uses a chunky DKG session
/// and a trusted setup ceremony (spec section 6).
pub struct TimelockSetup {
    pub tc: WeightedConfigArkworks<Fr>,
    pub ek: <Scheme as BatchThresholdEncryption>::EncryptionKey,
    pub dk: <Scheme as BatchThresholdEncryption>::DigestKey,
    pub vks: Vec<<Scheme as BatchThresholdEncryption>::VerificationKey>,
    pub msk_shares: Vec<<Scheme as BatchThresholdEncryption>::MasterSecretKeyShare>,
}

impl TimelockSetup {
    /// `weights` are validator weights. `threshold` is the weight needed to reveal.
    pub fn for_testing(
        seed: u64,
        max_batch_size: usize,
        num_rounds: usize,
        threshold: usize,
        weights: Vec<usize>,
    ) -> Result<Self> {
        let tc = WeightedConfigArkworks::new(threshold, weights)?;
        let (ek, dk, vks, msk_shares) =
            Scheme::setup_for_testing(seed, max_batch_size, num_rounds, &tc)?;
        Ok(Self {
            tc,
            ek,
            dk,
            vks,
            msk_shares,
        })
    }
}

/// Bidder side.
pub fn encrypt_bid<R: CryptoRng + RngCore>(
    setup: &TimelockSetup,
    rng: &mut R,
    bid: &Bid,
    ad: &BidAd,
) -> Result<Ciphertext> {
    Scheme::encrypt(&setup.ek, rng, bid, ad)
}

/// Auction contract side. Must run before a ciphertext is stored.
pub fn verify_bid(ct: &Ciphertext, ad: &BidAd) -> Result<(), TimelockError> {
    Scheme::verify_ct(ct, ad).map_err(|e| TimelockError::BadCiphertext(e.to_string()))
}

/// Cap and duplicate checks the contract runs at submission (`E_BATCH_FULL`).
pub fn admit(batch: &[Ciphertext], new: &Ciphertext) -> Result<(), TimelockError> {
    if batch.len() >= MAX_BATCH_SIZE {
        return Err(TimelockError::BatchFull(batch.len() + 1));
    }
    let id = Scheme::ct_id(new);
    if batch.iter().any(|c| Scheme::ct_id(c) == id) {
        return Err(TimelockError::DuplicateBid);
    }
    Ok(())
}

/// One reveal: a frozen batch bound to one round and one digest.
pub struct Reveal<'a> {
    setup: &'a TimelockSetup,
    pub round: u64,
    pub cts: Vec<Ciphertext>,
    pub digest: Digest,
    eval_proofs: <Scheme as BatchThresholdEncryption>::EvalProofs,
}

impl<'a> Reveal<'a> {
    /// Freeze the batch and derive its digest. Computes the eval proofs.
    pub fn new(setup: &'a TimelockSetup, cts: Vec<Ciphertext>, round: u64) -> Result<Self> {
        if cts.len() > MAX_BATCH_SIZE {
            return Err(TimelockError::BatchFull(cts.len()).into());
        }
        if round >= Scheme::num_rounds(&setup.dk) as u64 {
            return Err(TimelockError::RoundExhausted(round).into());
        }
        let (digest, promise) = Scheme::digest(&setup.dk, &cts, round)?;
        let eval_proofs = Scheme::eval_proofs_compute_all(&promise, &setup.dk);
        Ok(Self {
            setup,
            round,
            cts,
            digest,
            eval_proofs,
        })
    }

    /// What validator `i` does. In production each node holds one share only.
    pub fn derive_share(&self, validator: usize) -> Result<Share> {
        Scheme::derive_decryption_key_share(&self.setup.msk_shares[validator], &self.digest)
    }

    /// What the validator transaction handler does on every received share.
    pub fn verify_share(&self, validator: usize, share: &Share) -> Result<(), TimelockError> {
        if share.player().get_id() != validator {
            return Err(TimelockError::BadShare("player id mismatch".into()));
        }
        Scheme::verify_decryption_key_share(&self.setup.vks[validator], &self.digest, share)
            .map_err(|e| TimelockError::BadShare(e.to_string()))
    }

    /// Rebuild the key from verified shares, then check it against the epoch key.
    /// Nothing is returned unless both steps pass.
    pub fn combine(&self, shares: &[Share]) -> Result<DecryptionKey, TimelockError> {
        let mut seen = std::collections::HashSet::new();
        for s in shares {
            if !seen.insert(s.player().get_id()) {
                return Err(TimelockError::DuplicateShare(s.player().get_id()));
            }
        }
        let key = Scheme::reconstruct_decryption_key(shares, &self.setup.tc)
            .map_err(|e| TimelockError::Reconstruct(e.to_string()))?;
        self.verify_key(&key)?;
        Ok(key)
    }

    /// Check a key against the epoch public key and this reveal's digest.
    pub fn verify_key(&self, key: &DecryptionKey) -> Result<(), TimelockError> {
        Scheme::verify_decryption_key(&self.setup.ek, &self.digest, key)
            .map_err(|e| TimelockError::BadKey(e.to_string()))
    }

    /// Decrypt every bid in batch order.
    pub fn decrypt_all(&self, key: &DecryptionKey) -> Result<Vec<Bid>> {
        self.cts
            .iter()
            .map(|ct| {
                let prepared = Scheme::prepare_ct(ct, &self.digest, &self.eval_proofs)
                    .map_err(|e| anyhow!("{e}"))?;
                Scheme::decrypt::<Bid>(key, &prepared)
            })
            .collect()
    }
}
