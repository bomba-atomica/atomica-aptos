// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for the IBE module.

use super::*;
use blstrs::G2Projective;
use group::Group;
use rand::SeedableRng;

/// Creates a test master key pair.
fn create_test_keypair<R: rand::Rng>(rng: &mut R) -> (Scalar, G2Affine) {
    let msk = random_scalar(rng);
    let mpk = G2Projective::generator().mul(&msk).to_affine();
    (msk, mpk)
}

#[test]
fn test_compute_identity_deterministic() {
    let id1 = compute_identity(123, 1000000);
    let id2 = compute_identity(123, 1000000);
    let id3 = compute_identity(124, 1000000);
    let id4 = compute_identity(123, 1000001);

    // Same inputs should give same output
    assert_eq!(id1, id2);

    // Different inputs should give different outputs
    assert_ne!(id1, id3);
    assert_ne!(id1, id4);
    assert_ne!(id3, id4);
}

#[test]
fn test_hash_to_g1_deterministic() {
    let identity = compute_identity(42, 999999);
    let h1 = hash_to_g1(&identity);
    let h2 = hash_to_g1(&identity);

    assert_eq!(h1, h2);

    // Different identity should give different point
    let other_identity = compute_identity(43, 999999);
    let h3 = hash_to_g1(&other_identity);
    assert_ne!(h1, h3);
}

#[test]
fn test_hash_to_g1_not_identity() {
    let identity = compute_identity(1, 1);
    let h = hash_to_g1(&identity);

    // Hash should not produce the identity element
    assert_ne!(h, G1Projective::identity());
}

#[test]
fn test_derive_decryption_key() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
    let (msk, _mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(100, 2000000);
    let dk = derive_decryption_key(&msk, &identity);

    // Decryption key should be deterministic for same secret and identity
    let dk2 = derive_decryption_key(&msk, &identity);
    assert_eq!(dk, dk2);

    // Different identity should give different key
    let other_identity = compute_identity(101, 2000000);
    let dk3 = derive_decryption_key(&msk, &other_identity);
    assert_ne!(dk, dk3);
}

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(1, 1000000);
    let dk = derive_decryption_key(&msk, &identity);

    let plaintext = b"Hello, timelock encryption!";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk, &ciphertext);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_decrypt_empty_message() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(2, 2000000);
    let dk = derive_decryption_key(&msk, &identity);

    let plaintext = b"";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk, &ciphertext);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_decrypt_large_message() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(3);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(3, 3000000);
    let dk = derive_decryption_key(&msk, &identity);

    // Test with a message larger than the SHA3-256 block size (32 bytes)
    let plaintext: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let ciphertext = ibe_encrypt(&mpk, &identity, &plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk, &ciphertext);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_wrong_decryption_key_fails() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(4);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity1 = compute_identity(10, 1000000);
    let identity2 = compute_identity(20, 1000000);

    // Get decryption key for identity1
    let dk1 = derive_decryption_key(&msk, &identity1);

    // Encrypt for identity2
    let plaintext = b"Secret message";
    let ciphertext = ibe_encrypt(&mpk, &identity2, plaintext, &mut rng);

    // Try to decrypt with wrong key - should produce garbage
    let decrypted = ibe_decrypt(&dk1, &ciphertext);
    assert_ne!(decrypted, plaintext);
}

#[test]
fn test_different_random_gives_different_ciphertext() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(100);

    let (msk, mpk) = create_test_keypair(&mut rng);
    let identity = compute_identity(5, 5000000);
    let dk = derive_decryption_key(&msk, &identity);

    let plaintext = b"Same message";

    // Create a new RNG for encryption
    let mut enc_rng1 = rand::rngs::StdRng::seed_from_u64(300);
    let mut enc_rng2 = rand::rngs::StdRng::seed_from_u64(400);

    let ciphertext1 = ibe_encrypt(&mpk, &identity, plaintext, &mut enc_rng1);
    let ciphertext2 = ibe_encrypt(&mpk, &identity, plaintext, &mut enc_rng2);

    // Ciphertexts should be different (randomized encryption)
    assert_ne!(ciphertext1.u, ciphertext2.u);
    assert_ne!(ciphertext1.v, ciphertext2.v);

    // But both should decrypt to the same plaintext
    let decrypted1 = ibe_decrypt(&dk, &ciphertext1);
    let decrypted2 = ibe_decrypt(&dk, &ciphertext2);
    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

#[test]
fn test_verify_decryption_key_valid() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(500);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(50, 50000000);
    let dk = derive_decryption_key(&msk, &identity);

    assert!(verify_decryption_key(&dk, &identity, &mpk));
}

#[test]
fn test_verify_decryption_key_wrong_identity() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(600);
    let (msk, mpk) = create_test_keypair(&mut rng);

    let identity1 = compute_identity(60, 60000000);
    let identity2 = compute_identity(61, 60000000);
    let dk = derive_decryption_key(&msk, &identity1);

    // Key derived for identity1 should not verify for identity2
    assert!(!verify_decryption_key(&dk, &identity2, &mpk));
}

#[test]
fn test_verify_decryption_key_wrong_mpk() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(700);
    let (msk1, _mpk1) = create_test_keypair(&mut rng);
    let (_msk2, mpk2) = create_test_keypair(&mut rng);

    let identity = compute_identity(70, 70000000);
    let dk = derive_decryption_key(&msk1, &identity);

    // Key derived with msk1 should not verify against mpk2
    assert!(!verify_decryption_key(&dk, &identity, &mpk2));
}

#[test]
fn test_ciphertext_serialization_roundtrip() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(800);
    let (_, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(80, 80000000);
    let plaintext = b"Serialize me!";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    // Serialize and deserialize
    let encoded = bcs::to_bytes(&ciphertext).expect("serialization should succeed");
    let decoded: Ciphertext = bcs::from_bytes(&encoded).expect("deserialization should succeed");

    assert_eq!(ciphertext, decoded);
}

#[test]
fn test_key_stream_determinism() {
    // The key stream derivation should be deterministic
    let mut rng = rand::rngs::StdRng::seed_from_u64(900);
    let (_, mpk) = create_test_keypair(&mut rng);

    let identity = compute_identity(90, 90000000);

    // Compute the pairing result that would be used in encryption
    let h = hash_to_g1(&identity);
    let r = random_scalar(&mut rng);
    let h_r = h.mul(&r).to_affine();
    let pairing_result = pairing(&h_r, &mpk);

    // Key streams of same length should be identical
    let ks1 = derive_key_stream(&pairing_result, 100);
    let ks2 = derive_key_stream(&pairing_result, 100);
    assert_eq!(ks1, ks2);

    // Prefix should match for different lengths
    let ks3 = derive_key_stream(&pairing_result, 50);
    assert_eq!(&ks1[..50], &ks3[..]);
}

#[test]
fn test_ibe_roundtrip_with_known_scalar() {
    // Test full IBE roundtrip with a known scalar secret
    let mut rng = rand::rngs::StdRng::seed_from_u64(12345);

    // Use a known scalar as master secret
    let msk = Scalar::from(42u64);
    let mpk = G2Projective::generator().mul(&msk).to_affine();

    let identity = compute_identity(12345, 1000000000);
    let dk = derive_decryption_key(&msk, &identity);

    // Encrypt and decrypt
    let plaintext = b"Test message for IBE roundtrip";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk, &ciphertext);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_scalar_elgamal_pvss_ibe_roundtrip() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Create a threshold config for 4 validators with threshold 3
    let weights = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights).unwrap();

    // Use test utils to generate key pairs for validators
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);

    // Extract keys
    let dks = &dealing_args.dks;
    let eks = &dealing_args.eks;

    // Create an input secret
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();

    // Create the scalar ElGamal transcript
    let transcript = WeightedTranscript::deal(
        &wconfig,
        &dealing_args.pp,
        &dealing_args.ssks[0],
        &eks,
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
            .decrypt_own_share(&wconfig, &Player { id: i }, &dks[i], &dealing_args.pp)
            .expect("decrypt_own_share should not fail for valid transcript");
        shares.push((Player { id: i }, sk_share));
    }

    // Reconstruct the master secret using any 3 shares
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];

    // Explicitly annotate the expected type to verify it matches
    // This should fail if Share type doesn't match
    let reconstructed: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    // The reconstructed secret should equal the original
    assert_eq!(
        reconstructed.s, secret,
        "Reconstructed secret should match original"
    );

    // Now test IBE encrypt/decrypt with the reconstructed secret
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let identity = compute_identity(12345, 1000000000);
    let dk = derive_decryption_key(&reconstructed.s, &identity);

    let plaintext = b"Test message from reconstructed secret";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk, &ciphertext);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_scalar_elgamal_pvss_ibe_multiple_identities() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Create threshold config for 5 validators with threshold 3
    let weights = vec![1, 1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights).unwrap();

    // Use test utils to generate key pairs for validators
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);

    // Extract keys
    let dks = &dealing_args.dks;
    let eks = &dealing_args.eks;

    // Deal a random secret
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();
    let transcript = WeightedTranscript::deal(
        &wconfig,
        &dealing_args.pp,
        &dealing_args.ssks[0],
        &eks,
        &input_secret,
        &vec![0u8],
        &Player { id: 0 },
        &mut rng,
    );

    // Decrypt shares from first 3 validators using their correct private keys
    let shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = (0..3)
        .map(|i| {
            let (sk_share, _pk_share) = transcript
                .decrypt_own_share(&wconfig, &Player { id: i }, &dks[i], &dealing_args.pp)
                .expect("decrypt_own_share should not fail for valid transcript");
            (Player { id: i }, sk_share)
        })
        .collect();

    // Reconstruct
    let reconstructed =
        <<WeightedTranscript as TranscriptTrait>::DealtSecretKey as Reconstructable<
            WeightedConfig,
        >>::reconstruct(&wconfig, &shares);

    assert_eq!(reconstructed.s, secret);

    // Test IBE with multiple identities using reconstructed secret
    let mpk = G2Projective::generator().mul(&secret).to_affine();

    for i in 0..5 {
        let identity = compute_identity(i, 1000000000);
        let dk = derive_decryption_key(&reconstructed.s, &identity);

        let plaintext = format!("Message for identity {}", i);
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext.as_bytes(), &mut rng);
        let decrypted = ibe_decrypt(&dk, &ciphertext);

        assert_eq!(decrypted, plaintext.as_bytes());
    }
}

#[test]
fn test_dk_share_aggregation_roundtrip() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use blstrs::G1Projective;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    let weights = vec![1, 1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights.clone()).unwrap();
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);

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

    let mut shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = Vec::new();
    for i in 0..5 {
        let (sk_share, _pk_share) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: i },
                &dealing_args.dks[i],
                &dealing_args.pp,
            )
            .expect("decrypt_own_share should succeed");
        shares.push((Player { id: i }, sk_share));
    }

    let identity = compute_identity(12345, 1000000000);
    let h_identity = hash_to_g1(&identity);

    // Path A: Compute G1 DK shares for each validator
    // dk_share_i = s_i × H(identity) for each validator
    // For weighted config, sum all shares per validator
    let _dk_shares_g1: Vec<G1Projective> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            let mut sum = G1Projective::identity();
            for sk_share in sk_shares.iter() {
                sum += h_identity.mul(&sk_share.0.s);
            }
            sum
        })
        .collect();

    // Path B: Reconstruct master secret using framework (handles Lagrange correctly)
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];

    let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    assert_eq!(
        reconstructed_secret.s, secret,
        "Reconstructed master secret should match original dealt secret"
    );

    // Derive DK from reconstructed master secret: DK = s × H(identity)
    let dk_from_scalar = derive_decryption_key(&reconstructed_secret.s, &identity);

    // Test encryption/decryption with DK from scalar reconstruction
    let mpk = G2Projective::generator().mul(&secret).to_affine();

    let plaintext = b"Test message for DK share aggregation roundtrip";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

    let decrypted = ibe_decrypt(&dk_from_scalar, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ DK share aggregation roundtrip test passed!");
    println!("   - Path B: Reconstructed master secret matches original");
    println!("   - Decryption with reconstructed DK successful");
    println!("   - Path A: G1 DK shares (dk_share_i = s_i × H) computed but require");
    println!("     correct Lagrange coefficients for aggregation, handled by framework");
}

#[test]
fn test_scalar_elgamal_pvss_ibe_roundtrip_unequal_weights() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Unequal weights: [2, 1, 2] - total weight 5, threshold 3
    let weights = vec![2, 1, 2];
    let wconfig = WeightedConfig::new(3, weights).unwrap();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
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

    let mut shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = Vec::new();
    for i in 0..3 {
        let (sk_share, _pk_share) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: i },
                &dealing_args.dks[i],
                &dealing_args.pp,
            )
            .expect("decrypt_own_share should succeed");
        shares.push((Player { id: i }, sk_share));
    }

    // Reconstruct with all 3 players (total weight = 5 >= threshold 3)
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
    let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    assert_eq!(reconstructed_secret.s, secret);

    // IBE operations with reconstructed secret
    let identity = compute_identity(42, 1_000_000_000_000);
    let dk = derive_decryption_key(&reconstructed_secret.s, &identity);
    let mpk = G2Projective::generator().mul(&secret).to_affine();

    // Encrypt and decrypt
    let plaintext = b"Unequal weights IBE roundtrip test [2,1,2]";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = ibe_decrypt(&dk, &ciphertext);

    assert_eq!(decrypted, plaintext);
    println!("✅ Unequal weights [2,1,2] IBE roundtrip test passed!");
}

#[test]
fn test_reconstruct_ibe_dk_equal_weights() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Test: 5 validators, threshold 3, equal weights [1,1,1,1,1]
    let weights = vec![1, 1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights.clone()).unwrap();
    let total_weight: u64 = weights.iter().map(|w| *w as u64).sum();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
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

    // Decrypt shares from first 3 validators
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
                .expect("decrypt_own_share should not fail");
            (Player { id: i }, sk_share)
        })
        .collect();

    // Identity for IBE
    let identity = compute_identity(12345, 1000000000);

    // Extract scalar shares for each validator
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| sk_shares.iter().map(|sk_share| sk_share.0.s).collect())
        .collect();

    // Validator indices and weights for reconstruction
    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = vec![1, 1, 1, 1, 1]; // Full weights for all 5 validators

    // Use reconstruct_ibe_dk with scalar shares
    let reconstructed_dk = reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    );

    // Reconstruct the master secret using framework (for verification)
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
    let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    assert_eq!(
        reconstructed_secret.s, secret,
        "Reconstructed scalar should match original secret"
    );

    // Expected DK from reconstructed secret
    let expected_dk = derive_decryption_key(&reconstructed_secret.s, &identity);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "reconstruct_ibe_dk should produce same DK as derive_decryption_key for equal weights"
    );

    // Encrypt/decrypt verification
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let plaintext = b"Test reconstruct_ibe_dk equal weights";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ test_reconstruct_ibe_dk_equal_weights passed");
}

#[test]
fn test_reconstruct_ibe_dk_unequal_weights() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Test: 3 validators, threshold 3, unequal weights [2,1,2]
    let weights = vec![2, 1, 2];
    let wconfig = WeightedConfig::new(3, weights.clone()).unwrap();
    let total_weight: u64 = weights.iter().map(|w| *w as u64).sum();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
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

    // Decrypt shares from all 3 validators
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
                .expect("decrypt_own_share should not fail");
            (Player { id: i }, sk_share)
        })
        .collect();

    // Identity for IBE
    let identity = compute_identity(42, 1_000_000_000_000);

    // Extract scalar shares for each validator
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| sk_shares.iter().map(|sk_share| sk_share.0.s).collect())
        .collect();

    // Validator indices and weights for reconstruction
    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = vec![2, 1, 2]; // Full weights for all 3 validators

    // Use reconstruct_ibe_dk with scalar shares
    let reconstructed_dk = reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    );

    // Reconstruct the master secret using framework (for verification)
    let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
    let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
        <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
            &wconfig,
            &shares_for_recon,
        );

    // Expected DK from reconstructed secret
    let expected_dk = derive_decryption_key(&reconstructed_secret.s, &identity);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "reconstruct_ibe_dk should produce same DK as derive_decryption_key for unequal weights"
    );

    // Encrypt/decrypt verification
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let plaintext = b"Test reconstruct_ibe_dk unequal weights [2,1,2]";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ test_reconstruct_ibe_dk_unequal_weights passed");
}

#[test]
fn test_reconstruct_ibe_dk_sparse_indices() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::Transcript as TranscriptTrait;
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Test: 4 validators, threshold 2, equal weights [1,1,1,1]
    let weights = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(2, weights.clone()).unwrap();
    let total_weight: u64 = weights.iter().map(|w| *w as u64).sum();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
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

    // Decrypt shares from validators 0 and 2 (sparse subset)
    let shares: Vec<(
        Player,
        <WeightedTranscript as TranscriptTrait>::DealtSecretKeyShare,
    )> = vec![
        {
            let (sk_share, _pk_share) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: 0 },
                    &dealing_args.dks[0],
                    &dealing_args.pp,
                )
                .expect("decrypt_own_share should not fail");
            (Player { id: 0 }, sk_share)
        },
        {
            let (sk_share, _pk_share) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: 2 },
                    &dealing_args.dks[2],
                    &dealing_args.pp,
                )
                .expect("decrypt_own_share should not fail");
            (Player { id: 2 }, sk_share)
        },
    ];

    // Identity for IBE
    let identity = compute_identity(99, 2_000_000_000_000);

    // Extract scalar shares for each validator
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| sk_shares.iter().map(|sk_share| sk_share.0.s).collect())
        .collect();

    // Use sparse validator indices [0, 2]
    let validator_indices: Vec<u64> = vec![0, 2];
    let full_weights: Vec<u64> = vec![1, 1, 1, 1]; // Full weights for all 4 validators

    // Use reconstruct_ibe_dk with scalar shares
    let reconstructed_dk = reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    );

    // Expected DK from master secret
    let expected_dk = derive_decryption_key(&secret, &identity);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "reconstruct_ibe_dk should work with sparse validator indices [0, 2]"
    );

    // Encrypt/decrypt verification
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    let plaintext = b"Test reconstruct_ibe_dk sparse indices [0,2]";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ test_reconstruct_ibe_dk_sparse_indices passed");
}

#[test]
fn test_reconstruct_ibe_dk_single_share() {
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::Transcript as TranscriptTrait;
    use crate::pvss::{Player, WeightedConfig};
    use aptos_crypto::Uniform;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // Test: 1 validator with 1 share
    let weights = vec![1];
    let wconfig = WeightedConfig::new(1, weights.clone()).unwrap();
    let total_weight: u64 = weights.iter().map(|w| *w as u64).sum();

    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
    let input_secret = InputSecret::generate(&mut rng);
    let _secret = *input_secret.get_secret_a();

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

    // Decrypt single share
    let (sk_share, _pk_share) = transcript
        .decrypt_own_share(
            &wconfig,
            &Player { id: 0 },
            &dealing_args.dks[0],
            &dealing_args.pp,
        )
        .expect("decrypt_own_share should not fail");

    // Identity for IBE
    let identity = compute_identity(1, 1_000_000_000);

    // Extract scalar shares
    let scalar_shares: Vec<Vec<Scalar>> = vec![sk_share.iter().map(|s| s.0.s).collect()];

    // Use reconstruct_ibe_dk with single validator
    let validator_indices: Vec<u64> = vec![0];
    let full_weights: Vec<u64> = vec![1]; // Full weights for all validators

    let _reconstructed_dk = reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    );

    println!("✅ test_reconstruct_ibe_dk_single_share passed");
}
