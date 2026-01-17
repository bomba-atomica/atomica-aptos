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
