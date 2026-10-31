// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The timelock chunky DKG session (fork only).
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` section 4. Each epoch runs a second
//! chunky DKG session for the timelock secret. It has the same dealers and the same target validator set
//! as the mempool session. It differs in four things, so that nothing a validator signs or deals for one
//! session is valid for the other:
//!
//! 1. the session id (`TimelockDKGSessionId`, see `chunky_dkg.rs`), bound into every dealer proof,
//! 2. the signed message of the subtranscript certificate ([`TimelockSubtranscriptSigningMessage`]),
//!    which is a different hash domain from the mempool's signed `AggregatedSubtranscript`,
//! 3. the wire request kind ([`ChunkyDKGSessionKind`]), which routes the request to the right manager,
//! 4. the on-chain state and the validator transaction (`TimelockDKGResult`).
//!
//! The Boneh-Franklin key material is read from the aggregated subtranscript without reconstructing the
//! secret: `mpk` is the dealt public key, a verification key is the public key share of one virtual
//! player, and a validator's secret shares are decrypted with its consensus key.

use crate::{
    aggregate_signature::AggregateSignature,
    chain_id::ChainId,
    dkg::chunky_dkg::{
        chunky_threshold_config, AggregatedSubtranscript, CertifiedAggregatedChunkySubtranscript,
        ChunkyDKGPublicParameters, ChunkyDKGSessionMetadata, ChunkyDKGThresholdConfig,
        ChunkyDecryptPrivKey, ChunkySubtranscript,
    },
    on_chain_config::OnChainConfig,
    validator_verifier::ValidatorVerifier,
};
use anyhow::{anyhow, ensure, Context, Result};
use aptos_crypto::{
    bls12381, hash::CryptoHash, HashValue, Signature as _, SigningKey, TSecretSharingConfig,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    ibe::{
        check_mpk_matches_verification_keys, decode_g2, encode_g2, IbeError, MasterScalar,
        G2_COMPRESSED_LEN,
    },
    pvss::{traits::TranscriptCore, Player},
};
use ark_ec::{AffineRepr, CurveGroup};
use move_core_types::{
    ident_str, identifier::IdentStr, language_storage::TypeTag, move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

/// Seed of the built-in test parameters of the timelock session. Differs from the mempool test seed.
pub const TEST_TIMELOCK_PUBLIC_PARAMETERS_SEED: u64 = 300;

/// Domain tag of the signed message of a timelock subtranscript certificate.
pub const TIMELOCK_SIGNING_TAG: &str = "atomica/timelock-chunky-dkg-certificate/v1";

/// Which of the two chunky DKG sessions of an epoch a request, a dealing or a signature belongs to.
///
/// The mempool session is the upstream session. The timelock session is the fork's second session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ChunkyDKGSessionKind {
    Mempool,
    Timelock,
}

/// What a validator signs to certify an aggregated subtranscript of the timelock session.
///
/// The mempool session signs the `AggregatedSubtranscript` itself. The timelock session signs this
/// message instead. The two have different `CryptoHasher` domains, so a certificate of one session
/// never verifies as a certificate of the other, even for the same subtranscript bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct TimelockSubtranscriptSigningMessage {
    pub tag: String,
    pub chain_id: ChainId,
    pub dealer_epoch: u64,
    /// Hash of the `AggregatedSubtranscript`.
    pub subtranscript_hash: HashValue,
}

impl TimelockSubtranscriptSigningMessage {
    pub fn new(chain_id: ChainId, aggregated: &AggregatedSubtranscript) -> Self {
        Self {
            tag: TIMELOCK_SIGNING_TAG.to_string(),
            chain_id,
            dealer_epoch: aggregated.dealer_epoch,
            subtranscript_hash: aggregated.hash(),
        }
    }
}

impl ChunkyDKGSessionKind {
    /// Signs `aggregated` in the hash domain of the session. `chain_id` is only used by the timelock domain.
    pub fn sign_subtranscript(
        self,
        chain_id: ChainId,
        ssk: &bls12381::PrivateKey,
        aggregated: &AggregatedSubtranscript,
    ) -> Result<bls12381::Signature> {
        match self {
            Self::Mempool => ssk
                .sign(aggregated)
                .map_err(|e| anyhow!("failed to sign subtranscript: {:?}", e)),
            Self::Timelock => ssk
                .sign(&TimelockSubtranscriptSigningMessage::new(
                    chain_id, aggregated,
                ))
                .map_err(|e| anyhow!("failed to sign timelock subtranscript: {:?}", e)),
        }
    }

    /// Verifies one signature in the hash domain of the session.
    pub fn verify_subtranscript_signature(
        self,
        chain_id: ChainId,
        aggregated: &AggregatedSubtranscript,
        signature: &bls12381::Signature,
        public_key: &bls12381::PublicKey,
    ) -> Result<()> {
        match self {
            Self::Mempool => signature.verify(aggregated, public_key),
            Self::Timelock => signature.verify(
                &TimelockSubtranscriptSigningMessage::new(chain_id, aggregated),
                public_key,
            ),
        }
    }

    /// Verifies the aggregate (multi) signature of a certified subtranscript in the hash domain of the session.
    pub fn verify_multi_signatures(
        self,
        chain_id: ChainId,
        verifier: &ValidatorVerifier,
        aggregated: &AggregatedSubtranscript,
        signature: &AggregateSignature,
    ) -> Result<()> {
        match self {
            Self::Mempool => verifier.verify_multi_signatures(aggregated, signature),
            Self::Timelock => verifier.verify_multi_signatures(
                &TimelockSubtranscriptSigningMessage::new(chain_id, aggregated),
                signature,
            ),
        }
        .map_err(|e| anyhow!("multi-signature verification failed: {:?}", e))
    }
}

/// Output of the timelock chunky DKG session, carried by the `TimelockDKGResult` validator transaction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertifiedTimelockDKGOutput {
    pub certified_transcript: CertifiedAggregatedChunkySubtranscript,
    /// The claimed master public key (compressed G2). The handler rederives it and compares.
    #[serde(with = "serde_bytes")]
    pub mpk: Vec<u8>,
    /// SHA3-256 of the timelock `PublicParameters` blob the dealers used.
    pub public_parameters_hash: HashValue,
}

// ---------------------------------------------------------------------------------------------
// On-chain state reflection
// ---------------------------------------------------------------------------------------------

/// Reflection of Move type `0x1::timelock_dkg::TimelockDKGSessionState`. The metadata has the layout of
/// `ChunkyDKGSessionMetadata`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelockDKGSessionState {
    pub metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
    pub transcript: Vec<u8>,
}

impl TimelockDKGSessionState {
    pub fn target_epoch(&self) -> u64 {
        self.metadata.dealer_epoch + 1
    }
}

/// Reflection of Move type `0x1::timelock_dkg::PendingTimelockConfig`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingTimelockConfig {
    pub target_epoch: u64,
    pub mpk: Vec<u8>,
    pub reconstruct_threshold_weight: u64,
    pub weights: Vec<u64>,
    pub epoch_interval_us: u64,
    pub public_parameters_hash: Vec<u8>,
    pub subtranscript_hash: Vec<u8>,
    pub verification_keys: Vec<Vec<u8>>,
}

/// Reflection of Move type `0x1::timelock_dkg::TimelockDKGState`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelockDKGState {
    pub enabled: bool,
    pub grace_period_secs: u64,
    pub last_completed: Option<TimelockDKGSessionState>,
    pub in_progress: Option<TimelockDKGSessionState>,
    pub pending_config: Option<PendingTimelockConfig>,
}

impl TimelockDKGState {
    pub fn maybe_last_complete(&self, epoch: u64) -> Option<&TimelockDKGSessionState> {
        match &self.last_completed {
            Some(session) if session.target_epoch() == epoch => Some(session),
            _ => None,
        }
    }
}

impl OnChainConfig for TimelockDKGState {
    const MODULE_IDENTIFIER: &'static str = "timelock_dkg";
    const TYPE_IDENTIFIER: &'static str = "TimelockDKGState";
}

/// Reflection of `0x1::timelock_dkg::TimelockDKGStartEvent`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockDKGStartEvent {
    pub session_metadata: ChunkyDKGSessionMetadata,
    pub start_time_us: u64,
}

impl MoveStructType for TimelockDKGStartEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("timelock_dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TimelockDKGStartEvent");
}

pub static TIMELOCK_DKG_START_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(TimelockDKGStartEvent::struct_tag())));

// ---------------------------------------------------------------------------------------------
// Public parameters blob of the timelock session
// ---------------------------------------------------------------------------------------------

/// The timelock `PublicParameters`, loaded with full validation, with the hash of the blob they came from.
#[derive(Debug)]
pub struct TimelockPublicParameters {
    pub pp: Arc<ChunkyDKGPublicParameters>,
    /// SHA3-256 of the blob bytes. Stored on chain as `TimelockEpochConfig.public_parameters_hash`.
    pub blob_hash: HashValue,
}

/// Hash of a parameters blob. The blake3 file metric of the mempool `DigestKey` is not used.
pub fn timelock_public_parameters_hash(blob: &[u8]) -> HashValue {
    HashValue::sha3_256_of(blob)
}

/// Loads a timelock `PublicParameters` blob with validated deserialization.
///
/// The plain BCS decoder checks only the encoding. This function then validates every point (on curve,
/// prime-order subgroup, not the identity), requires the `G_2` commitment base to be the standard
/// BLS12-381 G2 generator, and checks the dlog table. A blob with one corrupted point is refused.
pub fn load_timelock_public_parameters(blob: &[u8]) -> Result<TimelockPublicParameters> {
    let pp: ChunkyDKGPublicParameters =
        bcs::from_bytes(blob).context("timelock PublicParameters blob does not decode")?;
    pp.validate(true)
        .context("timelock PublicParameters blob failed validation")?;
    Ok(TimelockPublicParameters {
        pp: Arc::new(pp),
        blob_hash: timelock_public_parameters_hash(blob),
    })
}

/// Compares the hash of a blob with the hash stored on chain in `TimelockEpochConfig`. The node refuses
/// to start the timelock service on a mismatch.
pub fn check_blob_hash_against_config(
    params: &TimelockPublicParameters,
    on_chain_hash: &[u8],
) -> Result<()> {
    ensure!(
        params.blob_hash.as_ref() == on_chain_hash,
        "timelock PublicParameters blob hash {} differs from the on-chain hash {}",
        params.blob_hash.to_hex(),
        hex::encode(on_chain_hash)
    );
    Ok(())
}

/// What the validation command reports.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct TimelockParametersReport {
    pub blob_bytes: usize,
    pub sha3_256: String,
    /// Whether the hash was compared with an expected value, and it matched.
    pub hash_matches_expected: Option<bool>,
}

/// The honest validation: the real load (validated points, standard G2 base, dlog table) and, when given,
/// the comparison with the expected hash. Anything that reports the parameters as valid must call this.
pub fn validate_timelock_public_parameters_blob(
    blob: &[u8],
    expected_hash: Option<&[u8]>,
) -> Result<TimelockParametersReport> {
    let params = load_timelock_public_parameters(blob)?;
    let hash_matches_expected = match expected_hash {
        Some(expected) => {
            check_blob_hash_against_config(&params, expected)?;
            Some(true)
        },
        None => None,
    };
    Ok(TimelockParametersReport {
        blob_bytes: blob.len(),
        sha3_256: params.blob_hash.to_hex(),
        hash_matches_expected,
    })
}

static TIMELOCK_PUBLIC_PARAMETERS_PATH: OnceLock<PathBuf> = OnceLock::new();
static TIMELOCK_PUBLIC_PARAMETERS_TEST_FALLBACK: OnceLock<()> = OnceLock::new();

/// Stores the path of the timelock `PublicParameters` blob. No I/O is performed.
pub fn set_timelock_public_parameters_path(path: PathBuf) {
    TIMELOCK_PUBLIC_PARAMETERS_PATH
        .set(path)
        .expect("timelock PublicParameters path already set");
}

/// Where the timelock parameters come from on this node.
#[derive(Debug)]
pub enum TimelockParametersSource {
    WillLoadFromFile {
        path: PathBuf,
        file_size: u64,
    },
    /// Built-in test parameters (test chain only).
    TestKeyFallback,
    /// No parameters: this node cannot take part in the timelock session.
    NotAvailable,
}

pub fn initialize_timelock_public_parameters(chain_id: ChainId) -> TimelockParametersSource {
    if let Some(path) = TIMELOCK_PUBLIC_PARAMETERS_PATH.get() {
        match std::fs::metadata(path) {
            Ok(meta) => TimelockParametersSource::WillLoadFromFile {
                path: path.clone(),
                file_size: meta.len(),
            },
            Err(_) => TimelockParametersSource::NotAvailable,
        }
    } else if chain_id == ChainId::test() {
        let _ = TIMELOCK_PUBLIC_PARAMETERS_TEST_FALLBACK.set(());
        TimelockParametersSource::TestKeyFallback
    } else {
        TimelockParametersSource::NotAvailable
    }
}

/// Test parameters of the timelock session. Own randomness (own seed), not the mempool test parameters.
pub static TEST_TIMELOCK_PUBLIC_PARAMETERS: Lazy<Arc<TimelockPublicParameters>> = Lazy::new(|| {
    use aptos_batch_encryption::group::G2Affine;
    use rand::{rngs::StdRng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(TEST_TIMELOCK_PUBLIC_PARAMETERS_SEED);
    let pp = ChunkyDKGPublicParameters::new_for_testing(
        72,
        aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_DEPLOYMENT,
        20,
        G2Affine::generator(),
        &mut rng,
    );
    let blob = bcs::to_bytes(&pp).expect("test parameters serialize");
    Arc::new(TimelockPublicParameters {
        pp: Arc::new(pp),
        blob_hash: timelock_public_parameters_hash(&blob),
    })
});

/// The timelock parameters of this node, loaded and validated on first access. `None` when the node has
/// none or the blob is refused. The error is logged once.
pub static TIMELOCK_PUBLIC_PARAMETERS: Lazy<Option<Arc<TimelockPublicParameters>>> =
    Lazy::new(|| {
        if let Some(path) = TIMELOCK_PUBLIC_PARAMETERS_PATH.get() {
            let blob = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!(
                        "[TimelockParameters] cannot read blob {}: {}",
                        path.display(),
                        e
                    );
                    return None;
                },
            };
            return match load_timelock_public_parameters(&blob) {
                Ok(p) => {
                    tracing::info!(
                        "[TimelockParameters] loaded and validated {} ({} bytes), sha3-256 {}",
                        path.display(),
                        blob.len(),
                        p.blob_hash.to_hex()
                    );
                    Some(Arc::new(p))
                },
                Err(e) => {
                    tracing::error!(
                        "[TimelockParameters] blob {} refused: {:#}",
                        path.display(),
                        e
                    );
                    None
                },
            };
        }
        if TIMELOCK_PUBLIC_PARAMETERS_TEST_FALLBACK.get().is_some() {
            return Some(Arc::clone(&TEST_TIMELOCK_PUBLIC_PARAMETERS));
        }
        None
    });

// ---------------------------------------------------------------------------------------------
// Boneh-Franklin key material from the subtranscript
// ---------------------------------------------------------------------------------------------

/// Why the key material of a timelock subtranscript is refused.
#[derive(Debug)]
pub enum TimelockKeyError {
    /// The shape of `Vs` does not match the weights.
    ShapeMismatch(String),
    /// A group element is not a valid point.
    InvalidPoint(String),
    /// `V0` is not the interpolation of the verification keys.
    MpkMismatch(IbeError),
    /// A decrypted share does not match its verification key. The node must not serve the timelock.
    ShareMismatch { virtual_player: usize },
    /// The claimed `mpk` differs from the rederived one.
    ClaimedMpkMismatch,
    /// Decrypting the shares failed (a wrong key or a malformed ciphertext). The upstream decryption
    /// panics in that case. The node catches it and refuses the timelock for the epoch.
    DecryptionFailed,
}

impl std::fmt::Display for TimelockKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TimelockKeyError {}

/// The `TimelockEpochConfig` data derived from an accepted subtranscript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockDerivedConfig {
    /// `V0` (compressed G2, 96 bytes).
    pub mpk: Vec<u8>,
    /// Verification key of every virtual player, in player order (compressed G2, 96 bytes).
    pub verification_keys: Vec<Vec<u8>>,
    pub reconstruct_threshold_weight: u64,
    /// Per-validator weights from the rounding profile.
    pub weights: Vec<u64>,
}

/// Rederives `mpk` and the verification keys from the subtranscript, with every point validated, and checks
/// that `V0` equals the G2 interpolation of threshold sets of the verification keys.
///
/// `mpk = subtranscript.get_dealt_public_key().as_g2()`. A verification key is `get_public_key_share` of one
/// virtual player. The master secret is not reconstructed.
pub fn derive_timelock_config(
    subtranscript: &ChunkySubtranscript,
    threshold_config: &ChunkyDKGThresholdConfig,
) -> Result<TimelockDerivedConfig, TimelockKeyError> {
    let n = threshold_config.get_total_num_players();
    let total = threshold_config.get_total_weight();
    if subtranscript.Vs.len() != n {
        return Err(TimelockKeyError::ShapeMismatch(format!(
            "Vs has {} players, expected {}",
            subtranscript.Vs.len(),
            n
        )));
    }
    let mut weights = Vec::with_capacity(n);
    let mut verification_keys = Vec::with_capacity(total);
    let mut points = Vec::with_capacity(total);
    for id in 0..n {
        let player = Player { id };
        let weight = threshold_config
            .get_player_weight(&player)
            .map_err(|e| TimelockKeyError::ShapeMismatch(e.to_string()))?;
        if subtranscript.Vs[id].len() != weight {
            return Err(TimelockKeyError::ShapeMismatch(format!(
                "Vs[{}] has {} keys, expected weight {}",
                id,
                subtranscript.Vs[id].len(),
                weight
            )));
        }
        weights.push(weight as u64);
        for share in subtranscript.get_public_key_share(threshold_config, &player) {
            let bytes = encode_g2(&share.as_g2());
            // Validated decode: on curve, in the prime-order subgroup, not the identity.
            let point = decode_g2(&bytes)
                .map_err(|e| TimelockKeyError::InvalidPoint(format!("vk: {e}")))?;
            verification_keys.push(bytes.to_vec());
            points.push(point);
        }
    }
    let mpk_bytes = encode_g2(&subtranscript.get_dealt_public_key().as_g2());
    let mpk =
        decode_g2(&mpk_bytes).map_err(|e| TimelockKeyError::InvalidPoint(format!("mpk: {e}")))?;
    check_mpk_matches_verification_keys(&mpk, &points, threshold_config.get_threshold_weight())
        .map_err(TimelockKeyError::MpkMismatch)?;
    debug_assert!(verification_keys
        .iter()
        .all(|k| k.len() == G2_COMPRESSED_LEN));
    Ok(TimelockDerivedConfig {
        mpk: mpk_bytes.to_vec(),
        verification_keys,
        reconstruct_threshold_weight: threshold_config.get_threshold_weight() as u64,
        weights,
    })
}

/// A validator's secret shares of the timelock master secret, one per virtual player, derived from the
/// on-chain subtranscript and its consensus key. Never persisted.
pub struct TimelockOwnShares {
    shares: Vec<MasterScalar>,
}

impl TimelockOwnShares {
    pub fn shares(&self) -> &[MasterScalar] {
        &self.shares
    }
}

impl std::fmt::Debug for TimelockOwnShares {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelockOwnShares")
            .field("shares", &"<redacted>")
            .finish()
    }
}

/// Decrypts the shares of `player` with `dk` (the consensus key) and checks `vk_j == g2 * share_j` for each
/// virtual player of the validator. On any mismatch the node refuses to serve the timelock for the epoch.
pub fn derive_own_shares(
    subtranscript: &ChunkySubtranscript,
    threshold_config: &ChunkyDKGThresholdConfig,
    player: &Player,
    dk: &ChunkyDecryptPrivKey,
    pp: &ChunkyDKGPublicParameters,
) -> Result<TimelockOwnShares, TimelockKeyError> {
    // `decrypt_own_share` panics ("dlog_vec failed") when the key does not decrypt a ciphertext. A node
    // must not die on data from chain state, so the panic becomes an error.
    let (sk_shares, pk_shares) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        subtranscript.decrypt_own_share(threshold_config, player, dk, pp)
    }))
    .map_err(|_| TimelockKeyError::DecryptionFailed)?;
    if sk_shares.len() != pk_shares.len() {
        return Err(TimelockKeyError::ShapeMismatch(
            "share and key count differ".to_string(),
        ));
    }
    let start = threshold_config.get_player_starting_index(player);
    let g2 = aptos_batch_encryption::group::G2Affine::generator();
    let mut shares = Vec::with_capacity(sk_shares.len());
    for (j, (sk, pk)) in sk_shares.iter().zip(pk_shares.iter()).enumerate() {
        if (g2 * sk.0).into_affine() != pk.as_g2() {
            return Err(TimelockKeyError::ShareMismatch {
                virtual_player: start + j,
            });
        }
        shares.push(sk.0);
    }
    Ok(TimelockOwnShares { shares })
}

/// What the `TimelockDKGResult` handler computes from a certified result: the checks of spec section 4.
/// `dealer_epoch` of the aggregate and the metadata epoch are checked by the caller.
pub fn check_claimed_mpk(
    derived: &TimelockDerivedConfig,
    claimed_mpk: &[u8],
) -> Result<(), TimelockKeyError> {
    if derived.mpk.as_slice() != claimed_mpk {
        return Err(TimelockKeyError::ClaimedMpkMismatch);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Acceptance of a certified result (the check of the `TimelockDKGResult` handler)
// ---------------------------------------------------------------------------------------------

/// Why a certified timelock DKG result is refused. The transaction is discarded and nothing changes.
#[derive(Debug, PartialEq, Eq)]
pub enum ResultRejection {
    EpochNotCurrent,
    NotEnoughVotingPower,
    TranscriptDeserializationFailed,
    MultiSigVerificationFailed,
    /// The shape or a point of the subtranscript is invalid, or `V0` is not the interpolation of the
    /// verification keys.
    KeyDerivationFailed(String),
    ClaimedMpkMismatch,
    ParametersHashMismatch,
}

/// A result that passed every check, with the values the Move side stores.
#[derive(Debug)]
pub struct CheckedTimelockResult {
    pub aggregated: AggregatedSubtranscript,
    pub derived: TimelockDerivedConfig,
    pub subtranscript_hash: HashValue,
}

/// All checks of a certified timelock DKG result against chain state. No VM, no I/O.
///
/// `local_params_hash` is the hash of this node's validated timelock parameters blob, `None` on a node
/// without it (a fullnode). Like the mempool handler, such a node trusts consensus for that one check.
pub fn check_timelock_dkg_result(
    output: &CertifiedTimelockDKGOutput,
    current_epoch: u64,
    session_metadata: &ChunkyDKGSessionMetadata,
    chain_id: ChainId,
    verifier: &ValidatorVerifier,
    local_params_hash: Option<HashValue>,
) -> Result<CheckedTimelockResult, ResultRejection> {
    let certified = &output.certified_transcript;
    if certified.metadata.epoch != current_epoch || session_metadata.dealer_epoch != current_epoch {
        return Err(ResultRejection::EpochNotCurrent);
    }

    let authors = certified
        .signature
        .get_signers_addresses(&verifier.get_ordered_account_addresses());
    verifier
        .check_voting_power(authors.iter(), true)
        .map_err(|_| ResultRejection::NotEnoughVotingPower)?;

    let aggregated: AggregatedSubtranscript = bcs::from_bytes(&certified.transcript_bytes)
        .map_err(|_| ResultRejection::TranscriptDeserializationFailed)?;
    if aggregated.dealer_epoch != certified.metadata.epoch {
        return Err(ResultRejection::EpochNotCurrent);
    }

    // The timelock hash domain. A mempool certificate fails here.
    ChunkyDKGSessionKind::Timelock
        .verify_multi_signatures(chain_id, verifier, &aggregated, &certified.signature)
        .map_err(|_| ResultRejection::MultiSigVerificationFailed)?;

    // Rederive mpk and the verification keys from the subtranscript, check V0 against the
    // interpolated keys, then compare with the claimed mpk.
    let threshold_config = chunky_threshold_config(session_metadata);
    let derived = derive_timelock_config(&aggregated.subtranscript, &threshold_config)
        .map_err(|e| ResultRejection::KeyDerivationFailed(e.to_string()))?;
    check_claimed_mpk(&derived, &output.mpk).map_err(|_| ResultRejection::ClaimedMpkMismatch)?;

    if let Some(local) = local_params_hash
        && local != output.public_parameters_hash
    {
        return Err(ResultRejection::ParametersHashMismatch);
    }

    let subtranscript_hash = aggregated.hash();
    Ok(CheckedTimelockResult {
        aggregated,
        derived,
        subtranscript_hash,
    })
}

// ---------------------------------------------------------------------------------------------
// Node side: the timelock service of one epoch
// ---------------------------------------------------------------------------------------------

/// Reflection of Move type `0x1::ibe_config::TimelockEpochConfig`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimelockEpochConfigResource {
    pub epoch: u64,
    pub mpk: Vec<u8>,
    pub reconstruct_threshold_weight: u64,
    pub total_weight: u64,
    pub weights: Vec<u64>,
    pub epoch_interval_us: u64,
    pub public_parameters_hash: Vec<u8>,
    pub subtranscript_hash: Vec<u8>,
    pub verification_keys: Vec<Vec<u8>>,
}

impl OnChainConfig for TimelockEpochConfigResource {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "TimelockEpochConfig";
}

/// Why the node does not serve the timelock in an epoch.
#[derive(Debug)]
pub enum TimelockServiceRefusal {
    /// No `TimelockEpochConfig` for this epoch: the timelock DKG did not complete.
    NoKeyThisEpoch,
    /// This node has no valid timelock `PublicParameters` blob.
    NoParameters,
    /// The blob hash differs from `TimelockEpochConfig.public_parameters_hash`.
    ParametersHashMismatch(String),
    /// The on-chain subtranscript is missing, or is not the one the config was derived from.
    SubtranscriptMismatch(String),
    /// The on-chain verification keys are not the ones the subtranscript gives.
    VerificationKeyMismatch,
    /// The node cannot derive or verify its shares. It must not serve the timelock.
    KeyDerivation(TimelockKeyError),
}

impl std::fmt::Display for TimelockServiceRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// The node's timelock state for one epoch.
#[derive(Debug)]
pub enum TimelockServiceState {
    /// The node serves the timelock: its shares match the verification keys.
    Ready {
        epoch: u64,
        shares: TimelockOwnShares,
    },
    /// The node does not serve the timelock in this epoch.
    Refused {
        epoch: u64,
        reason: TimelockServiceRefusal,
    },
}

impl TimelockServiceState {
    pub fn is_ready(&self) -> bool {
        matches!(self, TimelockServiceState::Ready { .. })
    }
}

/// Derives the timelock service state of an epoch from chain state and the node's keys. Shares are derived
/// again at every epoch start, as the mempool does, and are not persisted.
///
/// Order of the checks: a config exists for the epoch, the local blob matches the on-chain hash (the node
/// refuses to start the service otherwise), the on-chain subtranscript hashes to the config's subtranscript
/// hash and gives the on-chain verification keys, and every share decrypted with the consensus key matches
/// its verification key.
pub fn derive_timelock_service_state(
    epoch: u64,
    config: Option<&TimelockEpochConfigResource>,
    state: &TimelockDKGState,
    params: Option<&TimelockPublicParameters>,
    my_player: &Player,
    dk: &ChunkyDecryptPrivKey,
) -> TimelockServiceState {
    let refuse = |reason| TimelockServiceState::Refused { epoch, reason };
    let Some(config) = config.filter(|c| c.epoch == epoch) else {
        return refuse(TimelockServiceRefusal::NoKeyThisEpoch);
    };
    let Some(params) = params else {
        return refuse(TimelockServiceRefusal::NoParameters);
    };
    if let Err(e) = check_blob_hash_against_config(params, &config.public_parameters_hash) {
        return refuse(TimelockServiceRefusal::ParametersHashMismatch(
            e.to_string(),
        ));
    }
    let Some(session) = state.maybe_last_complete(epoch) else {
        return refuse(TimelockServiceRefusal::SubtranscriptMismatch(
            "no completed timelock session for this epoch".to_string(),
        ));
    };
    let aggregated: AggregatedSubtranscript = match bcs::from_bytes(&session.transcript) {
        Ok(a) => a,
        Err(e) => {
            return refuse(TimelockServiceRefusal::SubtranscriptMismatch(format!(
                "undecodable subtranscript: {e}"
            )))
        },
    };
    if aggregated.hash().as_ref() != config.subtranscript_hash.as_slice() {
        return refuse(TimelockServiceRefusal::SubtranscriptMismatch(
            "subtranscript hash differs from the epoch config".to_string(),
        ));
    }
    let threshold_config = chunky_threshold_config(&session.metadata);
    match derive_timelock_config(&aggregated.subtranscript, &threshold_config) {
        Ok(derived) => {
            if derived.verification_keys != config.verification_keys || derived.mpk != config.mpk {
                return refuse(TimelockServiceRefusal::VerificationKeyMismatch);
            }
        },
        Err(e) => return refuse(TimelockServiceRefusal::KeyDerivation(e)),
    }
    match derive_own_shares(
        &aggregated.subtranscript,
        &threshold_config,
        my_player,
        dk,
        &params.pp,
    ) {
        Ok(shares) => TimelockServiceState::Ready { epoch, shares },
        Err(e) => refuse(TimelockServiceRefusal::KeyDerivation(e)),
    }
}
