// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Paranoid tests for IBE DK reconstruction.
//!
//! These tests verify the DK reconstruction by:
//! 1. Using real PVSS transcripts to generate valid shares
//! 2. Verifying end-to-end encrypt/decrypt with reconstructed DKs
//! 3. Testing error handling for invalid inputs
//!
//! The goal is maximum confidence in the correctness of the native function.

use aptos_crypto::Uniform;
use aptos_dkg::pvss::traits::Transcript as TranscriptTrait;
use blstrs::Scalar;
use group::Group;
use rand::SeedableRng;
use std::ops::Mul;

/// Test golden vector file can be loaded and parsed.
#[test]
fn test_golden_vectors_loadable() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => {
            println!("⚠️  Skipping golden vector test - file not found");
            return;
        },
    };

    assert!(
        !golden_vectors.ibe_roundtrip_vectors.is_empty(),
        "Should have at least one roundtrip vector"
    );

    println!(
        "✅ Loaded {} golden vectors",
        golden_vectors.ibe_roundtrip_vectors.len()
    );
}

/// Test reconstruction with 3 validators (threshold 3, equal weights [1,1,1]).
/// Uses real PVSS transcript to generate valid shares.
#[test]
fn test_dk_reconstruction_three_validators_equal() {
    use aptos_dkg::pvss::input_secret::InputSecret;
    use aptos_dkg::pvss::scalar_elgamal::WeightedTranscript;
    use aptos_dkg::pvss::test_utils::setup_dealing;
    use aptos_dkg::pvss::Player;
    use aptos_dkg::pvss::WeightedConfig;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEADBEEF);

    // 3 validators, each weight 1, threshold 3
    let weights: Vec<usize> = vec![1, 1, 1];
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
    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    // Extract scalar shares using the public scalar() method
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            sk_shares
                .iter()
                .map(|sk_share| *sk_share.scalar())
                .collect()
        })
        .collect();

    // Validator indices and weights for reconstruction
    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = weights.iter().map(|w| *w as u64).collect();

    // Use test_reconstruct_ibe_from_secret_shares
    let reconstructed_dk = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    )
    .expect("test_reconstruct_ibe_from_secret_shares should succeed with valid shares");

    // Verify encryption/decrypt roundtrip
    let mpk = blstrs::G2Projective::generator().mul(&secret).into();
    let plaintext = b"Test 3 validators equal weights";
    let ciphertext = aptos_dkg::ibe::ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = aptos_dkg::ibe::ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ Three validators equal weights test passed");
}

/// Test reconstruction with unequal weights [2, 1, 2].
#[test]
fn test_dk_reconstruction_unequal_weights_215() {
    use aptos_dkg::pvss::input_secret::InputSecret;
    use aptos_dkg::pvss::scalar_elgamal::WeightedTranscript;
    use aptos_dkg::pvss::test_utils::setup_dealing;
    use aptos_dkg::pvss::Player;
    use aptos_dkg::pvss::WeightedConfig;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xFEEDFACE);

    // 3 validators, weights [2, 1, 2], threshold 3
    let weights: Vec<usize> = vec![2, 1, 2];
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

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    // Extract scalar shares using the public scalar() method
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            sk_shares
                .iter()
                .map(|sk_share| *sk_share.scalar())
                .collect()
        })
        .collect();

    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = weights.iter().map(|w| *w as u64).collect();

    let reconstructed_dk = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    )
    .expect("test_reconstruct_ibe_from_secret_shares should succeed with valid shares");

    // Verify roundtrip
    let mpk = blstrs::G2Projective::generator().mul(&secret).into();
    let plaintext = b"Test unequal weights [2,1,2]";
    let ciphertext = aptos_dkg::ibe::ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = aptos_dkg::ibe::ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ Unequal weights [2, 1, 2] test passed");
}

/// Test reconstruction with 5 validators (threshold 3).
#[test]
fn test_dk_reconstruction_five_validators() {
    use aptos_dkg::pvss::input_secret::InputSecret;
    use aptos_dkg::pvss::scalar_elgamal::WeightedTranscript;
    use aptos_dkg::pvss::test_utils::setup_dealing;
    use aptos_dkg::pvss::Player;
    use aptos_dkg::pvss::WeightedConfig;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xAABBCCDD);

    // 5 validators, each weight 1, threshold 3
    let weights: Vec<usize> = vec![1, 1, 1, 1, 1];
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

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    // Extract scalar shares using the public scalar() method
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            sk_shares
                .iter()
                .map(|sk_share| *sk_share.scalar())
                .collect()
        })
        .collect();

    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = weights.iter().map(|w| *w as u64).collect();

    let reconstructed_dk = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    )
    .expect("test_reconstruct_ibe_from_secret_shares should succeed with valid shares");

    let mpk = blstrs::G2Projective::generator().mul(&secret).into();
    let plaintext = b"Test 5 validators threshold 3";
    let ciphertext = aptos_dkg::ibe::ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = aptos_dkg::ibe::ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ Five validators test passed");
}

/// Test reconstruction with sparse validator indices [0, 2].
#[test]
fn test_dk_reconstruction_sparse_indices() {
    use aptos_dkg::pvss::input_secret::InputSecret;
    use aptos_dkg::pvss::scalar_elgamal::WeightedTranscript;
    use aptos_dkg::pvss::test_utils::setup_dealing;
    use aptos_dkg::pvss::Player;
    use aptos_dkg::pvss::WeightedConfig;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xCAFEBABE);

    // 4 validators, each weight 1, threshold 2
    let weights: Vec<usize> = vec![1, 1, 1, 1];
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

    // Decrypt shares from validators 0 and 2 (sparse)
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

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    // Extract scalar shares using the public scalar() method
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            sk_shares
                .iter()
                .map(|sk_share| *sk_share.scalar())
                .collect()
        })
        .collect();

    let validator_indices: Vec<u64> = vec![0, 2];
    let full_weights: Vec<u64> = weights.iter().map(|w| *w as u64).collect();

    let reconstructed_dk = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity,
    )
    .expect("test_reconstruct_ibe_from_secret_shares should succeed with valid shares");

    let mpk = blstrs::G2Projective::generator().mul(&secret).into();
    let plaintext = b"Test sparse indices [0,2]";
    let ciphertext = aptos_dkg::ibe::ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let decrypted = aptos_dkg::ibe::ibe_decrypt(&reconstructed_dk, &ciphertext);
    assert_eq!(decrypted, plaintext);

    println!("✅ Sparse indices test passed");
}

/// Test that different identities produce different DKs.
#[test]
fn test_dk_reconstruction_different_identities() {
    use aptos_dkg::pvss::input_secret::InputSecret;
    use aptos_dkg::pvss::scalar_elgamal::WeightedTranscript;
    use aptos_dkg::pvss::test_utils::setup_dealing;
    use aptos_dkg::pvss::Player;
    use aptos_dkg::pvss::WeightedConfig;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x12345678);

    // 3 validators, equal weights
    let weights: Vec<usize> = vec![1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights.clone()).unwrap();
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

    // Extract scalar shares using the public scalar() method
    let scalar_shares: Vec<Vec<Scalar>> = shares
        .iter()
        .map(|(_player, sk_shares)| {
            sk_shares
                .iter()
                .map(|sk_share| *sk_share.scalar())
                .collect()
        })
        .collect();

    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = weights.iter().map(|w| *w as u64).collect();

    let identity1 = [0x01u8; 32];
    let identity2 = [0x02u8; 32];
    let identity3 = [0x03u8; 32];

    let dk1 = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares.clone(),
        &full_weights,
        total_weight,
        &identity1,
    )
    .expect("should succeed");
    let dk2 = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares.clone(),
        &full_weights,
        total_weight,
        &identity2,
    )
    .expect("should succeed");
    let dk3 = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        total_weight,
        &identity3,
    )
    .expect("should succeed");

    assert!(
        dk1.to_compressed() != dk2.to_compressed(),
        "Different identities should produce different DKs"
    );
    assert!(
        dk2.to_compressed() != dk3.to_compressed(),
        "Different identities should produce different DKs"
    );

    println!("✅ Different identities produce different DKs test passed");
}

/// Test that scalar shares must match weights.
#[test]
fn test_dk_reconstruction_share_weight_mismatch() {
    let weights: Vec<u64> = vec![1, 1, 1];
    let total_weight = 3;
    let validator_indices: Vec<u64> = vec![0, 1, 2];

    // Provide 2 shares for validator 0 but weight is 1
    let scalar_shares: Vec<Vec<Scalar>> = vec![
        vec![Scalar::from(1u64), Scalar::from(2u64)], // 2 shares for weight 1
        vec![Scalar::from(3u64)],
        vec![Scalar::from(4u64)],
    ];

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    let result = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &weights,
        total_weight,
        &identity,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        aptos_dkg::ibe::IbeError::ShareWeightMismatch {
            validator_index: 0,
            shares_count: 2,
            expected_weight: 1,
        } => {
            println!("✅ Share weight mismatch error returned correctly");
        },
        _ => panic!(
            "Expected ShareWeightMismatch for validator 0 with 2 shares and weight 1, got: {}",
            err
        ),
    }
}

/// Test that empty shares return error.
#[test]
fn test_dk_reconstruction_empty_shares() {
    let weights: Vec<u64> = vec![1, 1, 1];
    let total_weight = 3;
    let validator_indices: Vec<u64> = vec![];
    let scalar_shares: Vec<Vec<Scalar>> = vec![];

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    let result = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &weights,
        total_weight,
        &identity,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        aptos_dkg::ibe::IbeError::EmptyShares => {
            println!("✅ Empty shares error returned correctly");
        },
        _ => panic!("Expected EmptyShares error, got: {}", err),
    }
}

/// Test that mismatched indices and shares return error.
#[test]
fn test_dk_reconstruction_indices_shares_mismatch() {
    let weights: Vec<u64> = vec![1, 1, 1];
    let total_weight = 3;
    let validator_indices: Vec<u64> = vec![0, 1, 2]; // 3 indices
    let scalar_shares: Vec<Vec<Scalar>> = vec![
        vec![Scalar::from(1u64)], // Only 2 share vectors
        vec![Scalar::from(2u64)],
    ];

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    let result = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &weights,
        total_weight,
        &identity,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        aptos_dkg::ibe::IbeError::ValidatorIndicesSharesMismatch {
            indices_len: 3,
            shares_len: 2,
        } => {
            println!("✅ Validator indices/shares mismatch error returned correctly");
        },
        _ => panic!(
            "Expected ValidatorIndicesSharesMismatch(3, 2), got: {}",
            err
        ),
    }
}

/// Test that total_weight mismatch with sum of weights returns error.
#[test]
fn test_dk_reconstruction_weight_sum_mismatch() {
    let weights: Vec<u64> = vec![1, 1, 1]; // Sum = 3
    let total_weight = 5; // But we say total is 5
    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let scalar_shares: Vec<Vec<Scalar>> = vec![
        vec![Scalar::from(1u64)],
        vec![Scalar::from(2u64)],
        vec![Scalar::from(3u64)],
    ];

    let identity = aptos_dkg::ibe::compute_identity(1, 1704067200000000);

    let result = aptos_dkg::ibe::test_reconstruct_ibe_from_secret_shares(
        &validator_indices,
        &scalar_shares,
        &weights,
        total_weight,
        &identity,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        aptos_dkg::ibe::IbeError::WeightSumMismatch {
            total_weight: 5,
            computed_sum: 3,
        } => {
            println!("✅ Weight sum mismatch error returned correctly");
        },
        _ => panic!("Expected WeightSumMismatch(5, 3), got: {}", err),
    }
}
