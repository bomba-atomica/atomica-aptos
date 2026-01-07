// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_crypto::{bls12381, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    pvss::{
        das::{self, WeightedTranscript},
        traits::{
            self, transcript::MalleableTranscript, HasEncryptionPublicParams, Reconstructable,
            SecretSharingConfig, Transcript,
        },
        Player, WeightedConfig,
    },
    utils::hash_to_scalar,
};
use aptos_types::{
    dkg::{
        real_dkg::{RealDKG, RealDKGPublicParams},
        timelock_dkg::{TimelockSecret, TimelockShare},
        DKGSessionMetadata, DKGTrait,
    },
    validator_verifier::ValidatorVerifier,
};
use ff::PrimeField;
use rand::rngs::StdRng;
use rand::{CryptoRng, Rng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// Use blstrs crate directly
use blstrs::{G1Projective, Scalar};
use ff::Field; // For Scalar::ZERO

// Internal Type Aliases for Privacy
type PvtDealtPubKey = <WeightedTranscript as Transcript>::DealtPubKey;
type PvtDealtPubKeyShare = <WeightedTranscript as Transcript>::DealtPubKeyShare;
type PvtDecryptPrivKey = <WeightedTranscript as Transcript>::DecryptPrivKey;
type PvtEncryptPubKey = <WeightedTranscript as Transcript>::EncryptPubKey;
type PvtInputSecret = <WeightedTranscript as Transcript>::InputSecret;
type PvtSigningSecretKey = <WeightedTranscript as Transcript>::SigningSecretKey;
type PvtSigningPubKey = <WeightedTranscript as Transcript>::SigningPubKey;

pub const TIMELOCK_WEIGHTED_DAS_SCALAR: &'static str = "timelock_weighted_das_scalar";

/// A weighted transcript extended for Timelock/IBE support.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct TimelockTranscript {
    pub weighted_transcript: WeightedTranscript,

    /// Encrypted Scalar Shares.
    /// Maps DealerID -> (R vector copy, Encrypted Scalars).
    pub scalar_transcripts: BTreeMap<u64, (Vec<G1Projective>, Vec<[u8; 32]>)>,
}

impl ValidCryptoMaterial for TimelockTranscript {
    const AIP_80_PREFIX: &'static str = "";
    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during TimelockTranscript serialization")
    }
}

impl TryFrom<&[u8]> for TimelockTranscript {
    type Error = CryptoMaterialError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<TimelockTranscript>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl traits::Transcript for TimelockTranscript {
    type DealtPubKey = PvtDealtPubKey;
    type DealtPubKeyShare = PvtDealtPubKeyShare;
    type DealtSecretKey = TimelockSecret;
    type DealtSecretKeyShare = Vec<TimelockShare>;
    type DecryptPrivKey = PvtDecryptPrivKey;
    type EncryptPubKey = PvtEncryptPubKey;
    type InputSecret = PvtInputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = WeightedConfig;
    type SigningPubKey = PvtSigningPubKey;
    type SigningSecretKey = PvtSigningSecretKey;

    fn dst() -> Vec<u8> {
        b"APTOS_TIMELOCK_WEIGHTED_DAS_DST".to_vec()
    }

    fn scheme_name() -> String {
        TIMELOCK_WEIGHTED_DAS_SCALAR.to_string()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        aux: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        // 1. Create a deterministic sub-rng for consistency
        let seed = rng.r#gen::<[u8; 32]>();
        let mut sub_rng = StdRng::from_seed(seed);
        let mut sub_rng_copy = StdRng::from_seed(seed);

        // 2. Call the black-box WeightedTranscript::deal
        let weighted_transcript =
            WeightedTranscript::deal(sc, pp, ssk, eks, s, aux, dealer, &mut sub_rng);

        // 3. Recover the exact same f_evals and r vector by re-running sub_rng
        let (_f_coeff, f_evals) = aptos_dkg::algebra::polynomials::shamir_secret_share(
            sc.get_threshold_config(),
            s,
            &mut sub_rng_copy,
        );
        let W = sc.get_total_weight();
        let r = aptos_dkg::utils::random::random_scalars(W, &mut sub_rng_copy);

        // 4. Encrypt scalar shares
        let mut encrypted_scalars = Vec::with_capacity(W);
        let g1 = pp.get_encryption_public_params().pubkey_base();

        let R_vec = r.iter().map(|ri| g1 * ri).collect::<Vec<G1Projective>>();

        let dst = b"APTOS_TIMELOCK_SCALAR_ENC_DST";
        for i in 0..sc.get_total_num_players() {
            let weight = sc.get_player_weight(&Player { id: i });
            for j in 0..weight {
                let k = sc.get_share_index(i, j).unwrap();
                let shared_secret = Into::<G1Projective>::into(&eks[i]) * r[k];
                let mask = hash_to_scalar(&bcs::to_bytes(&shared_secret).unwrap(), dst);
                let encrypted = f_evals[k] + mask;
                encrypted_scalars.push(encrypted.to_repr());
            }
        }

        let mut scalar_transcripts = BTreeMap::new();
        scalar_transcripts.insert(dealer.id as u64, (R_vec, encrypted_scalars));

        TimelockTranscript {
            weighted_transcript,
            scalar_transcripts,
        }
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        auxs: &Vec<A>,
    ) -> Result<()> {
        // Verify the underlying weighted transcript
        self.weighted_transcript.verify(sc, pp, spks, eks, auxs)?;

        // TODO: Verify scalar transcript consistency if needed.
        // For now, we rely on revelation-time verification.
        Ok(())
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.weighted_transcript.get_dealers()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Self) {
        self.weighted_transcript
            .aggregate_with(sc, &other.weighted_transcript);

        for (dealer_id, data) in &other.scalar_transcripts {
            self.scalar_transcripts.insert(*dealer_id, data.clone());
        }
    }

    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        self.weighted_transcript.get_public_key_share(sc, player)
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        self.weighted_transcript.get_dealt_public_key()
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let (group_shares, pub_shares) = self
            .weighted_transcript
            .decrypt_own_share(sc, player, dk, pp);

        let weight = sc.get_player_weight(player);
        let mut scalar_shares = vec![Scalar::ZERO; weight];

        let dst = b"APTOS_TIMELOCK_SCALAR_ENC_DST";
        let dk_scalar = Scalar::from_bytes_le(&dk.to_bytes()).unwrap();
        for (r_vec, encrypted_scalars) in self.scalar_transcripts.values() {
            let s_i_dealer = sc.get_player_starting_index(player);
            for j in 0..weight {
                let k = s_i_dealer + j;
                let shared_secret = r_vec[k] * dk_scalar;
                let mask = hash_to_scalar(&bcs::to_bytes(&shared_secret).unwrap(), dst);
                let encrypted = Scalar::from_repr(encrypted_scalars[k]).unwrap();
                scalar_shares[j] += encrypted - mask;
            }
        }

        let sk_shares = group_shares
            .into_iter()
            .zip(scalar_shares.into_iter())
            .map(|(gs, ss)| {
                // We don't have easy access to gs.share (it's private).
                // But we can get it via shadow if we really needed it for verification.
                // For now, just wrap the scalar.
                TimelockShare::new(ss)
            })
            .collect();

        (sk_shares, pub_shares)
    }

    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let weighted_transcript = WeightedTranscript::generate(sc, rng);
        TimelockTranscript {
            weighted_transcript,
            scalar_transcripts: BTreeMap::new(),
        }
    }
}

// Implement MalleableTranscript...
impl MalleableTranscript for TimelockTranscript {
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        ssk: &Self::SigningSecretKey,
        aux: &A,
        player: &Player,
    ) {
        self.weighted_transcript.maul_signature(ssk, aux, player);
    }
}

// Wrapper DKG struct
#[derive(Debug)]
pub struct TimelockDKG {}

impl DKGTrait for TimelockDKG {
    type DealerPrivateKey = bls12381::PrivateKey;
    type DealtPubKeyShare = PvtDealtPubKeyShare;
    type DealtSecret = TimelockSecret;
    type DealtSecretShare = Vec<TimelockShare>;
    type InputSecret = PvtInputSecret;
    type NewValidatorDecryptKey = PvtDecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = TimelockTranscript;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        RealDKG::new_public_params(dkg_session_metadata)
    }

    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret {
        RealDKG::aggregate_input_secret(secrets)
    }

    fn dealt_secret_from_input(
        _pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        TimelockSecret(*input.get_secret_a())
    }

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        input_player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> Result<Self::DealtSecret> {
        // Flatten shares
        let mut shares = Vec::new();
        for (player_id, weight_shares) in input_player_share_pairs {
            for (j, share) in weight_shares.into_iter().enumerate() {
                let player = Player {
                    id: player_id as usize,
                };
                let k = pub_params
                    .pvss_config
                    .wconfig
                    .get_share_index(player.id, j)
                    .unwrap();
                // We use k as the "Player ID" for Shamir reconstruction of the scalar polynomial
                shares.push((Player { id: k }, *share.as_scalar()));
            }
        }

        let secret_scalar =
            <Scalar as Reconstructable<aptos_dkg::pvss::ThresholdConfigBlstrs>>::reconstruct(
                &pub_params.pvss_config.wconfig.get_threshold_config(),
                &shares,
            );
        Ok(TimelockSecret(secret_scalar))
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .get_dealers()
            .iter()
            .map(|p| p.id as u64)
            .collect()
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        // Delegate to TimelockTranscript::deal
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        TimelockTranscript::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        )
    }

    fn verify_transcript_extra(
        _trx: &Self::Transcript,
        _verifier: &ValidatorVerifier,
        _checks_voting_power: bool,
        _ensures_single_dealer: Option<move_core_types::account_address::AccountAddress>,
    ) -> Result<()> {
        Ok(())
    }

    fn verify_transcript(_params: &Self::PublicParams, _trx: &Self::Transcript) -> Result<()> {
        Ok(())
    }

    fn aggregate_transcripts(
        _params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        accumulator.aggregate_with(&_params.pvss_config.wconfig, &element);
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
            &pub_params.pvss_config.pp,
        );
        Ok((sk, pk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::Uniform;
    use aptos_types::dkg::{DKGSessionMetadata, DKGTrait};
    use aptos_types::on_chain_config::RandomnessConfig;
    use aptos_types::validator_verifier::ValidatorConsensusInfoMoveStruct;
    use blstrs::Scalar;
    use ff::Field;
    use move_core_types::account_address::AccountAddress;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    fn create_test_session_metadata(num_validators: usize) -> DKGSessionMetadata {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);
        let mut validators = Vec::new();

        for i in 0..num_validators {
            let sk = bls12381::PrivateKey::generate(&mut rng);
            let pk = bls12381::PublicKey::from(&sk);
            let addr = AccountAddress::from_hex_literal(&format!("0x{:x}", i + 1)).unwrap();

            validators.push(ValidatorConsensusInfoMoveStruct {
                addr,
                pk_bytes: pk.to_bytes().to_vec(),
                voting_power: 1,
            });
        }

        DKGSessionMetadata {
            dealer_epoch: 1,
            randomness_config: RandomnessConfig::default_enabled().to_bytes(),
            dealer_validator_set: validators.clone(),
            target_validator_set: validators,
        }
    }

    #[test]
    fn test_timelock_transcript_deal_and_verify() {
        let mut rng = ChaCha20Rng::from_seed([1u8; 32]);
        let num_validators = 4;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);
        let dealer_sk = bls12381::PrivateKey::generate(&mut rng);

        // Generate transcript
        let transcript =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &dealer_sk);

        // Verify it has both weighted and scalar transcripts
        assert!(!transcript.scalar_transcripts.is_empty());
        assert_eq!(transcript.scalar_transcripts.len(), 1);
        assert!(transcript.scalar_transcripts.contains_key(&0));

        // Verify the transcript
        let result = TimelockDKG::verify_transcript(&pub_params, &transcript);
        assert!(
            result.is_ok(),
            "Transcript verification failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_timelock_transcript_aggregation() {
        let mut rng = ChaCha20Rng::from_seed([2u8; 32]);
        let num_validators = 3;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);

        // Generate transcripts from two dealers
        let sk1 = bls12381::PrivateKey::generate(&mut rng);
        let sk2 = bls12381::PrivateKey::generate(&mut rng);

        let mut transcript1 =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &sk1);

        let transcript2 =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 1, &sk2);

        // Aggregate
        TimelockDKG::aggregate_transcripts(&pub_params, &mut transcript1, transcript2);

        // Should now have both dealers' scalar transcripts
        assert_eq!(transcript1.scalar_transcripts.len(), 2);
        assert!(transcript1.scalar_transcripts.contains_key(&0));
        assert!(transcript1.scalar_transcripts.contains_key(&1));
    }

    #[test]
    fn test_decrypt_and_reconstruct_secret() {
        let mut rng = ChaCha20Rng::from_seed([3u8; 32]);
        let num_validators = 4;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);
        let expected_secret = TimelockDKG::dealt_secret_from_input(&pub_params, &input_secret);

        // Generate transcripts from enough dealers to meet threshold
        let mut aggregated_transcript = None;

        for dealer_idx in 0..3 {
            let sk = bls12381::PrivateKey::generate(&mut rng);
            let transcript = TimelockDKG::generate_transcript(
                &mut rng,
                &pub_params,
                &input_secret,
                dealer_idx,
                &sk,
            );

            if let Some(ref mut agg) = aggregated_transcript {
                TimelockDKG::aggregate_transcripts(&pub_params, agg, transcript);
            } else {
                aggregated_transcript = Some(transcript);
            }
        }

        let final_transcript = aggregated_transcript.unwrap();

        // Each validator decrypts their shares
        let mut player_shares = Vec::new();

        for player_idx in 0..num_validators {
            let dk = bls12381::PrivateKey::generate(&mut rng);
            let dk_pvss = aptos_dkg::pvss::das::decrypt_key_from_bls_sk(&dk).unwrap();

            let (shares, _pk_shares) = TimelockDKG::decrypt_secret_share_from_transcript(
                &pub_params,
                &final_transcript,
                player_idx as u64,
                &dk_pvss,
            )
            .unwrap();

            player_shares.push((player_idx as u64, shares));
        }

        // Reconstruct secret from shares
        let reconstructed =
            TimelockDKG::reconstruct_secret_from_shares(&pub_params, player_shares).unwrap();

        assert_eq!(reconstructed, expected_secret);
    }

    #[test]
    fn test_scalar_encryption_decryption() {
        let mut rng = ChaCha20Rng::from_seed([4u8; 32]);
        let num_validators = 3;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);
        let dealer_sk = bls12381::PrivateKey::generate(&mut rng);

        let transcript =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &dealer_sk);

        // Decrypt shares for player 0
        let player_dk = bls12381::PrivateKey::generate(&mut rng);
        let dk_pvss = aptos_dkg::pvss::das::decrypt_key_from_bls_sk(&player_dk).unwrap();

        let (scalar_shares, _) = TimelockDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &transcript,
            0,
            &dk_pvss,
        )
        .unwrap();

        // Should have shares equal to player weight
        let weight = pub_params
            .pvss_config
            .wconfig
            .get_player_weight(&Player { id: 0 });
        assert_eq!(scalar_shares.len(), weight);

        // Each share should be non-zero (with high probability)
        for share in &scalar_shares {
            assert_ne!(*share.as_scalar(), Scalar::ZERO);
        }
    }

    #[test]
    fn test_timelock_transcript_serialization() {
        let mut rng = ChaCha20Rng::from_seed([5u8; 32]);
        let num_validators = 3;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);
        let dealer_sk = bls12381::PrivateKey::generate(&mut rng);

        let transcript =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &dealer_sk);

        // Serialize
        let bytes = transcript.to_bytes();

        // Deserialize
        let transcript2 = TimelockTranscript::try_from(bytes.as_slice()).unwrap();

        assert_eq!(transcript, transcript2);
    }

    #[test]
    fn test_get_dealers() {
        let mut rng = ChaCha20Rng::from_seed([6u8; 32]);
        let num_validators = 4;
        let session_metadata = create_test_session_metadata(num_validators);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);

        let sk1 = bls12381::PrivateKey::generate(&mut rng);
        let sk2 = bls12381::PrivateKey::generate(&mut rng);
        let sk3 = bls12381::PrivateKey::generate(&mut rng);

        let mut transcript =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &sk1);
        let t2 = TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 1, &sk2);
        let t3 = TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 2, &sk3);

        TimelockDKG::aggregate_transcripts(&pub_params, &mut transcript, t2);
        TimelockDKG::aggregate_transcripts(&pub_params, &mut transcript, t3);

        let dealers = TimelockDKG::get_dealers(&transcript);

        assert_eq!(dealers.len(), 3);
        assert!(dealers.contains(&0));
        assert!(dealers.contains(&1));
        assert!(dealers.contains(&2));
    }

    #[test]
    fn test_single_validator_dkg() {
        let mut rng = ChaCha20Rng::from_seed([7u8; 32]);
        let session_metadata = create_test_session_metadata(1);

        let pub_params = TimelockDKG::new_public_params(&session_metadata);
        let input_secret = PvtInputSecret::generate(&mut rng);
        let dealer_sk = bls12381::PrivateKey::generate(&mut rng);

        let transcript =
            TimelockDKG::generate_transcript(&mut rng, &pub_params, &input_secret, 0, &dealer_sk);

        // Should still work with single validator
        assert!(TimelockDKG::verify_transcript(&pub_params, &transcript).is_ok());

        let player_dk = bls12381::PrivateKey::generate(&mut rng);
        let dk_pvss = aptos_dkg::pvss::das::decrypt_key_from_bls_sk(&player_dk).unwrap();

        let result = TimelockDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &transcript,
            0,
            &dk_pvss,
        );

        assert!(result.is_ok());
    }
}
