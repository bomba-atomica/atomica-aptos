// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Generate golden test vectors for IBE (Identity-Based Encryption).
//!
//! This module generates cryptographic test vectors for:
//! - Identity computation (hash from timelock_id + deadline)
//! - IBE DK share reconstruction via Lagrange interpolation
//! - Full encryption/decryption roundtrip
//!
//! These vectors are used across:
//! - Rust unit tests
//! - Move VM native function tests
//! - Move language unit tests (ibe.move, ibe_config.move)
//!
//! Run with: `cargo test --package aptos-dkg generate_golden_vectors -- --ignored --nocapture`

use super::{
    compute_identity, derive_decryption_key, hash_to_g1, ibe_decrypt, ibe_encrypt,
    verify_decryption_key,
};
use crate::algebra::lagrange::lagrange_coefficients;
use crate::pvss::input_secret::InputSecret;
use crate::pvss::scalar_elgamal::WeightedTranscript;
use crate::pvss::test_utils::setup_dealing;
use crate::pvss::traits::{Reconstructable, Transcript as TranscriptTrait};
use crate::pvss::{Player, WeightedConfig};
use aptos_crypto::Uniform;
use blstrs::{G1Affine, G1Projective, G2Affine, G2Projective, Scalar};
use ff::Field;
use group::{Curve, Group};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::ops::Mul;
use std::{fs::File, io::Write};

/// Identity computation test vector (timelock_id, deadline -> identity hash)
#[derive(Serialize, Deserialize, Debug)]
struct IdentityVector {
    description: String,
    timelock_id: u64,
    deadline_us: u64,
    /// Identity hash (32 bytes hex) - matches Move's compute_identity output
    identity_hash_hex: String,
    /// H(identity) mapped to G1 (48 bytes compressed hex)
    h_identity_g1_hex: String,
}

/// IBE roundtrip test vector with DK share reconstruction
#[derive(Serialize, Deserialize, Debug)]
struct IbeRoundtripVector {
    description: String,
    /// Seed used for deterministic generation (for reproducibility)
    rng_seed: u64,
    /// Master secret key scalar (32 bytes hex) - for verification only
    msk_hex: String,
    /// Master public key (G2 compressed, 96 bytes hex)
    mpk_g2_hex: String,
    /// Identity hash (32 bytes hex)
    identity_hash_hex: String,
    /// H(identity) as G1 point (48 bytes compressed hex)
    h_identity_g1_hex: String,
    /// Threshold configuration
    threshold: u64,
    total_weight: u64,
    /// Validator configuration (parallel arrays)
    /// Indices are 0-based player IDs matching the PVSS framework
    validator_indices: Vec<u64>,
    validator_weights: Vec<u64>,
    /// DK shares: dk_i = s_i * H(identity) as G1 (48 bytes compressed hex each)
    /// Each validator may have multiple sub-shares based on weight
    dk_shares_g1_hex: Vec<String>,
    /// Reconstructed DK = sum(lambda_i * dk_i) as G1 (48 bytes compressed hex)
    reconstructed_dk_g1_hex: String,
    /// Plaintext for encryption test (hex)
    plaintext_hex: String,
    /// Ciphertext U component (G2 compressed, 96 bytes hex)
    ciphertext_u_g2_hex: String,
    /// Ciphertext V component (encrypted payload, hex)
    ciphertext_v_hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GoldenVectors {
    version: String,
    generated_at: String,
    description: String,
    identity_vectors: Vec<IdentityVector>,
    ibe_roundtrip_vectors: Vec<IbeRoundtripVector>,
}

/// Serialize G1 point to compressed hex (48 bytes)
fn g1_to_hex(point: &G1Affine) -> String {
    hex::encode(point.to_compressed())
}

/// Serialize G2 point to compressed hex (96 bytes)
fn g2_to_hex(point: &G2Affine) -> String {
    hex::encode(point.to_compressed())
}

/// Serialize scalar to hex (32 bytes, little-endian)
fn scalar_to_hex(s: &Scalar) -> String {
    hex::encode(s.to_bytes_le())
}

#[test]
#[ignore] // Run with --ignored flag to generate vectors
fn generate_golden_vectors() {
    println!("\n🔧 Generating IBE golden test vectors...\n");

    let mut vectors = GoldenVectors {
        version: "2.0.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        description: "Golden test vectors for IBE identity computation and DK reconstruction"
            .to_string(),
        identity_vectors: vec![],
        ibe_roundtrip_vectors: vec![],
    };

    // ==========================================
    // Part 1: Identity computation vectors
    // ==========================================
    println!("📝 Generating identity computation vectors...\n");

    let identity_test_cases = [
        (0u64, 1_000_000_000_000u64, "Basic timelock with ID 0"),
        (
            1u64,
            1_000_000_000_000u64,
            "Different ID, same deadline (should differ)",
        ),
        (
            0u64,
            2_000_000_000_000u64,
            "Same ID, different deadline (should differ)",
        ),
        (999999u64, 9_999_999_999_999u64, "Large values"),
        (u64::MAX, u64::MAX, "Maximum u64 values"),
    ];

    for (timelock_id, deadline_us, description) in identity_test_cases.iter() {
        let identity = compute_identity(*timelock_id, *deadline_us);
        let h_identity = hash_to_g1(&identity);

        let vector = IdentityVector {
            description: description.to_string(),
            timelock_id: *timelock_id,
            deadline_us: *deadline_us,
            identity_hash_hex: hex::encode(&identity),
            h_identity_g1_hex: g1_to_hex(&h_identity.to_affine()),
        };

        println!("  ✓ ID={}, deadline={}", timelock_id, deadline_us);
        println!("    identity: {}", &vector.identity_hash_hex);

        vectors.identity_vectors.push(vector);
    }

    // ==========================================
    // Part 2: IBE roundtrip vectors using PVSS
    // ==========================================
    println!("\n📝 Generating IBE roundtrip vectors using PVSS framework...\n");

    // Test case 1: 5 validators, threshold 3, equal weights
    {
        let seed = 12345u64;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let weights: Vec<usize> = vec![1, 1, 1, 1, 1];
        let threshold = 3usize;
        let wconfig = WeightedConfig::new(threshold, weights.clone()).unwrap();

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

        let timelock_id = 1u64;
        let deadline_us = 1_704_067_200_000_000u64;
        let identity = compute_identity(timelock_id, deadline_us);
        let h_identity = hash_to_g1(&identity);

        let dk_shares_g1: Vec<G1Projective> = shares
            .iter()
            .map(|(_player, sk_shares)| {
                let mut sum = G1Projective::identity();
                for sk_share in sk_shares.iter() {
                    sum += h_identity.mul(&sk_share.0.s);
                }
                sum
            })
            .collect();

        // Reconstruct using first 3 shares (threshold=3)
        let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
        let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &shares_for_recon,
            );

        assert_eq!(reconstructed_secret.s, secret);

        let expected_dk = derive_decryption_key(&secret, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        // Reconstruct DK from G1 shares
        let player_ids: Vec<usize> = vec![0, 1, 2];
        let lagr = lagrange_coefficients(
            wconfig.get_batch_evaluation_domain(),
            &player_ids,
            &Scalar::ZERO,
        );

        let mut reconstructed_dk_g1 = G1Projective::identity();
        for (i, &player_id) in player_ids.iter().enumerate() {
            reconstructed_dk_g1 += dk_shares_g1[player_id].mul(&lagr[i]);
        }
        let reconstructed_dk = reconstructed_dk_g1.to_affine();

        assert_eq!(reconstructed_dk, expected_dk);

        let plaintext = b"Hello IBE golden vector test with PVSS!";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&reconstructed_dk, &ciphertext);
        assert_eq!(decrypted, plaintext);

        let vector = IbeRoundtripVector {
            description: "5 validators, threshold 3, equal weights (PVSS-based)".to_string(),
            rng_seed: seed,
            msk_hex: scalar_to_hex(&secret),
            mpk_g2_hex: g2_to_hex(&mpk),
            identity_hash_hex: hex::encode(&identity),
            h_identity_g1_hex: g1_to_hex(&h_identity.to_affine()),
            threshold: threshold as u64,
            total_weight: weights.iter().map(|w| *w as u64).sum(),
            validator_indices: (0..5).collect(),
            validator_weights: weights.iter().map(|w| *w as u64).collect(),
            dk_shares_g1_hex: dk_shares_g1
                .iter()
                .map(|s| g1_to_hex(&s.to_affine()))
                .collect(),
            reconstructed_dk_g1_hex: g1_to_hex(&reconstructed_dk),
            plaintext_hex: hex::encode(plaintext),
            ciphertext_u_g2_hex: g2_to_hex(&ciphertext.u),
            ciphertext_v_hex: hex::encode(&ciphertext.v),
        };

        println!("  ✓ Test case 1: {}", vector.description);
        vectors.ibe_roundtrip_vectors.push(vector);
    }

    // Test case 2: 4 validators, threshold 2, equal weights
    {
        let seed = 67890u64;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let weights: Vec<usize> = vec![1, 1, 1, 1];
        let threshold = 2usize;
        let wconfig = WeightedConfig::new(threshold, weights.clone()).unwrap();

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
        for i in 0..4 {
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

        let timelock_id = 42u64;
        let deadline_us = 2_000_000_000_000u64;
        let identity = compute_identity(timelock_id, deadline_us);
        let h_identity = hash_to_g1(&identity);

        let dk_shares_g1: Vec<G1Projective> = shares
            .iter()
            .map(|(_player, sk_shares)| {
                let mut sum = G1Projective::identity();
                for sk_share in sk_shares.iter() {
                    sum += h_identity.mul(&sk_share.0.s);
                }
                sum
            })
            .collect();

        let shares_for_recon = vec![shares[0].clone(), shares[2].clone()];
        let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &shares_for_recon,
            );

        assert_eq!(reconstructed_secret.s, secret);

        let expected_dk = derive_decryption_key(&secret, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        let player_ids: Vec<usize> = vec![0, 2];
        let lagr = lagrange_coefficients(
            wconfig.get_batch_evaluation_domain(),
            &player_ids,
            &Scalar::ZERO,
        );

        let mut reconstructed_dk_g1 = G1Projective::identity();
        for (i, &player_id) in player_ids.iter().enumerate() {
            reconstructed_dk_g1 += dk_shares_g1[player_id].mul(&lagr[i]);
        }
        let reconstructed_dk = reconstructed_dk_g1.to_affine();

        assert_eq!(reconstructed_dk, expected_dk);

        let plaintext = b"Second test case with 4 validators";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&reconstructed_dk, &ciphertext);
        assert_eq!(decrypted, plaintext);

        let vector = IbeRoundtripVector {
            description: "4 validators, threshold 2, equal weights".to_string(),
            rng_seed: seed,
            msk_hex: scalar_to_hex(&secret),
            mpk_g2_hex: g2_to_hex(&mpk),
            identity_hash_hex: hex::encode(&identity),
            h_identity_g1_hex: g1_to_hex(&h_identity.to_affine()),
            threshold: threshold as u64,
            total_weight: weights.iter().map(|w| *w as u64).sum(),
            validator_indices: (0..4).collect(),
            validator_weights: weights.iter().map(|w| *w as u64).collect(),
            dk_shares_g1_hex: dk_shares_g1
                .iter()
                .map(|s| g1_to_hex(&s.to_affine()))
                .collect(),
            reconstructed_dk_g1_hex: g1_to_hex(&reconstructed_dk),
            plaintext_hex: hex::encode(plaintext),
            ciphertext_u_g2_hex: g2_to_hex(&ciphertext.u),
            ciphertext_v_hex: hex::encode(&ciphertext.v),
        };

        println!("  ✓ Test case 2: {}", vector.description);
        vectors.ibe_roundtrip_vectors.push(vector);
    }

    // Test case 3: 3 validators, threshold 3, unequal weights [2, 1, 2]
    {
        let seed = 99999u64;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let weights: Vec<usize> = vec![2, 1, 2];
        let threshold = 3usize;
        let wconfig = WeightedConfig::new(threshold, weights.clone()).unwrap();

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

        let timelock_id = 100u64;
        let deadline_us = 3_000_000_000_000u64;
        let identity = compute_identity(timelock_id, deadline_us);
        let h_identity = hash_to_g1(&identity);

        let dk_shares_g1: Vec<G1Projective> = shares
            .iter()
            .map(|(_player, sk_shares)| {
                let mut sum = G1Projective::identity();
                for sk_share in sk_shares.iter() {
                    sum += h_identity.mul(&sk_share.0.s);
                }
                sum
            })
            .collect();

        let shares_for_recon = vec![shares[0].clone(), shares[1].clone(), shares[2].clone()];
        let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &shares_for_recon,
            );

        assert_eq!(reconstructed_secret.s, secret);

        let expected_dk = derive_decryption_key(&reconstructed_secret.s, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        let plaintext = b"Unequal weights test [2,1,2]";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&expected_dk, &ciphertext);
        assert_eq!(decrypted, plaintext);

        let vector = IbeRoundtripVector {
            description: "3 validators, threshold 3, unequal weights [2,1,2]".to_string(),
            rng_seed: seed,
            msk_hex: scalar_to_hex(&secret),
            mpk_g2_hex: g2_to_hex(&mpk),
            identity_hash_hex: hex::encode(&identity),
            h_identity_g1_hex: g1_to_hex(&h_identity.to_affine()),
            threshold: threshold as u64,
            total_weight: weights.iter().map(|w| *w as u64).sum(),
            validator_indices: (0..3).collect(),
            validator_weights: weights.iter().map(|w| *w as u64).collect(),
            dk_shares_g1_hex: dk_shares_g1
                .iter()
                .map(|s| g1_to_hex(&s.to_affine()))
                .collect(),
            reconstructed_dk_g1_hex: g1_to_hex(&expected_dk),
            plaintext_hex: hex::encode(plaintext),
            ciphertext_u_g2_hex: g2_to_hex(&ciphertext.u),
            ciphertext_v_hex: hex::encode(&ciphertext.v),
        };

        println!("  ✓ Test case 3: {}", vector.description);
        vectors.ibe_roundtrip_vectors.push(vector);
    }

    // Test case 4: 4 validators, threshold 3, unequal weights [2, 3, 2, 1]
    {
        let seed = 11111u64;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        let weights: Vec<usize> = vec![2, 3, 2, 1];
        let threshold = 3usize;
        let wconfig = WeightedConfig::new(threshold, weights.clone()).unwrap();

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
        for i in 0..4 {
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

        let timelock_id = 200u64;
        let deadline_us = 4_000_000_000_000u64;
        let identity = compute_identity(timelock_id, deadline_us);
        let h_identity = hash_to_g1(&identity);

        let dk_shares_g1: Vec<G1Projective> = shares
            .iter()
            .map(|(_player, sk_shares)| {
                let mut sum = G1Projective::identity();
                for sk_share in sk_shares.iter() {
                    sum += h_identity.mul(&sk_share.0.s);
                }
                sum
            })
            .collect();

        let shares_for_recon = vec![shares[0].clone(), shares[1].clone()];
        let reconstructed_secret: <WeightedTranscript as TranscriptTrait>::DealtSecretKey =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &shares_for_recon,
            );

        assert_eq!(reconstructed_secret.s, secret);

        let expected_dk = derive_decryption_key(&reconstructed_secret.s, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        let plaintext = b"Unequal weights [2,3,2,1] 4 validators";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&expected_dk, &ciphertext);
        assert_eq!(decrypted, plaintext);

        let vector = IbeRoundtripVector {
            description: "4 validators, threshold 3, unequal weights [2,3,2,1]".to_string(),
            rng_seed: seed,
            msk_hex: scalar_to_hex(&secret),
            mpk_g2_hex: g2_to_hex(&mpk),
            identity_hash_hex: hex::encode(&identity),
            h_identity_g1_hex: g1_to_hex(&h_identity.to_affine()),
            threshold: threshold as u64,
            total_weight: weights.iter().map(|w| *w as u64).sum(),
            validator_indices: (0..4).collect(),
            validator_weights: weights.iter().map(|w| *w as u64).collect(),
            dk_shares_g1_hex: dk_shares_g1
                .iter()
                .map(|s| g1_to_hex(&s.to_affine()))
                .collect(),
            reconstructed_dk_g1_hex: g1_to_hex(&expected_dk),
            plaintext_hex: hex::encode(plaintext),
            ciphertext_u_g2_hex: g2_to_hex(&ciphertext.u),
            ciphertext_v_hex: hex::encode(&ciphertext.v),
        };

        println!("  ✓ Test case 4: {}", vector.description);
        vectors.ibe_roundtrip_vectors.push(vector);
    }

    // ==========================================
    // Write output files
    // ==========================================
    let output_path = "atomica/golden_vectors/ibe_golden_vectors.json";
    std::fs::create_dir_all("atomica/golden_vectors").unwrap();
    let json = serde_json::to_string_pretty(&vectors).unwrap();
    let mut file = File::create(output_path).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    println!(
        "\n✅ Generated {} identity vectors",
        vectors.identity_vectors.len()
    );
    println!(
        "✅ Generated {} IBE roundtrip vectors",
        vectors.ibe_roundtrip_vectors.len()
    );
    println!("✅ Saved to {}", output_path);

    // Human-readable summary
    let txt_path = "atomica/golden_vectors/ibe_golden_vectors.txt";
    let mut txt_file = File::create(txt_path).unwrap();
    writeln!(txt_file, "IBE Golden Test Vectors").unwrap();
    writeln!(txt_file, "=======================").unwrap();
    writeln!(txt_file, "Generated: {}", vectors.generated_at).unwrap();
    writeln!(txt_file, "Version: {}", vectors.version).unwrap();
    writeln!(txt_file, "\n{}\n", vectors.description).unwrap();

    writeln!(txt_file, "Identity Vectors:").unwrap();
    for (i, v) in vectors.identity_vectors.iter().enumerate() {
        writeln!(
            txt_file,
            "  {}. {} (ID={}, deadline={})",
            i + 1,
            v.description,
            v.timelock_id,
            v.deadline_us
        )
        .unwrap();
        writeln!(txt_file, "     identity: {}", v.identity_hash_hex).unwrap();
    }

    writeln!(txt_file, "\nIBE Roundtrip Vectors:").unwrap();
    for (i, v) in vectors.ibe_roundtrip_vectors.iter().enumerate() {
        writeln!(txt_file, "  {}. {}", i + 1, v.description).unwrap();
        writeln!(
            txt_file,
            "     threshold: {}/{}",
            v.threshold, v.total_weight
        )
        .unwrap();
        writeln!(txt_file, "     validators: {:?}", v.validator_indices).unwrap();
        writeln!(
            txt_file,
            "     dk_shares: {} shares",
            v.dk_shares_g1_hex.len()
        )
        .unwrap();
    }

    println!("✅ Saved text summary to {}\n", txt_path);
}

/// Test unequal weights encryption/decryption roundtrip with fresh keys.
/// This verifies that the PVSS framework correctly handles weighted reconstruction
/// for IBE operations.
#[test]
fn test_unequal_weights_ibe_roundtrip() {
    use rand::SeedableRng;

    // Test case 1: [2, 1, 2] weights
    {
        let mut rng = rand::rngs::StdRng::seed_from_u64(99999);
        let weights: Vec<usize> = vec![2, 1, 2];
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
            let (sk_share, _) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: i },
                    &dealing_args.dks[i],
                    &dealing_args.pp,
                )
                .unwrap();
            shares.push((Player { id: i }, sk_share));
        }

        let reconstructed_secret =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &vec![shares[0].clone(), shares[1].clone(), shares[2].clone()],
            );
        assert_eq!(reconstructed_secret.s, secret);

        let identity = compute_identity(100, 3_000_000_000_000);
        let dk = derive_decryption_key(&reconstructed_secret.s, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        let plaintext = b"Unequal weights [2,1,2] test";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&dk, &ciphertext);
        assert_eq!(decrypted, plaintext);
    }

    // Test case 2: [2, 3, 2, 1] weights
    {
        let mut rng = rand::rngs::StdRng::seed_from_u64(11111);
        let weights: Vec<usize> = vec![2, 3, 2, 1];
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
        for i in 0..4 {
            let (sk_share, _) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: i },
                    &dealing_args.dks[i],
                    &dealing_args.pp,
                )
                .unwrap();
            shares.push((Player { id: i }, sk_share));
        }

        let reconstructed_secret =
            <WeightedTranscript as TranscriptTrait>::DealtSecretKey::reconstruct(
                &wconfig,
                &vec![shares[0].clone(), shares[1].clone()],
            );
        assert_eq!(reconstructed_secret.s, secret);

        let identity = compute_identity(200, 4_000_000_000_000);
        let dk = derive_decryption_key(&reconstructed_secret.s, &identity);
        let mpk = G2Projective::generator().mul(&secret).into();

        let plaintext = b"Unequal weights [2,3,2,1] test";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        let decrypted = ibe_decrypt(&dk, &ciphertext);
        assert_eq!(decrypted, plaintext);
    }

    println!("✅ Unequal weights IBE roundtrip tests passed!");
}

/// Test that loads and verifies the golden vector JSON file.
/// This ensures the generated fixtures are cryptographically valid using high-level APIs.
#[test]
fn test_golden_vectors_file_validity() {
    #[derive(serde::Deserialize)]
    struct GoldenVectorsJson {
        version: String,
        identity_vectors: Vec<serde_json::Value>,
        ibe_roundtrip_vectors: Vec<IbeRoundtripVectorJson>,
    }

    #[derive(serde::Deserialize)]
    struct IbeRoundtripVectorJson {
        description: String,
        msk_hex: String,
        mpk_g2_hex: String,
        identity_hash_hex: String,
        validator_indices: Vec<u64>,
        validator_weights: Vec<u64>,
        dk_shares_g1_hex: Vec<String>,
        reconstructed_dk_g1_hex: String,
        plaintext_hex: String,
        ciphertext_u_g2_hex: String,
        ciphertext_v_hex: String,
    }

    let json_path = "atomica/golden_vectors/ibe_golden_vectors.json";
    let content =
        std::fs::read_to_string(json_path).expect(&format!("Failed to read {}", json_path));

    let vectors: GoldenVectorsJson =
        serde_json::from_str(&content).expect("Failed to parse golden vectors JSON");

    assert!(
        vectors.version.starts_with("2."),
        "Expected version 2.x, got {}",
        vectors.version
    );
    assert!(
        !vectors.ibe_roundtrip_vectors.is_empty(),
        "Expected at least one roundtrip vector"
    );

    for (i, v) in vectors.ibe_roundtrip_vectors.iter().enumerate() {
        println!("Verifying roundtrip vector {}: {}", i + 1, v.description);

        // Load the golden vector data
        let msk_bytes: [u8; 32] = hex::decode(&v.msk_hex)
            .expect(&format!("Vector {}: Invalid MSK hex", i + 1))
            .try_into()
            .expect("MSK should be 32 bytes");
        let msk = blstrs::Scalar::from_bytes_le(&msk_bytes).expect("Invalid MSK scalar bytes");

        // Compute MPK from MSK
        let mpk = G2Projective::generator().mul(&msk).into();

        // Load identity from golden vector
        let identity: [u8; 32] = hex::decode(&v.identity_hash_hex)
            .expect(&format!("Vector {}: Invalid identity hex", i + 1))
            .try_into()
            .expect("Identity should be 32 bytes");

        // Load plaintext
        let plaintext = hex::decode(&v.plaintext_hex)
            .expect(&format!("Vector {}: Invalid plaintext hex", i + 1));

        // Load ciphertext components
        let ciphertext_u_bytes: [u8; 96] = hex::decode(&v.ciphertext_u_g2_hex)
            .expect(&format!("Vector {}: Invalid ciphertext U hex", i + 1))
            .try_into()
            .expect("Ciphertext U should be 96 bytes");
        let ciphertext_u = blstrs::G2Affine::from_compressed(&ciphertext_u_bytes)
            .expect(&format!("Vector {}: Invalid ciphertext U point", i + 1));
        let ciphertext_v = hex::decode(&v.ciphertext_v_hex)
            .expect(&format!("Vector {}: Invalid ciphertext V hex", i + 1));

        // Load reconstructed DK and verify decryption works
        let dk_bytes: [u8; 48] = hex::decode(&v.reconstructed_dk_g1_hex)
            .expect(&format!("Vector {}: Invalid DK hex", i + 1))
            .try_into()
            .expect("DK should be 48 bytes");
        let dk = blstrs::G1Affine::from_compressed(&dk_bytes)
            .expect(&format!("Vector {}: Invalid DK point", i + 1));

        // Decrypt using high-level API
        let ciphertext = super::Ciphertext {
            u: ciphertext_u,
            v: ciphertext_v,
        };
        let decrypted = ibe_decrypt(&dk, &ciphertext);
        assert_eq!(
            decrypted,
            plaintext,
            "Vector {}: Decryption mismatch",
            i + 1
        );

        // Verify DK is valid using pairing check (high-level API)
        assert!(
            verify_decryption_key(&dk, &identity, &mpk),
            "Vector {}: DK pairing verification failed",
            i + 1
        );

        // Verify MSK correctly derives the expected DK
        let expected_dk = derive_decryption_key(&msk, &identity);
        assert_eq!(
            dk,
            expected_dk,
            "Vector {}: DK derivation from MSK mismatch",
            i + 1
        );

        println!("  ✅ Vector {} verified", i + 1);
    }

    println!(
        "✅ All {} golden vectors verified!",
        vectors.ibe_roundtrip_vectors.len()
    );
}
