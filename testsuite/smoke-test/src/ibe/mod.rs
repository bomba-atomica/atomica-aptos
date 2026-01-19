// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for IBE encryption/decryption using DKG-derived scalar shares.
//!
//! These tests verify the end-to-end flow:
//! 1. DKG produces scalar shares via scalar ElGamal PVSS
//! 2. Scalar shares are reconstructed using Lagrange interpolation
//! 3. Master secret is used to derive IBE decryption key
//! 4. Encrypt/decrypt roundtrip works correctly

use aptos_crypto::Uniform;
use aptos_dkg::ibe::{compute_identity, derive_decryption_key, ibe_decrypt, ibe_encrypt};
use aptos_dkg::pvss::input_secret::InputSecret;
use aptos_dkg::pvss::{
    scalar_elgamal::WeightedTranscript,
    test_utils::setup_dealing,
    traits::{Reconstructable, Transcript as TranscriptTrait},
    Player, WeightedConfig,
};
use aptos_types::on_chain_config::OnChainConfig;
use blstrs::G2Projective;
use group::{Curve, Group};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

const G2_COMPRESSED_LENGTH: usize = 96;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IBEPublicParams {
    mpk: Vec<u8>,
    epoch: u64,
}

impl OnChainConfig for IBEPublicParams {
    const MODULE_IDENTIFIER: &'static str = "ibe_config";
    const TYPE_IDENTIFIER: &'static str = "IBEPublicParams";
}

fn deserialize_mpk(mpk_bytes: &[u8]) -> blstrs::G2Affine {
    let bytes: [u8; G2_COMPRESSED_LENGTH] = mpk_bytes.try_into().expect("MPK should be 96 bytes");
    blstrs::G2Affine::from_compressed(&bytes).expect("MPK should be valid G2 point")
}

/// Test IBE with scalar ElGamal PVSS using test setup (not full DKG).
/// This validates the core IBE-PVSS integration works correctly.
#[tokio::test]
async fn elgamal_encrypt_decrypt() {
    let mut rng = rand::thread_rng();

    // Create a weighted config for 4 validators with threshold 3
    let weights = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights).unwrap();

    // Use test utils to generate key pairs for validators
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);

    // Create an input secret
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();

    // Create the scalar ElGamal transcript
    let transcript = WeightedTranscript::deal(
        &wconfig,
        &dealing_args.pp,
        &dealing_args.ssks[0],
        &dealing_args.eks,
        &input_secret,
        &vec![0u8],
        &Player { id: 0 },
        &mut rng,
    );

    // Each validator decrypts their share using their own private key
    let mut shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = Vec::new();
    for i in 0..4 {
        let (sk_share, _pk_share) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: i },
                &dealing_args.dks[i],
                &dealing_args.pp,
            )
            .expect("decrypt_own_share should not fail for valid transcript");
        shares.push((Player { id: i }, sk_share));
    }

    // Reconstruct the master secret using any 3 shares
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
    let reconstructed = <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
        &wconfig,
        &shares_for_recon,
    );

    // Verify reconstruction
    assert_eq!(
        reconstructed.s, secret,
        "Reconstructed secret should match original"
    );

    // Now test IBE encrypt/decrypt with the reconstructed secret
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let test_timelock_id: u64 = 12345;
    let test_deadline_us: u64 = 1_000_000_000_000;
    let identity = compute_identity(test_timelock_id, test_deadline_us);

    let decryption_key = derive_decryption_key(&reconstructed.s, &identity);

    let plaintext = b"Hello, Timelock! This is a secret message for the future.";
    let mut test_rng = rand::rngs::StdRng::seed_from_u64(42);

    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut test_rng);
    let decrypted = ibe_decrypt(&decryption_key, &ciphertext);

    assert_eq!(
        decrypted, plaintext,
        "Decrypted message does not match original plaintext"
    );
}

/// Test IBE with multiple identities using the same reconstructed secret.
#[tokio::test]
async fn elgamal_encrypt_decrypt_with_different_identities() {
    let mut rng = rand::thread_rng();

    // Create weighted config for 4 validators with threshold 3
    let weights = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights).unwrap();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);

    // Create an input secret
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();

    let transcript = WeightedTranscript::deal(
        &wconfig,
        &dealing_args.pp,
        &dealing_args.ssks[0],
        &dealing_args.eks,
        &input_secret,
        &vec![0u8],
        &Player { id: 0 },
        &mut rng,
    );

    // Get shares from first 3 validators
    let shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = (0..3)
        .map(|i| {
            let (sk_share, _pk_share) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: i },
                    &dealing_args.dks[i],
                    &dealing_args.pp,
                )
                .expect("decrypt_own_share should not fail for valid transcript");
            (Player { id: i }, sk_share)
        })
        .collect();

    // Reconstruct
    let reconstructed =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(&wconfig, &shares);
    assert_eq!(reconstructed.s, secret);

    // Test IBE with multiple identities
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let mut test_rng = rand::rngs::StdRng::seed_from_u64(12345);

    for i in 0..3 {
        let identity = compute_identity(i, 1000000000);
        let dk = derive_decryption_key(&reconstructed.s, &identity);

        let plaintext = format!("Message for identity {}", i);
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext.as_bytes(), &mut test_rng);
        let decrypted = ibe_decrypt(&dk, &ciphertext);

        assert_eq!(decrypted, plaintext.as_bytes());
    }
}
