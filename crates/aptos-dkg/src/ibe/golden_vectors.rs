
use super::*;
use aptos_crypto::blstrs::random_scalar;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct GoldenVectors {
    description: String,
    timestamp: String,
    parameters: Parameters,
    keys: Keys,
    ciphertext: CiphertextHex,
    verification: Verification,
}

#[derive(Serialize, Deserialize)]
struct Parameters {
    chain_id: u8,
    interval: u64,
    message_string: String,
}

#[derive(Serialize, Deserialize)]
struct Keys {
    msk_hex: String,
    mpk_g2_hex: String,
    identity_hash_hex: String,
    decryption_key_g1_hex: String,
}

#[derive(Serialize, Deserialize)]
struct CiphertextHex {
    u_g2_hex: String,
    v_bytes_hex: String,
}

#[derive(Serialize, Deserialize)]
struct Verification {
    decrypted_hex: String,
}

#[test]
fn generate_and_save_golden_vectors() {
    let mut rng = thread_rng();

    // 1. Setup Keys
    let msk = random_scalar(&mut rng);
    let mpk = G2Projective::generator() * msk;

    // 2. Identity
    let chain_id = 4;
    let interval = 1000;
    let identity = compute_timelock_identity(interval, chain_id);

    // 3. Decryption Key
    let dk = derive_decryption_key(&msk, &identity).expect("Derivation failed");

    // 4. Encrypt
    let message_str = "Golden Vector Message 2024";
    let message = message_str.as_bytes();
    let ciphertext = ibe_encrypt(&mpk, &identity, message).expect("Encryption failed");

    // 5. Verify Decryption (Self-check)
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");
    assert_eq!(decrypted, message, "Self-check failed");

    // 6. Serialize to JSON Struct
    // Note: Rust scalars/points to hex
    let msk_bytes = msk.to_bytes_le(); 
    let mpk_bytes = serialize_g2(&mpk).unwrap();
    let dk_bytes = serialize_g1(&dk).unwrap();
    let u_bytes = serialize_g2(&ciphertext.u).unwrap();
        
    let fixtures = GoldenVectors {
        description: "IBE Golden Vectors for Atomica Timelock (Rust Generated)".to_string(),
        timestamp: format!("{:?}", std::time::SystemTime::now()),
        parameters: Parameters {
            chain_id,
            interval,
            message_string: message_str.to_string(),
        },
        keys: Keys {
            msk_hex: hex::encode(msk_bytes), // Scalar is 32 bytes
            mpk_g2_hex: hex::encode(mpk_bytes),
            identity_hash_hex: hex::encode(&identity),
            decryption_key_g1_hex: hex::encode(dk_bytes),
        },
        ciphertext: CiphertextHex {
            u_g2_hex: hex::encode(u_bytes),
            v_bytes_hex: hex::encode(&ciphertext.v),
        },
        verification: Verification {
            decrypted_hex: hex::encode(decrypted),
        },
    };

    // 7. Save to File
    // Path relative to where cargo test runs (usually workspace root or crate root)
    // We want `atomica/golden-vectors/ibe_fixtures_rust.json`
    // Assuming run from workspace root:
    let output_path = PathBuf::from("atomica/golden-vectors/ibe_fixtures.json");
    
    // Ensure directory exists (it should, but safety first)
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let json = serde_json::to_string_pretty(&fixtures).unwrap();
    let mut file = File::create(&output_path).expect("Failed to create golden vectors file");
    file.write_all(json.as_bytes()).expect("Failed to write golden vectors");

    println!("Saved golden vectors to {:?}", output_path.canonicalize().unwrap_or(output_path));
}

#[test]
fn verify_golden_vectors_roundtrip() {
    // 1. Load Fixtures
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let mut path = PathBuf::from(manifest_dir);
    // Walk up to workspace root from crate root (crates/aptos-dkg)
    path.pop(); // crates/
    path.pop(); // root
    path.push("atomica/golden-vectors/ibe_fixtures.json");

    if !path.exists() {
        panic!("Golden vectors file not found at {:?}. Run generate_and_save_golden_vectors first.", path);
    }
    
    let file = File::open(&path).expect("Failed to open golden vectors file");
    let fixture: GoldenVectors = serde_json::from_reader(file).expect("Failed to parse JSON");

    println!("Loaded fixture: {}", fixture.description);

    // 2. Verify Identity Param Computation
    let chain_id = fixture.parameters.chain_id;
    let interval = fixture.parameters.interval;
    let computed_identity = compute_timelock_identity(interval, chain_id);
    let expected_identity = hex::decode(&fixture.keys.identity_hash_hex).unwrap();
    
    assert_eq!(computed_identity, expected_identity, "Identity calculation mismatch");
    println!("✅ Identity calculation matches");

    // 3. Deserialize Keys
    let mpk_bytes = hex::decode(&fixture.keys.mpk_g2_hex).unwrap();
    let mpk = deserialize_g2(&mpk_bytes).expect("Failed to deserialize MPK");
    
    let dk_bytes = hex::decode(&fixture.keys.decryption_key_g1_hex).unwrap();
    let dk = deserialize_g1(&dk_bytes).expect("Failed to deserialize DK");
    println!("✅ Keys deserialized successfully");

    // 4. Decrypt Vector Ciphertext
    let u_bytes = hex::decode(&fixture.ciphertext.u_g2_hex).unwrap();
    let u = deserialize_g2(&u_bytes).expect("Failed to deserialize ciphertext U");
    let v = hex::decode(&fixture.ciphertext.v_bytes_hex).unwrap();
    
    let ciphertext = Ciphertext { u, v };
    
    let decrypted = ibe_decrypt(&dk, &ciphertext).expect("Decryption failed");
    let decrypted_hex = hex::encode(&decrypted);
    
    assert_eq!(decrypted_hex, fixture.verification.decrypted_hex, "Decryption verification failed");
    
    let message = std::str::from_utf8(&decrypted).unwrap();
    assert_eq!(message, fixture.parameters.message_string, "Decrypted message text mismatch");
    println!("✅ Decryption of golden vector successful: {:?}", message);

    // 5. Roundtrip Test (Encrypt -> Decrypt with restored keys)
    let new_ciphertext = ibe_encrypt(&mpk, &computed_identity, message.as_bytes()).expect("Encryption failed");
    let new_decrypted = ibe_decrypt(&dk, &new_ciphertext).expect("Roundtrip decryption failed");
    
    assert_eq!(new_decrypted, decrypted, "Roundtrip data mismatch");
    println!("✅ Roundtrip (Encrypt -> Decrypt) successful");
}
