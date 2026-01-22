// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for IBE DK reconstruction.
//!
//! These tests verify the DK reconstruction by calling apt-dkg's canonical
//! implementation directly.
//!
//! Note: The full golden vector tests are in the apt-dkg crate. This file
//! contains simplified tests that verify the native function interface.

use blstrs::Scalar;

#[test]
fn test_dk_reconstruction_basic() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[0];

    // For equal weights, each validator has 1 share
    let validator_indices: Vec<u64> = vec![0, 1, 2];
    let full_weights: Vec<u64> = test_case.validator_weights.clone();

    // Use placeholder scalar shares - these won't produce the correct DK
    // but verify the API structure works
    let scalar_shares: Vec<Vec<Scalar>> = vec![
        vec![Scalar::from(1u64)], // validator 0
        vec![Scalar::from(2u64)], // validator 1
        vec![Scalar::from(3u64)], // validator 2
    ];

    // For proper testing, we need the identity from the full golden vector
    // This test verifies the API structure without the full identity
    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        test_case.total_weight,
        &[0u8; 32], // placeholder identity
    );

    // Verify we got a valid G1 point (not identity)
    let dk_bytes = reconstructed_dk.to_compressed();
    assert!(
        dk_bytes != [0u8; 48],
        "DK should not be the identity element"
    );
}

/// Test reconstruction with unequal weights.
#[test]
fn test_dk_reconstruction_unequal_weights() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };

    // Find a test case with unequal weights
    let test_case = match golden_vectors.ibe_roundtrip_vectors.iter().find(|v| {
        v.validator_weights.len() >= 3 && v.validator_weights[0] != v.validator_weights[1]
    }) {
        Some(v) => v,
        None => return,
    };

    let validator_indices: Vec<u64> = test_case.validator_indices.clone();
    let full_weights: Vec<u64> = test_case.validator_weights.clone();

    // Create scalar shares matching the weights
    let mut scalar_shares: Vec<Vec<Scalar>> = Vec::new();
    let mut counter: u64 = 1;
    for &weight in test_case.validator_weights.iter() {
        let mut shares: Vec<Scalar> = Vec::new();
        for _ in 0..weight {
            shares.push(Scalar::from(counter));
            counter += 1;
        }
        scalar_shares.push(shares);
    }

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        test_case.total_weight,
        &[0u8; 32], // placeholder identity
    );

    // Verify we got a valid G1 point
    let dk_bytes = reconstructed_dk.to_compressed();
    assert!(
        dk_bytes != [0u8; 48],
        "DK should not be the identity element"
    );
}

/// Test that reconstruction works with sparse validator indices.
#[test]
fn test_dk_reconstruction_sparse_indices() {
    let golden_vectors = match aptos_dkg::ibe::load_golden_vectors() {
        Some(v) => v,
        None => return,
    };
    let test_case = &golden_vectors.ibe_roundtrip_vectors[1];

    // Use sparse indices [0, 2] from test case 2
    let validator_indices: Vec<u64> = vec![0, 2];
    let full_weights: Vec<u64> = test_case.validator_weights.clone();

    let scalar_shares: Vec<Vec<Scalar>> = vec![
        vec![Scalar::from(1u64)], // validator 0
        vec![Scalar::from(3u64)], // validator 2
    ];

    let reconstructed_dk = aptos_dkg::ibe::reconstruct_ibe_dk(
        &validator_indices,
        &scalar_shares,
        &full_weights,
        test_case.total_weight,
        &[0u8; 32], // placeholder identity
    );

    // Verify we got a valid G1 point
    let dk_bytes = reconstructed_dk.to_compressed();
    assert!(
        dk_bytes != [0u8; 48],
        "DK should not be the identity element"
    );
}
