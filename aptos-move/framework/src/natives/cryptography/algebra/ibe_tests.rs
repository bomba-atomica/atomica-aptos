// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for IBE DK reconstruction.
//!
//! These tests verify the DK reconstruction by calling apt-dkg's canonical
//! implementation directly with golden vectors.

use blstrs::G1Affine;

fn hex_to_g1(hex_str: &str) -> G1Affine {
    let bytes = hex::decode(hex_str).unwrap();
    let mut arr = [0u8; 48];
    arr.copy_from_slice(&bytes[..48]);
    G1Affine::from_compressed(&arr).unwrap()
}

#[test]
fn test_dk_reconstruction_matches_golden_vector_1() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[0];

    let validator_indices: Vec<u64> = test_case.validator_indices[0..3].to_vec();
    let weights: Vec<u64> = test_case.validator_weights[0..3].to_vec();

    let dk_shares: Vec<G1Affine> = (0..3)
        .map(|i| hex_to_g1(&test_case.dk_shares_g1_hex[i]))
        .collect();

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    let expected_dk = hex_to_g1(&test_case.reconstructed_dk_g1_hex);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "Reconstructed DK should match golden vector for test case 1"
    );
}

#[test]
fn test_dk_reconstruction_matches_golden_vector_2() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[1];

    let validator_indices: Vec<u64> = vec![0, 2];
    let weights: Vec<u64> = vec![1, 1];

    let dk_shares: Vec<G1Affine> = vec![
        hex_to_g1(&test_case.dk_shares_g1_hex[0]),
        hex_to_g1(&test_case.dk_shares_g1_hex[2]),
    ];

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    let expected_dk = hex_to_g1(&test_case.reconstructed_dk_g1_hex);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "Reconstructed DK should match golden vector for test case 2"
    );
}

#[test]
fn test_dk_reconstruction_unequal_weights() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[2];

    let validator_indices: Vec<u64> = test_case.validator_indices.clone();
    let weights: Vec<u64> = test_case.validator_weights.clone();

    let dk_shares: Vec<G1Affine> = test_case
        .dk_shares_g1_hex
        .iter()
        .map(|hex| hex_to_g1(hex))
        .collect();

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    let expected_dk = hex_to_g1(&test_case.reconstructed_dk_g1_hex);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "Reconstructed DK should match golden vector for test case 3 (unequal weights)"
    );
}

#[test]
fn test_reconstruction_with_validator_index_zero() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[0];

    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let weights: Vec<u64> = test_case.validator_weights[0..3].to_vec();

    let dk_shares: Vec<G1Affine> = (0..3)
        .map(|i| hex_to_g1(&test_case.dk_shares_g1_hex[i]))
        .collect();

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    assert_ne!(
        reconstructed_dk, dk_shares[0],
        "Reconstructed DK should not be same as single share"
    );
}

#[test]
fn test_single_share_reconstruction() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[0];

    let validator_indices: Vec<u64> = vec![0];
    let weights: Vec<u64> = vec![1];
    let total_weight = 1u64;

    let dk_shares: Vec<G1Affine> = vec![hex_to_g1(&test_case.dk_shares_g1_hex[0])];

    let reconstructed_dk =
        aptos_dkg::ibe::reconstruct_ibe_dk(&validator_indices, &dk_shares, &weights, total_weight);

    assert_eq!(
        reconstructed_dk, dk_shares[0],
        "Single share reconstruction should return the share itself"
    );
}

#[test]
fn test_contiguous_validator_indices() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[0];

    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let weights: Vec<u64> = test_case.validator_weights[0..3].to_vec();

    let dk_shares: Vec<G1Affine> = (0..3)
        .map(|i| hex_to_g1(&test_case.dk_shares_g1_hex[i]))
        .collect();

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    let expected_dk = hex_to_g1(&test_case.reconstructed_dk_g1_hex);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "Reconstruction with contiguous validator indices should match golden vector"
    );
}

#[test]
fn test_sparse_validator_indices() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[1];

    let validator_indices: Vec<u64> = vec![0, 2];
    let weights: Vec<u64> = vec![1, 1];

    let dk_shares: Vec<G1Affine> = vec![
        hex_to_g1(&test_case.dk_shares_g1_hex[0]),
        hex_to_g1(&test_case.dk_shares_g1_hex[2]),
    ];

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &dk_shares,
        &weights,
        test_case.total_weight,
    );

    let expected_dk = hex_to_g1(&test_case.reconstructed_dk_g1_hex);

    assert_eq!(
        reconstructed_dk, expected_dk,
        "Reconstruction with sparse validator indices should match golden vector"
    );
}
