// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Generate golden test vectors for timelock encryption.
//!
//! This test generates cryptographic test vectors for identity computation and key derivation.
//! These vectors are used across:
//! - Rust unit tests
//! - Move VM native function tests
//! - Move language unit tests
//!
//! Run with: `cargo test --package aptos-dkg generate_golden_vectors -- --ignored --nocapture`

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{fs::File, io::Write};

#[derive(Serialize, Deserialize, Debug)]
struct TimelockIdentityVector {
    /// Test case description
    description: String,
    /// Timelock ID
    timelock_id: u64,
    /// Deadline in microseconds
    deadline_us: u64,
    /// BCS-encoded timelock_id (little-endian, 8 bytes, hex)
    timelock_id_bcs_hex: String,
    /// BCS-encoded deadline_us (little-endian, 8 bytes, hex)
    deadline_us_bcs_hex: String,
    /// Expected identity hash (SHA3-256 of BCS(timelock_id) || BCS(deadline_us))
    identity_hash_hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TimelockGoldenVectors {
    version: String,
    generated_at: String,
    description: String,
    vectors: Vec<TimelockIdentityVector>,
}

/// Compute identity hash following the Move contract logic:
/// identity = SHA3-256(BCS(timelock_id) || BCS(deadline_us))
fn compute_identity_hash(timelock_id: u64, deadline_us: u64) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    // BCS encoding of u64 is little-endian 8 bytes
    hasher.update(&bcs::to_bytes(&timelock_id).unwrap());
    hasher.update(&bcs::to_bytes(&deadline_us).unwrap());
    hasher.finalize().to_vec()
}

#[test]
#[ignore] // Run with --ignored flag to generate vectors
fn generate_golden_vectors() {
    println!("\n🔧 Generating timelock identity golden test vectors...\n");

    let mut vectors = TimelockGoldenVectors {
        version: "1.0.0".to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        description: "Golden test vectors for Atomica timelock identity computation".to_string(),
        vectors: vec![],
    };

    // Test case 1: Basic timelock ID=0
    {
        println!("📝 Generating test vector 1: timelock_id=0, deadline=1000000000000");

        let timelock_id = 0u64;
        let deadline_us = 1_000_000_000_000u64;

        let identity = compute_identity_hash(timelock_id, deadline_us);

        let test_vector = TimelockIdentityVector {
            description: "Basic timelock with ID 0".to_string(),
            timelock_id,
            deadline_us,
            timelock_id_bcs_hex: hex::encode(bcs::to_bytes(&timelock_id).unwrap()),
            deadline_us_bcs_hex: hex::encode(bcs::to_bytes(&deadline_us).unwrap()),
            identity_hash_hex: hex::encode(&identity),
        };

        println!("  ✓ Timelock ID: {}", timelock_id);
        println!("  ✓ Deadline: {}", deadline_us);
        println!("  ✓ Identity: {}", &test_vector.identity_hash_hex);

        vectors.vectors.push(test_vector);
    }

    // Test case 2: Different ID, same deadline
    {
        println!("\n📝 Generating test vector 2: timelock_id=1, deadline=1000000000000");

        let timelock_id = 1u64;
        let deadline_us = 1_000_000_000_000u64; // Same as test 1

        let identity = compute_identity_hash(timelock_id, deadline_us);

        let test_vector = TimelockIdentityVector {
            description: "Different ID, same deadline as test 1 (should have different identity)"
                .to_string(),
            timelock_id,
            deadline_us,
            timelock_id_bcs_hex: hex::encode(bcs::to_bytes(&timelock_id).unwrap()),
            deadline_us_bcs_hex: hex::encode(bcs::to_bytes(&deadline_us).unwrap()),
            identity_hash_hex: hex::encode(&identity),
        };

        println!("  ✓ Identity: {}", &test_vector.identity_hash_hex);
        println!(
            "  ✓ Differs from test 1: {}",
            test_vector.identity_hash_hex != vectors.vectors[0].identity_hash_hex
        );

        vectors.vectors.push(test_vector);
    }

    // Test case 3: Same ID, different deadline
    {
        println!("\n📝 Generating test vector 3: timelock_id=0, deadline=2000000000000");

        let timelock_id = 0u64; // Same as test 1
        let deadline_us = 2_000_000_000_000u64; // Different

        let identity = compute_identity_hash(timelock_id, deadline_us);

        let test_vector = TimelockIdentityVector {
            description: "Same ID as test 1, different deadline (should have different identity)"
                .to_string(),
            timelock_id,
            deadline_us,
            timelock_id_bcs_hex: hex::encode(bcs::to_bytes(&timelock_id).unwrap()),
            deadline_us_bcs_hex: hex::encode(bcs::to_bytes(&deadline_us).unwrap()),
            identity_hash_hex: hex::encode(&identity),
        };

        println!("  ✓ Identity: {}", &test_vector.identity_hash_hex);
        println!(
            "  ✓ Differs from test 1: {}",
            test_vector.identity_hash_hex != vectors.vectors[0].identity_hash_hex
        );

        vectors.vectors.push(test_vector);
    }

    // Test case 4: Large values
    {
        println!("\n📝 Generating test vector 4: Large values");

        let timelock_id = 999999u64;
        let deadline_us = 9_999_999_999_999u64;

        let identity = compute_identity_hash(timelock_id, deadline_us);

        let test_vector = TimelockIdentityVector {
            description: "Large timelock ID and deadline values".to_string(),
            timelock_id,
            deadline_us,
            timelock_id_bcs_hex: hex::encode(bcs::to_bytes(&timelock_id).unwrap()),
            deadline_us_bcs_hex: hex::encode(bcs::to_bytes(&deadline_us).unwrap()),
            identity_hash_hex: hex::encode(&identity),
        };

        println!("  ✓ Identity: {}", &test_vector.identity_hash_hex);

        vectors.vectors.push(test_vector);
    }

    // Test case 5: Maximum values
    {
        println!("\n📝 Generating test vector 5: Maximum u64 values");

        let timelock_id = u64::MAX;
        let deadline_us = u64::MAX;

        let identity = compute_identity_hash(timelock_id, deadline_us);

        let test_vector = TimelockIdentityVector {
            description: "Maximum u64 values for stress testing".to_string(),
            timelock_id,
            deadline_us,
            timelock_id_bcs_hex: hex::encode(bcs::to_bytes(&timelock_id).unwrap()),
            deadline_us_bcs_hex: hex::encode(bcs::to_bytes(&deadline_us).unwrap()),
            identity_hash_hex: hex::encode(&identity),
        };

        println!("  ✓ Identity: {}", &test_vector.identity_hash_hex);

        vectors.vectors.push(test_vector);
    }

    // Write to JSON file
    let output_path = "atomica/golden_vectors/timelock_golden_vectors.json";
    std::fs::create_dir_all("atomica/golden_vectors").unwrap();
    let json = serde_json::to_string_pretty(&vectors).unwrap();
    let mut file = File::create(output_path).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    println!("\n✅ Generated {} test vectors", vectors.vectors.len());
    println!("✅ Saved to {}", output_path);

    // Also generate a human-readable text version
    let txt_path = "atomica/golden_vectors/timelock_golden_vectors.txt";
    let mut txt_file = File::create(txt_path).unwrap();
    writeln!(
        txt_file,
        "Timelock Identity Computation Golden Test Vectors"
    )
    .unwrap();
    writeln!(
        txt_file,
        "=================================================="
    )
    .unwrap();
    writeln!(txt_file, "Generated: {}", vectors.generated_at).unwrap();
    writeln!(txt_file, "Version: {}", vectors.version).unwrap();
    writeln!(txt_file, "{}\n", vectors.description).unwrap();

    for (i, v) in vectors.vectors.iter().enumerate() {
        writeln!(txt_file, "Test Vector #{}", i + 1).unwrap();
        writeln!(txt_file, "  Description: {}", v.description).unwrap();
        writeln!(txt_file, "  Timelock ID: {}", v.timelock_id).unwrap();
        writeln!(txt_file, "  Deadline (μs): {}", v.deadline_us).unwrap();
        writeln!(txt_file, "  BCS(timelock_id): {}", v.timelock_id_bcs_hex).unwrap();
        writeln!(txt_file, "  BCS(deadline_us): {}", v.deadline_us_bcs_hex).unwrap();
        writeln!(txt_file, "  Identity Hash: {}", v.identity_hash_hex).unwrap();
        writeln!(txt_file).unwrap();
    }

    println!("✅ Saved text version to {}\n", txt_path);
}
