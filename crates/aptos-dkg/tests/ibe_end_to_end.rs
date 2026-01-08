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
    ibe::{
        compute_timelock_identity, ibe_decrypt, ibe_encrypt, serialize_g1, serialize_g2, Ciphertext,
    },
    pvss::{
        das,
        test_utils::{setup_dealing, NoAux},
        traits::{Reconstructable, SecretSharingConfig, Transcript},
        Player, ThresholdConfigBlstrs, WeightedConfig,
    },
};
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::PrimeField;
use group::Group;
use rand::thread_rng;

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
    dealing_args: das::DealingArgs<das::WeightedTranscript>,
    wconfig: WeightedConfig,
    transcripts: Vec<das::WeightedTranscript>,
    reconstructed_msk_scalar: Scalar,
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
        wconfig.get_threshold_config().get_threshold()
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

    println!("PHASE 3: Decrypting secret key shares from transcripts...");

    // Get threshold number of players to reconstruct
    let eligible_players: Vec<Player> = wconfig
        .get_random_eligible_subset_of_players(&mut rng)
        .into_iter()
        .take(config.threshold)
        .collect();

    println!(
        "  - Selected {} players for reconstruction: {:?}",
        eligible_players.len(),
        eligible_players
            .iter()
            .map(|p| p.get_id())
            .collect::<Vec<_>>()
    );

    // Decrypt shares from each player
    // Use the Transcript trait's associated types
    type ShareType = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;

    let mut decrypted_shares: Vec<(Player, ShareType)> = Vec::new();
    for player in &eligible_players {
        let player_id = player.get_id();
        let (sk_share, pk_share) = aggregated_trx.decrypt_own_share(
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

        decrypted_shares.push((*player, sk_share));
        let share_bytes = sk_share.to_bytes();
        println!(
            "  - Player {}: decrypted share (G1 point, 48 bytes): {:02x?}",
            player_id,
            &share_bytes[0..16.min(16)]
        );
    }

    println!("\nPHASE 4: Reconstructing decryption key shares for IBE...");

    // Compute H(identity) = Q_id
    let identity = compute_timelock_identity(config.timelock_id, config.deadline_us);
    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    println!("  - Identity: {:02x?}", &identity[0..16.min(16)]);
    println!("  - Q_id (H(identity)): G1 point");

    // Reconstruct MSK scalar from shares
    // Each share is DealtSecretKeyShare which wraps DealtSecretKey(g1^msk_share)
    // We need to convert the group element to scalar
    let shares_for_reconstruction: Vec<(Player, Scalar)> = decrypted_shares
        .iter()
        .map(|(player, share)| {
            // DealtSecretKeyShare contains DealtSecretKey which has as_group_element()
            let share_ref: &ShareType = share;
            let share_element = share_ref.as_group_element();
            let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
                .expect("Failed to convert group element to scalar");
            (*player, scalar)
        })
        .collect();

    let reconstructed_msk_scalar =
        Scalar::reconstruct(wconfig.get_threshold_config(), &shares_for_reconstruction);

    println!(
        "  - Reconstructed MSK scalar: {:02x?}",
        &reconstructed_msk_scalar.to_bytes_le()[0..16]
    );

    // Verify reconstruction
    assert_eq!(
        reconstructed_msk_scalar, msk_scalar,
        "MSK reconstruction failed"
    );
    println!("  - MSK reconstruction verified!\n");

    println!("PHASE 5: Computing IBE decryption key...");

    // DK = MSK * Q_id
    let decryption_key = q_id * reconstructed_msk_scalar;
    let dk_bytes = serialize_g1(&decryption_key).expect("DK serialization failed");
    println!(
        "  - Decryption key (G1 point, 48 bytes): {:02x?}",
        &dk_bytes[0..16.min(16)]
    );

    // Verify DK matches what we get from the scalar directly
    let expected_dk = q_id * msk_scalar;
    assert_eq!(decryption_key, expected_dk, "DK mismatch");
    println!("  - Decryption key verified!\n");

    println!("PHASE 6: Encrypting message with IBE...");

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

    assert_eq!(
        reconstructed_msk_scalar, msk_scalar,
        "MSK reconstruction failed"
    );
    println!("  ✓ MSK reconstruction verified against original");

    assert_eq!(decryption_key, expected_dk, "DK mismatch");
    println!("  ✓ Decryption key matches expected value");

    println!("\n=== IBE END-TO-END TEST PASSED ===\n");

    Ok(E2EResult {
        dealing_args,
        wconfig,
        transcripts,
        reconstructed_msk_scalar,
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

    type ShareType = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;

    let mut shares: Vec<(Player, ShareType)> = Vec::new();
    for player in &eligible_players {
        let (sk_share, _) = aggregated_trx.decrypt_own_share(
            &wconfig,
            player,
            &dealing_args.dks[player.get_id()],
            &dealing_args.pp,
        );
        shares.push((*player, sk_share));
    }

    let identity = compute_timelock_identity(1, 1704070800000000u64);
    let message = b"Test message for 3-of-5 configuration";
    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("Encryption failed");

    let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

    let shares_for_recon: Vec<(Player, Scalar)> = shares
        .iter()
        .map(|(player, share)| {
            let share_element = share.as_group_element();
            let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
                .expect("Failed to convert group element to scalar");
            (*player, scalar)
        })
        .collect();

    let reconstructed_msk = Scalar::reconstruct(wconfig.get_threshold_config(), &shares_for_recon);

    let dk = q_id * reconstructed_msk;
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");

    assert_eq!(decrypted, message);
    assert_eq!(
        reconstructed_msk, msk_scalar,
        "Reconstructed MSK should match original"
    );
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

    type ShareType = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;

    println!("Testing reconstruction with players [0, 1, 2]...");
    let subset1: Vec<Player> = all_players[0..3].to_vec();
    let mut shares1: Vec<(Player, ShareType)> = Vec::new();
    for player in &subset1 {
        let (sk_share, _) = aggregated_trx.decrypt_own_share(
            &wconfig,
            player,
            &dealing_args.dks[player.get_id()],
            &dealing_args.pp,
        );
        shares1.push((*player, sk_share));
    }

    let shares1_for_recon: Vec<(Player, Scalar)> = shares1
        .iter()
        .map(|(player, share)| {
            let share_element = share.as_group_element();
            let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
                .expect("Failed to convert group element to scalar");
            (*player, scalar)
        })
        .collect();

    let reconstructed_msk1 =
        Scalar::reconstruct(wconfig.get_threshold_config(), &shares1_for_recon);
    let dk1 = q_id * reconstructed_msk1;
    let decrypted1 = ibe_decrypt(&dk1, &ciphertext).expect("Decryption failed");
    assert_eq!(decrypted1, message);
    println!("  ✓ Decryption successful with players [0, 1, 2]");

    println!("Testing reconstruction with players [0, 1, 3]...");
    let subset2 = vec![
        all_players[0].clone(),
        all_players[1].clone(),
        all_players[3].clone(),
    ];
    let mut shares2: Vec<(Player, ShareType)> = Vec::new();
    for player in &subset2 {
        let (sk_share, _) = aggregated_trx.decrypt_own_share(
            &wconfig,
            player,
            &dealing_args.dks[player.get_id()],
            &dealing_args.pp,
        );
        shares2.push((*player, sk_share));
    }

    let shares2_for_recon: Vec<(Player, Scalar)> = shares2
        .iter()
        .map(|(player, share)| {
            let share_element = share.as_group_element();
            let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
                .expect("Failed to convert group element to scalar");
            (*player, scalar)
        })
        .collect();

    let reconstructed_msk2 =
        Scalar::reconstruct(wconfig.get_threshold_config(), &shares2_for_recon);
    let dk2 = q_id * reconstructed_msk2;
    let decrypted2 = ibe_decrypt(&dk2, &ciphertext).expect("Decryption failed");
    assert_eq!(decrypted2, message);
    println!("  ✓ Decryption successful with players [0, 1, 3]");

    assert_eq!(
        reconstructed_msk1, reconstructed_msk2,
        "Different subsets should yield same MSK"
    );
    assert_eq!(
        reconstructed_msk1, msk_scalar,
        "Reconstructed MSK should match original"
    );
    println!("  ✓ Both subsets yield identical MSK matching original");

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
        wconfig.get_threshold_config().get_threshold()
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

    type ShareType = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;

    let mut shares: Vec<(Player, ShareType)> = Vec::new();
    for player in &all_players {
        let (sk_share, _) = aggregated_trx.decrypt_own_share(
            &wconfig,
            player,
            &dealing_args.dks[player.get_id()],
            &dealing_args.pp,
        );
        shares.push((*player, sk_share));
    }

    let shares_for_recon: Vec<(Player, Scalar)> = shares
        .iter()
        .map(|(player, share)| {
            let share_element = share.as_group_element();
            let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
                .expect("Failed to convert group element to scalar");
            (*player, scalar)
        })
        .collect();

    let reconstructed_msk = Scalar::reconstruct(wconfig.get_threshold_config(), &shares_for_recon);
    let dk = q_id * reconstructed_msk;
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");

    assert_eq!(decrypted, message);
    assert_eq!(
        reconstructed_msk, msk_scalar,
        "Reconstructed MSK should match original"
    );
    println!("  ✓ Weighted DKG IBE protocol test passed\n");
}
