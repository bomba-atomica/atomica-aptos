use aptos_crypto::blstrs::multi_pairing;
use aptos_dkg::weighted_vuf::bls::BLS_WVUF_DST;
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use group::Group;
use sha3::{Digest, Keccak256};
use std::iter;

#[test]
fn test_tlock_ibe_poc() {
    // 1. Setup (Simulating a Validator Set DKG result)
    // Deterministic key for POC
    let msk = Scalar::from(12345u64);

    // Master Public Key (MPK) - s * G2 (since BlsWUF uses G2 for PK)
    let g2_gen = G2Projective::generator();
    let mpk = g2_gen * msk;

    // 2. Encryption (Client Side)
    // Identity = Block Height 1000
    let identity = b"block_1000";
    let message = b"secret_bid_value";

    println!(
        "Encrypting message: {:?}",
        std::str::from_utf8(message).unwrap()
    );

    let ciphertext = ibe_encrypt(mpk, identity, message);

    // 3. Extraction (Validator Side - Threshold DKG)
    // Validators compute signature on identity: sigma = H(id)^s
    // In our POC, we just use the MSK directly to simulate the aggregated result
    // Hash to curve manually since function is private in BlsWUF but DST is public
    let h_id = G1Projective::hash_to_curve(identity, BLS_WVUF_DST, b"H(m)");
    let decryption_key = h_id * msk; // This is the aggregated signature

    // 4. Decryption (Client/Public Side)
    let decrypted = ibe_decrypt(decryption_key, ciphertext);

    println!(
        "Decrypted message: {:?}",
        std::str::from_utf8(&decrypted).unwrap()
    );
    assert_eq!(message, decrypted.as_slice());
}

// Basic Boneh-Franklin IBE Encryption:
// C = <rP, M XOR H(e(Q_ID, P_pub)^r)>
// P = Generator (G2), P_pub = MPK (G2)
// Q_ID = H(ID) (G1)
// We swap G1/G2 roles compared to standard papers if BlsWUF uses G2 for PK.
// BlsWUF: Eval in G1, PK in G2.
// e: G1 x G2 -> Gt
struct Ciphertext {
    u: G2Projective, // r * G2_gen
    v: Vec<u8>,      // M xor H(...)
}

fn ibe_encrypt(mpk: G2Projective, identity: &[u8], message: &[u8]) -> Ciphertext {
    let r = Scalar::from(987654321u64); // Deterministic randomness

    // U = r * G2_gen
    let u = G2Projective::generator() * r;

    // Q_ID = H(ID) in G1
    let q_id = G1Projective::hash_to_curve(identity, BLS_WVUF_DST, b"H(m)");

    // g_id = e(Q_ID, MPK)^r
    // multi_pairing takes iterables of &Projective
    // We compute e(Q_ID, MPK) first, then raise to r. Or e(Q_ID, MPK * r).
    // Let's do e(Q_ID, MPK).pow(r).
    let pair = multi_pairing(iter::once(&q_id), iter::once(&mpk));
    let gid = pair * r;

    // K = H(gid)
    let key_hash = hash_gt_to_bytes(gid);

    // V = M xor K
    let v = xor_bytes(message, &key_hash);

    Ciphertext { u, v }
}

fn ibe_decrypt(decryption_key: G1Projective, ciphertext: Ciphertext) -> Vec<u8> {
    // key = e(d_ID, U)
    //     = e(s * Q_ID, r * P)
    //     = e(Q_ID, P)^(sr)
    //     = gid
    let gid = multi_pairing(iter::once(&decryption_key), iter::once(&ciphertext.u));

    let key_hash = hash_gt_to_bytes(gid);
    xor_bytes(&ciphertext.v, &key_hash)
}

fn hash_gt_to_bytes(gt: Gt) -> Vec<u8> {
    // Simple serialization and hash for POC
    // In production, use a proper KDF
    let mut hasher = Keccak256::new();
    hasher.update(format!("{:?}", gt)); // Debug format is hacky but deterministic enough for POC
    hasher.finalize().to_vec()
}

fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter().cycle())
        .map(|(&x, &y)| x ^ y)
        .collect()
}
