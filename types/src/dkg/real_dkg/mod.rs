// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dkg::{
        real_dkg::rounding::DKGRounding, DKGSessionMetadata, DKGTrait, MayHaveRoundingSummary,
        RoundingSummary,
    },
    on_chain_config::OnChainRandomnessConfig,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::{anyhow, bail, ensure, Context};
#[cfg(any(test, feature = "testing"))]
use aptos_crypto::Uniform;
use aptos_crypto::{bls12381, bls12381::PrivateKey};
use aptos_dkg::{
    pvss,
    pvss::{
        scalar_elgamal,
        traits::{Convert, Reconstructable, Transcript},
        Player,
    },
};
use fixed::types::U64F64;
use move_core_types::account_address::AccountAddress;
use num_traits::Zero;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
    time::Instant,
};

pub mod rounding;

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

/// Weighted transcript type for Scalar ElGamal PVSS (produces scalar shares for IBE).
///
/// This is the scalar-output counterpart to `WTrx` (DAS). Both are dealt in parallel
/// from the same `InputSecret`, producing:
/// - `WTrx` → G1 shares for WVUF (randomness)
/// - `ScalarTrx` → Scalar shares for IBE (timelock)
///
/// See: `crates/aptos-dkg/src/pvss/scalar_elgamal/` for implementation.
pub type ScalarTrx = scalar_elgamal::WeightedTranscript;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness generation
    pub wconfig: SSConfig,
    // weighted config for randomness generation in fast path
    pub fast_wconfig: Option<SSConfig>,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
    // Some metrics for caller to consume.
    #[serde(skip)]
    pub rounding_summary: RoundingSummary,
}

impl PartialEq for DKGPvssConfig {
    fn eq(&self, other: &Self) -> bool {
        (
            self.epoch,
            &self.wconfig,
            &self.fast_wconfig,
            &self.pp,
            &self.eks,
        ) == (
            other.epoch,
            &other.wconfig,
            &other.fast_wconfig,
            &other.pp,
            &other.eks,
        )
    }
}

impl DKGPvssConfig {
    pub fn new(
        epoch: u64,
        wconfig: SSConfig,
        fast_wconfig: Option<SSConfig>,
        pp: DkgPP,
        eks: Vec<EncPK>,
        rounding_summary: RoundingSummary,
    ) -> Self {
        Self {
            epoch,
            wconfig,
            fast_wconfig,
            pp,
            eks,
            rounding_summary,
        }
    }
}

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    secrecy_threshold: U64F64,
    reconstruct_threshold: U64F64,
    maybe_fast_path_secrecy_threshold: Option<U64F64>,
    next_validators: &[ValidatorConsensusInfo],
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validators.iter().map(|vi| vi.voting_power).collect();
    let timer = Instant::now();
    let DKGRounding {
        profile,
        wconfig,
        fast_wconfig,
        rounding_error,
        rounding_method,
    } = DKGRounding::new(
        &validator_stakes,
        secrecy_threshold,
        reconstruct_threshold,
        maybe_fast_path_secrecy_threshold,
    );
    let rounding_time = timer.elapsed();
    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
        .iter()
        .map(|vi| vi.public_key.clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let pp = DkgPP::default_with_bls_base();

    let rounding_summary = RoundingSummary {
        method: rounding_method,
        output: profile,
        exec_time: rounding_time,
        error: rounding_error,
    };

    DKGPvssConfig::new(
        cur_epoch,
        wconfig,
        fast_wconfig,
        pp,
        consensus_keys,
        rounding_summary,
    )
}

#[derive(Debug)]
pub struct RealDKG {}

#[derive(Clone, Debug)]
pub struct RealDKGPublicParams {
    pub session_metadata: DKGSessionMetadata,
    pub pvss_config: DKGPvssConfig,
    pub verifier: Arc<ValidatorVerifier>,
}

impl MayHaveRoundingSummary for RealDKGPublicParams {
    fn rounding_summary(&self) -> Option<&RoundingSummary> {
        Some(&self.pvss_config.rounding_summary)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Transcripts {
    // transcript for main path (DAS PVSS → G1 shares for randomness)
    pub main: WTrx,
    // transcript for fast path (DAS PVSS → G1 shares for randomness fast path)
    pub fast: Option<WTrx>,
    // transcript for scalar shares (Scalar ElGamal PVSS → scalar shares for IBE)
    //
    // This is the dual-output DKG extension (ADR-001). When present, this transcript
    // shares the same underlying secret as `main`, but produces scalar shares instead
    // of G1 shares. This enables IBE (timelock encryption) without a separate DKG.
    //
    // Both transcripts produce the same MPK (g2^secret), but:
    // - `main` produces G1 shares for WVUF (randomness)
    // - `scalar` produces scalar shares for IBE (timelock)
    //
    // TODO(Phase 2): Implement ScalarTrx dealing in generate_transcript()
    pub scalar: Option<ScalarTrx>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtPubKeyShares {
    // dealt public key share for main path
    pub main: <WTrx as Transcript>::DealtPubKeyShare,
    // dealt public key share for fast path
    pub fast: Option<<WTrx as Transcript>::DealtPubKeyShare>,
    // dealt public key share for scalar/IBE path
    //
    // These are the public key shares corresponding to the scalar secret shares.
    // They can be used to verify that a validator's scalar share is correct
    // via pairing checks.
    //
    // TODO(Phase 2): Populate this field in decrypt_secret_share_from_transcript()
    pub scalar: Option<<ScalarTrx as Transcript>::DealtPubKeyShare>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtSecretKeyShares {
    // dealt secret key share for main path
    pub main: <WTrx as Transcript>::DealtSecretKeyShare,
    // dealt secret key share for fast path
    pub fast: Option<<WTrx as Transcript>::DealtSecretKeyShare>,
    // dealt secret key share for scalar/IBE path
    //
    // When present, this is a vector of scalar shares that can be used
    // for IBE decryption key derivation. The scalar shares can be
    // reconstructed using Lagrange interpolation to obtain the secret
    // scalar `s`, which is then used to compute `H(identity)^s`.
    //
    // TODO(Phase 2): Populate this field in decrypt_secret_share_from_transcript()
    pub scalar: Option<<ScalarTrx as Transcript>::DealtSecretKeyShare>,
}

impl DKGTrait for RealDKG {
    type DealerPrivateKey = <WTrx as Transcript>::SigningSecretKey;
    type DealtPubKeyShare = DealtPubKeyShares;
    type DealtSecret = <WTrx as Transcript>::DealtSecretKey;
    type DealtSecretShare = DealtSecretKeyShares;
    type InputSecret = <WTrx as Transcript>::InputSecret;
    type NewValidatorDecryptKey = <WTrx as Transcript>::DecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = Transcripts;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        let randomness_config = dkg_session_metadata
            .randomness_config_derived()
            .unwrap_or_else(OnChainRandomnessConfig::default_enabled);
        let secrecy_threshold = randomness_config
            .secrecy_threshold()
            .unwrap_or_else(|| *rounding::DEFAULT_SECRECY_THRESHOLD);
        let reconstruct_threshold = randomness_config
            .reconstruct_threshold()
            .unwrap_or_else(|| *rounding::DEFAULT_RECONSTRUCT_THRESHOLD);
        let maybe_fast_path_secrecy_threshold = randomness_config.fast_path_secrecy_threshold();

        let pvss_config = build_dkg_pvss_config(
            dkg_session_metadata.dealer_epoch,
            secrecy_threshold,
            reconstruct_threshold,
            maybe_fast_path_secrecy_threshold,
            &dkg_session_metadata.target_validator_consensus_infos_cloned(),
        );
        let verifier = ValidatorVerifier::new(dkg_session_metadata.dealer_consensus_infos_cloned());
        RealDKGPublicParams {
            session_metadata: dkg_session_metadata.clone(),
            pvss_config,
            verifier: verifier.into(),
        }
    }

    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret {
        secrets
            .into_iter()
            .fold(<WTrx as Transcript>::InputSecret::zero(), |acc, item| {
                acc + item
            })
    }

    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        input.to(&pub_params.pvss_config.pp)
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        let wtrx = WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        );
        // transcript for fast path
        let fast_wtrx = pub_params
            .pvss_config
            .fast_wconfig
            .as_ref()
            .map(|fast_wconfig| {
                WTrx::deal(
                    fast_wconfig,
                    &pub_params.pvss_config.pp,
                    sk,
                    &pub_params.pvss_config.eks,
                    input_secret,
                    &aux,
                    &Player { id: my_index },
                    rng,
                )
            });

        // TODO(Phase 2): Generate scalar transcript for IBE
        //
        // This should deal the same input_secret using ScalarTrx::deal().
        // The scalar transcript shares the same underlying secret as the main
        // transcript, but produces scalar shares instead of G1 shares.
        //
        // Implementation:
        // let scalar_trx = ScalarTrx::deal(
        //     &pub_params.pvss_config.wconfig,
        //     &pub_params.pvss_config.pp,
        //     sk,
        //     &pub_params.pvss_config.eks,
        //     input_secret,
        //     &aux,
        //     &Player { id: my_index },
        //     rng,
        // );
        //
        // For now, we set this to None until ScalarTrx::deal() is implemented.
        let scalar_trx: Option<ScalarTrx> = None;

        Transcripts {
            main: wtrx,
            fast: fast_wtrx,
            scalar: scalar_trx,
        }
    }

    /// Perform extra necessary checks missing in `verify_transcript`.
    ///
    /// Additionally:
    /// - (needed in block proposal validation) if `check_voting_power`,
    ///   also check if the dealer set specified in the transcript has enough voting power;
    /// - (needed in peer transcript verification) if `ensures_single_dealer` is set,
    ///   also check if the dealer set specified in the transcript only contains the peer.
    fn verify_transcript_extra(
        trx: &Self::Transcript,
        verifier: &ValidatorVerifier,
        checks_voting_power: bool,
        ensures_single_dealer: Option<AccountAddress>,
    ) -> anyhow::Result<()> {
        let all_validator_addrs = verifier.get_ordered_account_addresses();
        let main_trx_dealers = trx.main.get_dealers();
        let mut dealer_set = HashSet::with_capacity(main_trx_dealers.len());
        for dealer in main_trx_dealers.iter() {
            if let Some(dealer_addr) = all_validator_addrs.get(dealer.id) {
                dealer_set.insert(*dealer_addr);
            } else {
                bail!("invalid dealer idx");
            }
        }
        ensure!(main_trx_dealers.len() == dealer_set.len());
        if ensures_single_dealer.is_some() {
            let expected_dealer_set: HashSet<AccountAddress> =
                ensures_single_dealer.into_iter().collect();
            ensure!(expected_dealer_set == dealer_set);
        }

        if checks_voting_power {
            verifier
                .check_voting_power(dealer_set.iter(), true)
                .context("not enough power")?;
        }

        if let Some(fast_trx) = &trx.fast {
            ensure!(fast_trx.get_dealers() == main_trx_dealers);
            ensure!(trx.main.get_dealt_public_key() == fast_trx.get_dealt_public_key());
        }

        // TODO(Phase 2): Verify scalar transcript if present
        //
        // When scalar transcript is implemented:
        // 1. Verify dealers match main transcript
        // 2. Verify MPK matches main transcript (same underlying secret)
        //
        // if let Some(scalar_trx) = &trx.scalar {
        //     ensure!(scalar_trx.get_dealers() == main_trx_dealers);
        //     ensure!(trx.main.get_dealt_public_key() == scalar_trx.get_dealt_public_key());
        // }

        Ok(())
    }

    /// NOTE: this is used in VM.
    fn verify_transcript(
        params: &Self::PublicParams,
        trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        // Verify dealer indices are valid.
        let dealers = trx
            .main
            .get_dealers()
            .iter()
            .map(|player| player.id)
            .collect::<Vec<usize>>();
        let num_validators = params.session_metadata.dealer_validator_set.len();
        ensure!(
            dealers.iter().all(|id| *id < num_validators),
            "real_dkg::verify_transcript failed with invalid dealer index."
        );

        let all_eks = params.pvss_config.eks.clone();

        let addresses = params.verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();

        let spks = dealers_addresses
            .iter()
            .filter_map(|author| params.verifier.get_public_key(author))
            .collect::<Vec<_>>();

        let aux = dealers_addresses
            .iter()
            .map(|address| (params.pvss_config.epoch, address))
            .collect::<Vec<_>>();

        trx.main.verify(
            &params.pvss_config.wconfig,
            &params.pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        // Verify fast path is present if and only if fast_wconfig is present.
        ensure!(
            trx.fast.is_some() == params.pvss_config.fast_wconfig.is_some(),
            "real_dkg::verify_transcript failed with mismatched fast path flag in trx and params."
        );

        if let Some(fast_trx) = trx.fast.as_ref() {
            let fast_dealers = fast_trx
                .get_dealers()
                .iter()
                .map(|player| player.id)
                .collect::<Vec<usize>>();
            ensure!(
                dealers == fast_dealers,
                "real_dkg::verify_transcript failed with inconsistent dealer index."
            );
        }

        if let (Some(fast_trx), Some(fast_wconfig)) =
            (trx.fast.as_ref(), params.pvss_config.fast_wconfig.as_ref())
        {
            fast_trx.verify(fast_wconfig, &params.pvss_config.pp, &spks, &all_eks, &aux)?;
        }

        // TODO(Phase 2): Verify scalar transcript if present
        //
        // When ScalarTrx::verify() is implemented:
        // if let Some(scalar_trx) = trx.scalar.as_ref() {
        //     // Verify dealers match main transcript
        //     let scalar_dealers = scalar_trx
        //         .get_dealers()
        //         .iter()
        //         .map(|player| player.id)
        //         .collect::<Vec<usize>>();
        //     ensure!(
        //         dealers == scalar_dealers,
        //         "real_dkg::verify_transcript failed with inconsistent dealer index in scalar transcript."
        //     );
        //
        //     // Verify the scalar transcript
        //     scalar_trx.verify(&params.pvss_config.wconfig, &params.pvss_config.pp, &spks, &all_eks, &aux)?;
        //
        //     // Verify MPK matches main transcript (same underlying secret)
        //     ensure!(
        //         trx.main.get_dealt_public_key() == scalar_trx.get_dealt_public_key(),
        //         "real_dkg::verify_transcript failed with mismatched MPK between main and scalar transcripts."
        //     );
        // }

        Ok(())
    }

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        accumulator
            .main
            .aggregate_with(&params.pvss_config.wconfig, &element.main);
        if let (Some(acc), Some(ele), Some(config)) = (
            accumulator.fast.as_mut(),
            element.fast.as_ref(),
            params.pvss_config.fast_wconfig.as_ref(),
        ) {
            acc.aggregate_with(config, ele);
        }

        // TODO(Phase 2): Aggregate scalar transcripts
        //
        // When ScalarTrx::aggregate_with() is implemented:
        // if let (Some(acc), Some(ele)) = (
        //     accumulator.scalar.as_mut(),
        //     element.scalar.as_ref(),
        // ) {
        //     acc.aggregate_with(&params.pvss_config.wconfig, ele);
        // }
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.main.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
            &pub_params.pvss_config.pp,
        );
        assert_eq!(
            trx.fast.is_some(),
            pub_params.pvss_config.fast_wconfig.is_some()
        );
        let (fast_sk, fast_pk) = match (
            trx.fast.as_ref(),
            pub_params.pvss_config.fast_wconfig.as_ref(),
        ) {
            (Some(fast_trx), Some(fast_wconfig)) => {
                let (fast_sk, fast_pk) = fast_trx.decrypt_own_share(
                    fast_wconfig,
                    &Player {
                        id: player_idx as usize,
                    },
                    dk,
                    &pub_params.pvss_config.pp,
                );
                (Some(fast_sk), Some(fast_pk))
            },
            _ => (None, None),
        };

        // TODO(Phase 2): Decrypt scalar shares for IBE
        //
        // When ScalarTrx::decrypt_own_share() is implemented:
        // let (scalar_sk, scalar_pk) = match trx.scalar.as_ref() {
        //     Some(scalar_trx) => {
        //         let (scalar_sk, scalar_pk) = scalar_trx.decrypt_own_share(
        //             &pub_params.pvss_config.wconfig,
        //             &Player {
        //                 id: player_idx as usize,
        //             },
        //             dk,
        //             &pub_params.pvss_config.pp,
        //         );
        //         (Some(scalar_sk), Some(scalar_pk))
        //     },
        //     None => (None, None),
        // };
        let (scalar_sk, scalar_pk): (
            Option<<ScalarTrx as Transcript>::DealtSecretKeyShare>,
            Option<<ScalarTrx as Transcript>::DealtPubKeyShare>,
        ) = (None, None);

        Ok((
            DealtSecretKeyShares {
                main: sk,
                fast: fast_sk,
                scalar: scalar_sk,
            },
            DealtPubKeyShares {
                main: pk,
                fast: fast_pk,
                scalar: scalar_pk,
            },
        ))
    }

    // Test-only function
    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        input_player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let player_share_pairs = input_player_share_pairs
            .clone()
            .into_iter()
            .map(|(x, y)| (Player { id: x as usize }, y.main))
            .collect();
        let reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
            &pub_params.pvss_config.wconfig,
            &player_share_pairs,
        );
        if input_player_share_pairs
            .clone()
            .into_iter()
            .all(|(_, y)| y.fast.is_some())
            && pub_params.pvss_config.fast_wconfig.is_some()
        {
            let fast_player_share_pairs = input_player_share_pairs
                .into_iter()
                .map(|(x, y)| (Player { id: x as usize }, y.fast.unwrap()))
                .collect();
            let fast_reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
                pub_params.pvss_config.fast_wconfig.as_ref().unwrap(),
                &fast_player_share_pairs,
            );
            ensure!(
                reconstructed_secret == fast_reconstructed_secret,
                "real_dkg::reconstruct_secret_from_shares failed with inconsistent dealt secrets."
            );
        }
        Ok(reconstructed_secret)
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .main
            .get_dealers()
            .into_iter()
            .map(|x| x.id as u64)
            .collect()
    }

    /// Get the IBE Master Public Key (MPK) from the transcript.
    ///
    /// For RealDKG, this extracts the dealt public key from the main transcript
    /// and serializes it as compressed G2 bytes (96 bytes).
    ///
    /// # Implementation Status
    ///
    /// TODO(Phase 3): This is a stub implementation. The actual implementation
    /// should serialize the G2 element properly once the aptos-dkg serialization
    /// helpers are wired up.
    fn get_ibe_master_public_key(transcript: &Self::Transcript) -> Vec<u8> {
        // Extract the dealt public key (G2) from the main transcript
        let _dpk = transcript.main.get_dealt_public_key();

        // TODO(Phase 3): Serialize the G2 element
        //
        // The dealt public key is a G2Affine. We need to serialize it:
        // use aptos_dkg::utils::g2_proj_to_bytes;
        // g2_proj_to_bytes(&dpk.into())
        //
        // For now, return empty until serialization is wired up.
        // This allows the code to compile while the full implementation is pending.
        Vec::new()
    }

    /// Get the scalar secret share from a dealt secret share.
    ///
    /// For RealDKG, this extracts the scalar shares from the `DealtSecretKeyShares`
    /// struct and serializes them as bytes.
    ///
    /// # Implementation Status
    ///
    /// TODO(Phase 3): This is a stub implementation. Returns None until
    /// the scalar transcript dealing is fully implemented.
    fn get_scalar_secret_share(dealt_share: &Self::DealtSecretShare) -> Option<Vec<u8>> {
        // Check if scalar shares are present
        let _scalar_shares = dealt_share.scalar.as_ref()?;

        // TODO(Phase 3): Serialize the scalar shares
        //
        // Each scalar share is a Vec<Scalar> (for weighted threshold).
        // Serialize each scalar to 32 bytes (little-endian):
        //
        // Some(scalar_shares.iter()
        //     .flat_map(|s| s.to_bytes_le().to_vec())
        //     .collect())
        //
        // For now, return None until the full implementation is ready.
        None
    }
}

impl RealDKG {
    #[cfg(any(test, feature = "testing"))]
    pub fn sample_secret_and_generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &<RealDKG as DKGTrait>::PublicParams,
        my_index: u64,
        sk: &<RealDKG as DKGTrait>::DealerPrivateKey,
    ) -> <RealDKG as DKGTrait>::Transcript {
        let secret = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        Self::generate_transcript(rng, pub_params, &secret, my_index, sk)
    }

    /// The same dealer deals twice and aggregates the transcripts.
    #[cfg(any(test, feature = "testing"))]
    pub fn deal_twice_and_aggregate<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &<RealDKG as DKGTrait>::PublicParams,
        my_index: u64,
        sk: &<RealDKG as DKGTrait>::DealerPrivateKey,
    ) -> <RealDKG as DKGTrait>::Transcript {
        let secret_0 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let mut trx_0 = Self::generate_transcript(rng, pub_params, &secret_0, my_index, sk);
        let secret_1 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let trx_1 = Self::generate_transcript(rng, pub_params, &secret_1, my_index, sk);
        Self::aggregate_transcripts(pub_params, &mut trx_0, trx_1);
        assert_eq!(2, trx_0.main.get_dealers().len());
        trx_0
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn generate_transcript_for_inconsistent_secrets<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &<RealDKG as DKGTrait>::PublicParams,
        my_index: u64,
        sk: &<RealDKG as DKGTrait>::DealerPrivateKey,
    ) -> <RealDKG as DKGTrait>::Transcript {
        let secret_0 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let secret_1 = <RealDKG as DKGTrait>::InputSecret::generate(rng);
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        let wtrx = WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            &secret_0,
            &aux,
            &Player { id: my_index },
            rng,
        );
        // transcript for fast path
        let fast_wtrx = pub_params
            .pvss_config
            .fast_wconfig
            .as_ref()
            .map(|fast_wconfig| {
                WTrx::deal(
                    fast_wconfig,
                    &pub_params.pvss_config.pp,
                    sk,
                    &pub_params.pvss_config.eks,
                    &secret_1,
                    &aux,
                    &Player { id: my_index },
                    rng,
                )
            });
        Transcripts {
            main: wtrx,
            fast: fast_wtrx,
        }
    }
}
pub fn maybe_dk_from_bls_sk(
    sk: &PrivateKey,
) -> anyhow::Result<<WTrx as Transcript>::DecryptPrivKey> {
    let mut bytes = sk.to_bytes(); // in big-endian
    bytes.reverse();
    <WTrx as Transcript>::DecryptPrivKey::try_from(bytes.as_slice())
        .map_err(|e| anyhow!("dk_from_bls_sk failed with dk deserialization error: {e}"))
}
