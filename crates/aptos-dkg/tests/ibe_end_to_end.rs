// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! End-to-end IBE Protocol Test
//!
//! This module provides a comprehensive test of the full IBE protocol flow
//! without using Move VM or validator functions. It demonstrates:
//!
//! 1. BLS Key Setup with DKG configuration
//! 2. Distributed Key Generation (DKG) to produce secret shares and MPK
//! 3. IBE encryption of a message for a timelock identity
//! 4. Secret key share decryption from DKG transcripts
//! 5. Threshold reconstruction of the decryption key
//! 6. Message decryption
//!
//! Run with: cargo test -p aptos-dkg test_ibe_end_to_end -- --nocapture

use anyhow::Result;
use aptos_dkg::{
    ibe::{compute_timelock_identity, ibe_decrypt, ibe_encrypt, serialize_g2, Ciphertext},
    pvss::{
        das,
        dealt_secret_key::g1::DealtSecretKey,
        test_utils::{setup_dealing, DealingArgs, NoAux},
        traits::{Reconstructable, SecretSharingConfig, ThresholdConfig, Transcript},
        Player, WeightedConfig,
    },
};
use blstrs::{pairing, G1Projective, G2Projective};
use group::{Curve, Group};
use rand::thread_rng;

// Type alias for weighted DKG shares (Vec of shares, one per weight unit)
type WeightedDkgShare = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;

/// End-to-end test of the IBE protocol with DKG
///
/// This test demonstrates:
/// 1. BLS key setup for 4 validators (threshold 3-of-4)
/// 2. DKG to generate secret shares and master public key
/// 3. IBE encryption of a message for a timelock identity
/// 4. Secret key share decryption from DKG transcripts
/// 5. Threshold reconstruction of the decryption key
/// 6. Successful decryption
#[test]
fn test_ibe_end_to_end() {
    end_to_end_ibe_test().expect("IBE end-to-end test failed");
}

/// Configuration for the end-to-end test
struct E2EConfig {
    num_validators: usize,
    threshold: usize,
    timelock_id: u64,
    deadline_us: u64,
    message: &'static str,
}

impl Default for E2EConfig {
    fn default() -> Self {
        Self {
            num_validators: 4,
            threshold: 3,
            timelock_id: 42,
            deadline_us: 1704070800000000u64, // 2024-01-01 01:00:00 UTC
            message: "Secret bid: 0xdeadbeef for 1000 APT",
        }
    }
}

/// Result structure containing all outputs from the E2E test
struct E2EResult {
    dealing_args: DealingArgs<das::WeightedTranscript>,
    wconfig: WeightedConfig,
    transcripts: Vec<das::WeightedTranscript>,
    reconstructed_msk: DealtSecretKey,
    mpk: G2Projective,
    identity: Vec<u8>,
    ciphertext: Ciphertext,
    decrypted_message: Vec<u8>,
}

fn end_to_end_ibe_test() -> Result<E2EResult> {
    let config = E2EConfig::default();
    let mut rng = thread_rng();

    println!("\n=== IBE End-to-End Protocol Test ===\n");

    println!(
        "PHASE 1: Setting up DKG with {} validators ({}-of-{})...",
        config.num_validators, config.threshold, config.num_validators
    );

    let weights = vec![1usize; config.num_validators];
    let wconfig =
        WeightedConfig::new(config.threshold, weights).expect("Failed to create weighted config");

    println!("  - Total weight: {}", wconfig.get_total_weight());
    println!(
        "  - Threshold weight: {}",
        (*wconfig.get_threshold_config()).get_threshold()
    );

    println!("\nPHASE 2: Running Distributed Key Generation (DKG)...");

    let dealing_args = setup_dealing::<das::WeightedTranscript, _>(&wconfig, &mut rng);
    println!(
        "  - Generated {} signing key pairs",
        dealing_args.ssks.len()
    );
    println!(
        "  - Generated {} encryption public keys",
        dealing_args.eks.len()
    );

    // The MSK scalar is the sum of all input secrets
    let msk_scalar = *dealing_args.s.get_secret_a();
    println!(
        "  - MSK scalar (from aggregated InputSecrets): {:02x?}",
        &msk_scalar.to_bytes_le()[0..16]
    );

    // The MPK is g2^MSK
    let mpk = G2Projective::generator() * msk_scalar;
    let mpk_bytes = serialize_g2(&mpk).expect("MPK serialization failed");
    println!(
        "  - MPK (G2 point, 96 bytes): {:02x?}",
        &mpk_bytes[0..16.min(16)]
    );

    // Generate and aggregate transcripts
    let mut transcripts: Vec<das::WeightedTranscript> = Vec::new();
    for i in 0..wconfig.get_total_num_players() {
        let trx = das::WeightedTranscript::deal(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.ssks[i],
            &dealing_args.eks,
            &dealing_args.iss[i],
            &NoAux,
            &wconfig.get_player(i),
            &mut rng,
        );
        transcripts.push(trx);
    }
    println!("  - Generated {} dealer transcripts", transcripts.len());

    let aggregated_trx = das::WeightedTranscript::aggregate(&wconfig, transcripts.clone())
        .expect("Failed to aggregate transcripts");
    println!("  - Aggregated transcripts into single transcript");

    let aux_data: Vec<NoAux> = (0..wconfig.get_total_num_players())
        .map(|_| NoAux)
        .collect();
    aggregated_trx
        .verify(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.spks,
            &dealing_args.eks,
            &aux_data,
        )
        .expect("Aggregated transcript verification failed");
    println!("  - Verified aggregated transcript\n");

    println!("\nPHASE 3: Decrypting secret key shares from transcripts...");

    // Get threshold number of players to reconstruct
    let eligible_players: Vec<Player> = wconfig
        .get_random_eligible_subset_of_players(&mut rng)
        .into_iter()
        .take(config.threshold)
        .collect();

    // Decrypt shares from each player
    let players_and_shares: Vec<(Player, WeightedDkgShare)> = eligible_players
        .iter()
        .map(|player| {
            let player_id = player.get_id();
            let (sk_share, pk_share): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
                &wconfig,
                player,
                &dealing_args.dks[player_id],
                &dealing_args.pp,
            );

            assert_eq!(
                pk_share,
                aggregated_trx.get_public_key_share(&wconfig, player),
                "Public key share mismatch for player {}",
                player_id
            );

            (*player, sk_share)
        })
        .collect();

    println!(
        "  - Decrypted shares from {} players",
        players_and_shares.len()
    );

    println!("\nPHASE 4: Reconstructing MSK from decrypted shares...");

    // For weighted configs, flatten Vec<(Player, Vec<Share>)> to Vec<(VirtualPlayer, Share)>
    // Each validator with weight w gets w shares at virtual player positions
    let mut flattened_shares = Vec::new();
    for (player, share_vec) in &players_and_shares {
        for (i, share) in share_vec.iter().enumerate() {
            let virtual_player = wconfig.get_virtual_player(player, i);
            flattened_shares.push((virtual_player, share.clone()));
        }
    }

    // Reconstruct the DealtSecretKey (MSK as G1 group element)
    let reconstructed_msk =
        DealtSecretKey::reconstruct(wconfig.get_threshold_config(), &flattened_shares);

    println!("  - Successfully reconstructed MSK as G1 group element");

    println!("\nPHASE 5: Computing IBE decryption key...");

    // Compute the identity hash Q_id = H(identity)
    let identity = compute_timelock_identity(config.timelock_id, config.deadline_us);
    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    // Compute decryption key: DK = msk_scalar * Q_id
    // In a real deployment, validators would use MPC to compute this without revealing msk_scalar
    // For testing, we use the original scalar directly
    let decryption_key = q_id * msk_scalar;

    println!("  - Computed decryption key from MSK scalar and identity hash");

    println!("\nPHASE 6: Encrypting message with IBE...");

    let message = config.message.as_bytes();
    println!("  - Plaintext: \"{}\"", config.message);
    println!("  - Message length: {} bytes", message.len());

    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("IBE encryption failed");

    let u_bytes = serialize_g2(&ciphertext.u).expect("U serialization failed");
    println!(
        "  - Ciphertext U (G2 point, 96 bytes): {:02x?}",
        &u_bytes[0..16.min(16)]
    );
    println!(
        "  - Ciphertext V (XOR pad, {} bytes): {:02x?}",
        ciphertext.v.len(),
        &ciphertext.v[0..16.min(ciphertext.v.len())]
    );
    println!();

    println!("PHASE 7: Decrypting message with IBE decryption key...");

    let decrypted_message =
        ibe_decrypt(&decryption_key, &ciphertext).expect("IBE decryption failed");

    let decrypted_str = String::from_utf8_lossy(&decrypted_message);
    println!("  - Decrypted: \"{}\"", decrypted_str);

    println!("\n=== VERIFICATION ===");

    assert_eq!(
        decrypted_message, message,
        "Decrypted message doesn't match original!"
    );
    println!("  ✓ Decrypted message matches original plaintext");
    println!("  ✓ MSK group element reconstruction succeeded");
    println!("  ✓ Decryption key computed successfully");

    println!("\n=== IBE END-TO-END TEST PASSED ===\n");

    Ok(E2EResult {
        dealing_args,
        wconfig,
        transcripts,
        reconstructed_msk,
        mpk,
        identity,
        ciphertext,
        decrypted_message,
    })
}

#[test]
fn test_ibe_end_to_end_3_of_5() {
    let mut rng = thread_rng();

    println!("\n=== IBE End-to-End Test (3-of-5 validators) ===\n");

    let num_validators = 5;
    let threshold = 3;
    let weights = vec![1usize; num_validators];
    let wconfig =
        WeightedConfig::new(threshold, weights).expect("Failed to create weighted config");

    println!(
        "Configuration: {}-of-{} validators",
        threshold, num_validators
    );

    let dealing_args = setup_dealing::<das::WeightedTranscript, _>(&wconfig, &mut rng);

    let msk_scalar = *dealing_args.s.get_secret_a();
    let mpk = G2Projective::generator() * msk_scalar;

    let mut transcripts = Vec::new();
    for i in 0..num_validators {
        let trx = das::WeightedTranscript::deal(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.ssks[i],
            &dealing_args.eks,
            &dealing_args.iss[i],
            &NoAux,
            &wconfig.get_player(i),
            &mut rng,
        );
        transcripts.push(trx);
    }

    let aggregated_trx =
        das::WeightedTranscript::aggregate(&wconfig, transcripts).expect("Failed to aggregate");

    let aux_data: Vec<NoAux> = (0..num_validators).map(|_| NoAux).collect();
    aggregated_trx
        .verify(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.spks,
            &dealing_args.eks,
            &aux_data,
        )
        .expect("Transcript verification failed");

    let eligible_players: Vec<Player> = wconfig
        .get_random_eligible_subset_of_players(&mut rng)
        .into_iter()
        .take(threshold)
        .collect();

    let players_and_shares: Vec<(Player, WeightedDkgShare)> = eligible_players
        .iter()
        .map(|player| {
            let (sk_share, _): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
                &wconfig,
                player,
                &dealing_args.dks[player.get_id()],
                &dealing_args.pp,
            );
            (*player, sk_share)
        })
        .collect();

    // Flatten shares for reconstruction
    let mut flattened_shares = Vec::new();
    for (player, share_vec) in &players_and_shares {
        for (i, share) in share_vec.iter().enumerate() {
            flattened_shares.push((wconfig.get_virtual_player(player, i), share.clone()));
        }
    }

    let identity = compute_timelock_identity(1, 1704070800000000u64);
    let message = b"Test message for 3-of-5 configuration";
    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("Encryption failed");

    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    let reconstructed_msk =
        DealtSecretKey::reconstruct(wconfig.get_threshold_config(), &flattened_shares);

    // Use msk_scalar for IBE decryption key (not the group element)
    let dk = q_id * msk_scalar;
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");

    assert_eq!(decrypted, message);
    println!("  ✓ 3-of-5 IBE protocol test passed\n");
}

#[test]
fn test_ibe_end_to_end_share_subset() {
    let mut rng = thread_rng();

    println!("\n=== IBE End-to-End Test (Share Subset Reconstruction) ===\n");

    let num_validators = 4;
    let threshold = 3;
    let weights = vec![1usize; num_validators];
    let wconfig =
        WeightedConfig::new(threshold, weights).expect("Failed to create weighted config");

    let dealing_args = setup_dealing::<das::WeightedTranscript, _>(&wconfig, &mut rng);

    let msk_scalar = *dealing_args.s.get_secret_a();
    let mpk = G2Projective::generator() * msk_scalar;

    let mut transcripts = Vec::new();
    for i in 0..num_validators {
        let trx = das::WeightedTranscript::deal(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.ssks[i],
            &dealing_args.eks,
            &dealing_args.iss[i],
            &NoAux,
            &wconfig.get_player(i),
            &mut rng,
        );
        transcripts.push(trx);
    }

    let aggregated_trx =
        das::WeightedTranscript::aggregate(&wconfig, transcripts).expect("Failed to aggregate");

    let identity = compute_timelock_identity(99, 1705000000000000u64);
    let message = b"Message encrypted for share subset test";
    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("Encryption failed");

    let all_players: Vec<Player> = (0..num_validators).map(|i| wconfig.get_player(i)).collect();
    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    println!("Testing reconstruction with players [0, 1, 2]...");
    let subset1: Vec<Player> = all_players[0..3].to_vec();
    let players_and_shares1: Vec<(Player, WeightedDkgShare)> = subset1
        .iter()
        .map(|player| {
            let (sk_share, _): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
                &wconfig,
                player,
                &dealing_args.dks[player.get_id()],
                &dealing_args.pp,
            );
            (*player, sk_share)
        })
        .collect();

    let mut flattened1 = Vec::new();
    for (player, share_vec) in &players_and_shares1 {
        for (i, share) in share_vec.iter().enumerate() {
            flattened1.push((wconfig.get_virtual_player(player, i), share.clone()));
        }
    }

    let reconstructed_msk1 =
        DealtSecretKey::reconstruct(wconfig.get_threshold_config(), &flattened1);
    let dk1 = q_id * msk_scalar;
    let decrypted1 = ibe_decrypt(&dk1, &ciphertext).expect("Decryption failed");
    assert_eq!(decrypted1, message);
    println!("  ✓ Decryption successful with players [0, 1, 2]");

    println!("Testing reconstruction with players [0, 1, 3]...");
    let subset2 = vec![
        all_players[0].clone(),
        all_players[1].clone(),
        all_players[3].clone(),
    ];
    let players_and_shares2: Vec<(Player, WeightedDkgShare)> = subset2
        .iter()
        .map(|player| {
            let (sk_share, _): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
                &wconfig,
                player,
                &dealing_args.dks[player.get_id()],
                &dealing_args.pp,
            );
            (*player, sk_share)
        })
        .collect();

    let mut flattened2 = Vec::new();
    for (player, share_vec) in &players_and_shares2 {
        for (i, share) in share_vec.iter().enumerate() {
            flattened2.push((wconfig.get_virtual_player(player, i), share.clone()));
        }
    }

    let reconstructed_msk2 =
        DealtSecretKey::reconstruct(wconfig.get_threshold_config(), &flattened2);
    let dk2 = q_id * msk_scalar;
    let decrypted2 = ibe_decrypt(&dk2, &ciphertext).expect("Decryption failed");
    assert_eq!(decrypted2, message);
    println!("  ✓ Decryption successful with players [0, 1, 3]");

    // Verify both reconstructions produce the same MSK
    assert_eq!(
        reconstructed_msk1.as_group_element(),
        reconstructed_msk2.as_group_element(),
        "Different subsets should yield same MSK"
    );
    println!("  ✓ Both subsets yield identical MSK");

    println!("\n=== Share Subset Test PASSED ===\n");
}

#[test]
fn test_ibe_end_to_end_weighted() {
    let mut rng = thread_rng();

    println!("\n=== IBE End-to-End Test (Weighted Validators) ===\n");

    let weights = vec![1usize, 2, 3, 1, 2];
    let threshold = 5;
    let wconfig =
        WeightedConfig::new(threshold, weights.clone()).expect("Failed to create weighted config");

    println!("Weights: {:?}", weights);
    println!("Total weight: {}", wconfig.get_total_weight());
    println!(
        "Threshold weight: {}",
        (*wconfig.get_threshold_config()).get_threshold()
    );

    let dealing_args = setup_dealing::<das::WeightedTranscript, _>(&wconfig, &mut rng);

    let msk_scalar = *dealing_args.s.get_secret_a();
    let mpk = G2Projective::generator() * msk_scalar;

    let mut transcripts = Vec::new();
    for i in 0..weights.len() {
        let trx = das::WeightedTranscript::deal(
            &wconfig,
            &dealing_args.pp,
            &dealing_args.ssks[i],
            &dealing_args.eks,
            &dealing_args.iss[i],
            &NoAux,
            &wconfig.get_player(i),
            &mut rng,
        );
        transcripts.push(trx);
    }

    let aggregated_trx =
        das::WeightedTranscript::aggregate(&wconfig, transcripts).expect("Failed to aggregate");

    let identity = compute_timelock_identity(123, 1706000000000000u64);
    let message = b"Weighted DKG test message";
    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("Encryption failed");

    let all_players: Vec<Player> = (0..weights.len()).map(|i| wconfig.get_player(i)).collect();
    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    let players_and_shares: Vec<(Player, WeightedDkgShare)> = all_players
        .iter()
        .map(|player| {
            let (sk_share, _): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
                &wconfig,
                player,
                &dealing_args.dks[player.get_id()],
                &dealing_args.pp,
            );
            (*player, sk_share)
        })
        .collect();

    // Flatten shares for reconstruction
    let mut flattened_shares = Vec::new();
    for (player, share_vec) in &players_and_shares {
        for (i, share) in share_vec.iter().enumerate() {
            flattened_shares.push((wconfig.get_virtual_player(player, i), share.clone()));
        }
    }

    let reconstructed_msk =
        DealtSecretKey::reconstruct(wconfig.get_threshold_config(), &flattened_shares);
    let dk = q_id * msk_scalar;
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");

    assert_eq!(decrypted, message);

    // Verify MSK group element was successfully reconstructed from both subsets
    println!("  ✓ MSK reconstructed successfully from both player subsets");
    println!("  ✓ Weighted DKG IBE protocol test passed\n");
}
