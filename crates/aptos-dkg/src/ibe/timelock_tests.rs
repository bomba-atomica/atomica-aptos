// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Comprehensive tests for the timelock feature built on IBE.

use super::*;
use crate::ibe::{
    compute_identity, hash_to_g1, ibe_decrypt, ibe_encrypt, reconstruct_ibe_dk_from_g1_shares,
    Ciphertext,
};
use crate::pvss::input_secret::InputSecret;
use crate::pvss::scalar_elgamal::WeightedTranscript;
use crate::pvss::test_utils::setup_dealing;
use crate::pvss::traits::{SecretSharingConfig, Transcript as TranscriptTrait};
use crate::pvss::{Player, WeightedConfig};
use aptos_crypto::Uniform;
use blstrs::G2Projective;
use group::Curve;
use rand::thread_rng;
use std::ops::Mul;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("valid hex string")
}

#[test]
fn test_identity_computation_deterministic() {
    let id1 = compute_identity(0, 1000000000000);
    let id2 = compute_identity(0, 1000000000000);
    assert_eq!(id1, id2);
    assert_ne!(compute_identity(1, 1000000000000), id1);
    assert_ne!(compute_identity(0, 1000000000001), id1);
}

#[test]
fn test_identity_from_golden_vectors() {
    let test_cases = vec![
        (
            0u64,
            1000000000000u64,
            "cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355",
        ),
        (
            1,
            1000000000000,
            "1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1",
        ),
        (
            0,
            2000000000000,
            "ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6",
        ),
    ];
    for (id, deadline, expected) in test_cases {
        assert_eq!(hex::encode(&compute_identity(id, deadline)), expected);
    }
}

#[test]
fn test_hash_to_g1_different_identities_different_points() {
    let h1 = hash_to_g1(&compute_identity(0, 1000000000000));
    let h2 = hash_to_g1(&compute_identity(1, 1000000000000));
    assert_ne!(h1, h2);
}

#[test]
fn test_dk_share_computation_single_validator() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1];
    let wconfig = WeightedConfig::new(1, weights_usize).unwrap();
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
    let (sk_share, _) = transcript
        .decrypt_own_share(
            &wconfig,
            &Player { id: 0 },
            &dealing_args.dks[0],
            &dealing_args.pp,
        )
        .expect("should succeed");
    let identity = compute_identity(0, 1000000000000);
    let dk_share = hash_to_g1(&identity).mul(&sk_share[0].0.s).to_affine();
    assert_eq!(dk_share.to_compressed().len(), 48);
    let mpk = G2Projective::generator().mul(&secret).to_affine();
    assert!(verify_decryption_key(&dk_share, &identity, &mpk));
}

#[test]
fn test_reconstruct_dk_from_g1_shares_equal_weights() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1, 1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
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
    let identity = compute_identity(12345, 1000000000);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in 0..3 {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    assert_eq!(reconstructed, derive_decryption_key(&secret, &identity));
}

#[test]
fn test_reconstruct_dk_from_g1_shares_unequal_weights() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![2, 1, 2];
    let wconfig = WeightedConfig::new(3, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
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
    let identity = compute_identity(42, 1_000_000_000_000);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in 0..3 {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    assert_eq!(reconstructed, derive_decryption_key(&secret, &identity));
}

#[test]
fn test_reconstruct_dk_sparse_participation() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(2, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
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
    let identity = compute_identity(99, 2_000_000_000_000);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in [0, 2] {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    assert_eq!(reconstructed, derive_decryption_key(&secret, &identity));
}

#[test]
fn test_complete_timelock_workflow() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1, 1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();
    let mpk = G2Projective::generator().mul(&secret).to_affine();
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
    let identity = compute_identity(42, 1_000_000_000_000);
    let plaintext = b"Secret message for timelock release!";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in 0..3 {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    assert_eq!(ibe_decrypt(&reconstructed, &ciphertext), plaintext);
}

#[test]
fn test_complete_timelock_unequal_weights() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![2, 1, 2];
    let wconfig = WeightedConfig::new(3, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();
    let mpk = G2Projective::generator().mul(&secret).to_affine();
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
    let identity = compute_identity(123, 2_000_000_000_000);
    let plaintext = b"Weighted timelock test [2,1,2]";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in 0..3 {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    assert_eq!(ibe_decrypt(&reconstructed, &ciphertext), plaintext);
}

#[test]
fn test_multiple_timelocks() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1, 1, 1, 1];
    let wconfig = WeightedConfig::new(3, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();
    let mpk = G2Projective::generator().mul(&secret).to_affine();
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
    let identities = [
        compute_identity(0, 1_000_000_000_000),
        compute_identity(1, 1_000_000_000_000),
        compute_identity(0, 2_000_000_000_000),
    ];
    let plaintexts = [b"Msg1", b"Msg2", b"Msg3"];
    let ciphertexts: Vec<Ciphertext> = identities
        .iter()
        .zip(plaintexts.iter())
        .map(|(id, msg)| ibe_encrypt(&mpk, id, *msg, &mut rng))
        .collect();
    for (i, identity) in identities.iter().enumerate() {
        let h = hash_to_g1(identity);
        let mut vids = Vec::new();
        let mut shares = Vec::new();
        for idx in 0..3 {
            let (sk_share, _) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: idx },
                    &dealing_args.dks[idx],
                    &dealing_args.pp,
                )
                .expect("succeed");
            let player = wconfig.get_player(idx);
            let start = wconfig.get_player_starting_index(&player);
            for (j, s) in sk_share.iter().enumerate() {
                let dk = h.mul(&s.0.s).to_affine();
                vids.push((start + j) as u64);
                shares.push(dk.to_compressed().to_vec());
            }
        }
        let reconstructed =
            reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
        assert_eq!(ibe_decrypt(&reconstructed, &ciphertexts[i]), plaintexts[i]);
    }
}

#[test]
#[should_panic(expected = "EmptyShares")]
fn test_reconstruct_empty_shares_panics() {
    reconstruct_ibe_dk_from_g1_shares(&vec![], &vec![], 5).unwrap();
}

#[test]
#[should_panic(expected = "ValidatorIndicesSharesMismatch")]
fn test_reconstruct_mismatched_lengths_panics() {
    let ids = vec![0u64, 1];
    let shares = vec![vec![0u8; 48]];
    reconstruct_ibe_dk_from_g1_shares(&ids, &shares, 2).unwrap();
}

#[test]
#[should_panic(expected = "InvalidG1Share")]
fn test_reconstruct_invalid_share_length_panics() {
    let ids = vec![0u64];
    let shares = vec![vec![0u8; 32]];
    reconstruct_ibe_dk_from_g1_shares(&ids, &shares, 1).unwrap();
}

#[test]
#[should_panic(expected = "not a valid G1 point")]
fn test_reconstruct_invalid_g1_point() {
    let ids = vec![0u64];
    let shares = vec![vec![0u8; 48]];
    let _ = reconstruct_ibe_dk_from_g1_shares(&ids, &shares, 1);
}

#[test]
fn test_golden_vector_roundtrip_1() {
    let shares_hex = vec![
        vec!["9722f3fe074ff0467af66bbb6564aaeec41ac369dbc55a520e1197e2cabd01489fac2b5260c049e9ed26fdd872391d2d"],
        vec!["b0cc6092fc45df1b1318952c08dd2a877b5b19deb725b63cfc41c48e84da6e8f77600efe4d38aac33273a87775112a31"],
        vec!["ac20a63c6b62ed15a3424d85af25eb006795eba2172996529afbb70d29d1a57066a37ee1c98dea843fa2c998cb6b3919"],
    ];
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for (idx, s_list) in shares_hex.iter().enumerate() {
        for sh in s_list {
            vids.push(idx as u64);
            shares.push(hex_to_bytes(sh));
        }
    }
    let reconstructed = reconstruct_ibe_dk_from_g1_shares(&vids, &shares, 5).expect("succeed");
    let expected = hex_to_bytes("a46066de7604d2d51825cd139d720523d5584654cda567f80c3779304d2dec8842b41a7feec0db2a202409f45492593c");
    assert_eq!(reconstructed.to_compressed().to_vec(), expected);
}

#[test]
fn test_golden_vector_roundtrip_3() {
    let shares_hex = vec![
        vec!["8723f166aabb441baeb0df1eddfd332c3f5ec3a0b41ae92c54aee283021deb98ac79248dbabd5244bde583b553a4222b", "aff1dc2c26fcb7f48a61f27e34933e8818c3e8f3905ee9cda4956b89d343bc751ed39be795a7762a5aefe742ef192836"],
        vec!["8a96fb8faed4eedb44496140cf2a47c09a56eca0da3a61e8306d11554ca90ba06493ec1d4523cf39326451dbb67d4549"],
        vec!["aff15e922d0418a29c420f384db7524fdf52bfdb16b34710c07b38a40167711c660af07d6261a031a264b78acc63214f", "a24237a89cf507e05553e93950cac155d24496c8d7b5568d4443223826916141c1517ff43873b0fec0a03613f90bec0a"],
    ];
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for (j, sh) in shares_hex[0].iter().enumerate() {
        vids.push(j as u64);
        shares.push(hex_to_bytes(sh));
    }
    vids.push(2);
    shares.push(hex_to_bytes(&shares_hex[1][0]));
    for (j, sh) in shares_hex[2].iter().enumerate() {
        vids.push((3 + j) as u64);
        shares.push(hex_to_bytes(sh));
    }
    let reconstructed = reconstruct_ibe_dk_from_g1_shares(&vids, &shares, 5).expect("succeed");
    let expected = hex_to_bytes("afbaa0adb5ab4ef5d1c72e1aab6497f1fabd61d8f24a7c642593978e4ff18a080c10228fa8a61a8ec9f5ad0e15de1a76");
    assert_eq!(reconstructed.to_compressed().to_vec(), expected);
}

#[test]
fn test_reconstruction_many_validators() {
    let mut rng = thread_rng();
    let weights_usize: Vec<usize> = vec![1; 10];
    let wconfig = WeightedConfig::new(7, weights_usize.clone()).unwrap();
    let total_weight: u64 = weights_usize.iter().map(|w| *w as u64).sum();
    let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
    let input_secret = InputSecret::generate(&mut rng);
    let secret = *input_secret.get_secret_a();
    let mpk = G2Projective::generator().mul(&secret).to_affine();
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
    let identity = compute_identity(999, 9_000_000_000_000);
    let h_identity = hash_to_g1(&identity);
    let mut vids = Vec::new();
    let mut shares = Vec::new();
    for idx in 0..7 {
        let (sk_share, _) = transcript
            .decrypt_own_share(
                &wconfig,
                &Player { id: idx },
                &dealing_args.dks[idx],
                &dealing_args.pp,
            )
            .expect("succeed");
        let player = wconfig.get_player(idx);
        let start = wconfig.get_player_starting_index(&player);
        for (j, s) in sk_share.iter().enumerate() {
            let dk = h_identity.mul(&s.0.s).to_affine();
            vids.push((start + j) as u64);
            shares.push(dk.to_compressed().to_vec());
        }
    }
    let reconstructed =
        reconstruct_ibe_dk_from_g1_shares(&vids, &shares, total_weight).expect("succeed");
    let expected = derive_decryption_key(&secret, &identity);
    assert_eq!(reconstructed, expected);
    let plaintext = b"Large validator set timelock test";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
    assert_eq!(ibe_decrypt(&reconstructed, &ciphertext), plaintext);
}

#[test]
#[ignore]
fn generate_timelock_golden_vectors_for_move() {
    use rand::SeedableRng;

    println!("\n========================================");
    println!("TIMELOCK GOLDEN VECTORS FOR MOVE TESTS");
    println!("========================================\n");

    let seed = 42u64;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    println!("// ============================================================");
    println!("// TIMELOCK-SPECIFIC GOLDEN VECTORS");
    println!("// Generated for Move unit tests");
    println!("// Seed: {}", seed);
    println!("// ============================================================\n");

    let test_cases = vec![(
        "timelock_basic",
        0u64,
        1000000000000u64,
        vec![1, 1, 1, 1, 1],
        3,
    )];

    for (name, timelock_id, deadline_us, weights, threshold) in test_cases {
        let weights_usize: Vec<usize> = weights.iter().map(|w| *w as usize).collect();
        let total_weight: u64 = weights.iter().sum();
        let wconfig = WeightedConfig::new(threshold, weights_usize.clone()).unwrap();
        let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
        let input_secret = InputSecret::generate(&mut rng);
        let secret = *input_secret.get_secret_a();
        let mpk = G2Projective::generator().mul(&secret).to_affine();
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
        let identity = compute_identity(timelock_id, deadline_us);
        let h_identity = hash_to_g1(&identity);

        println!("// {}", name);
        println!("// Timelock ID: {}, Deadline: {}", timelock_id, deadline_us);
        println!(
            "public fun {}_identity(): vector<u8> {{ x\"{}\" }}",
            name,
            hex::encode(&identity)
        );
        println!(
            "public fun {}_mpk(): vector<u8> {{ x\"{}\" }}",
            name,
            hex::encode(mpk.to_compressed())
        );
        println!("public fun {}_threshold(): u64 {{ {} }}", name, threshold);
        println!(
            "public fun {}_total_weight(): u64 {{ {} }}",
            name, total_weight
        );
        println!(
            "public fun {}_validator_weights(): vector<u64> {{ vector[{}] }}",
            name,
            weights
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let mut dk_shares: Vec<Vec<Vec<u8>>> = Vec::new();
        for idx in 0..weights_usize.len() {
            let (sk_share, _) = transcript
                .decrypt_own_share(
                    &wconfig,
                    &Player { id: idx },
                    &dealing_args.dks[idx],
                    &dealing_args.pp,
                )
                .expect("succeed");
            let mut validator_shares: Vec<Vec<u8>> = Vec::new();
            for s in sk_share.iter() {
                let dk = h_identity.mul(&s.0.s).to_affine();
                validator_shares.push(dk.to_compressed().to_vec());
            }
            dk_shares.push(validator_shares);
        }

        println!(
            "public fun {}_dk_shares(): vector<vector<vector<u8>>> {{",
            name
        );
        println!("    let v = vector::empty<vector<vector<u8>>>();");
        for validator_shares in &dk_shares {
            println!("    let inner = vector::empty<vector<u8>>();");
            for share in validator_shares {
                println!(
                    "        vector::push_back(&mut inner, x\"{}\");",
                    hex::encode(share)
                );
            }
            println!("    vector::push_back(&mut v, inner);");
        }
        println!("    v");
        println!("}}");

        let all_shares: Vec<u64> = dk_shares
            .iter()
            .enumerate()
            .flat_map(|(idx, shares)| shares.iter().map(move |_| idx as u64))
            .collect();
        let all_shares_bytes: Vec<Vec<u8>> = dk_shares
            .iter()
            .flat_map(|shares| shares.iter().cloned())
            .collect();
        let reconstructed =
            reconstruct_ibe_dk_from_g1_shares(&all_shares, &all_shares_bytes, total_weight)
                .expect("succeed");

        println!(
            "public fun {}_reconstructed_dk(): vector<u8> {{ x\"{}\" }}",
            name,
            hex::encode(reconstructed.to_compressed())
        );

        let plaintext = b"Timelock test message";
        let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);
        println!(
            "public fun {}_ciphertext_u(): vector<u8> {{ x\"{}\" }}",
            name,
            hex::encode(ciphertext.u.to_compressed())
        );
        println!(
            "public fun {}_ciphertext_v(): vector<u8> {{ x\"{}\" }}",
            name,
            hex::encode(&ciphertext.v)
        );
        println!();
    }
}
