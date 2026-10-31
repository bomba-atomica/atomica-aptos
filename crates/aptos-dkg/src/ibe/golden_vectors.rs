// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Golden vectors for the hardened Boneh-Franklin timelock (V2).
//!
//! The vectors are produced from a seeded, real chunky DKG dealing: three dealers deal with the
//! chunky weighted transcript (v2), the subtranscripts are aggregated, and the master public key,
//! the verification keys and the per player secret shares come from the real `TranscriptCore`
//! APIs (`get_dealt_public_key`, `get_public_key_share`, `decrypt_own_share`). Nothing here uses
//! `aptos-batch-encryption`, FPTX digests, eval proofs or the `DigestKey` format.
//!
//! Outputs, checked in and byte-for-byte reproducible:
//!
//! - `atomica/golden_vectors/ibe_v2_golden_vectors.json`
//! - `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move`
//!
//! Regenerate with
//!
//! ```text
//! cargo test -p aptos-dkg --lib ibe::golden_vectors::write_golden_vectors -- --ignored --nocapture
//! ```
//!
//! Tests in this module check the checked-in JSON with the production verification code
//! (`checked_in_vectors_verify`), check the cross library claim against blstrs
//! (`checked_in_cross_library_vectors_match_both_libraries`), and check that regeneration gives the
//! checked-in bytes (`regeneration_reproduces_checked_in_bytes`).

use super::{
    ciphertext::{decrypt_bid, encrypt_bid, verify_bid_ct, BidCiphertext},
    *,
};
use crate::pvss::{
    chunky::{
        PublicParameters, UnsignedWeightedTranscriptv2, WeightedSubtranscript,
        DEFAULT_ELL_FOR_TESTING,
    },
    test_utils::generate_keys_and_secrets,
    traits::{
        transcript::{HasAggregatableSubtranscript, Transcript, TranscriptCore},
        Aggregatable,
    },
    Player,
};
use aptos_crypto::weighted_config::WeightedConfigArkworks;
use ark_bls12_381::{Fq, Fq2};
use ark_ff::Field;
use ed25519_dalek::{Signer, SigningKey};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::{json, Value};

const JSON_PATH: &str = "atomica/golden_vectors/ibe_v2_golden_vectors.json";
const MOVE_PATH: &str =
    "aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move";
const CHECKED_IN_JSON: &str =
    include_str!("../../../../atomica/golden_vectors/ibe_v2_golden_vectors.json");
const CHECKED_IN_MOVE: &str = include_str!(
    "../../../../aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move"
);

pub const GENERATOR_COMMAND: &str = "cargo test -p aptos-dkg --lib ibe::golden_vectors::write_golden_vectors -- --ignored --nocapture";

/// Seed of the chunky dealing. Everything else derives from it or from the constants below.
const DEALING_SEED: u64 = 0xA70_01CA_2026;
const CHAIN_ID: u8 = 7;
const EPOCH: u64 = 3;
const TIMELOCK_ID: u64 = 11;
const DEADLINE_US: u64 = 1_700_000_000_000_000;
const WEIGHTS: [usize; 3] = [2, 1, 2];
const THRESHOLD: usize = 3;
const BID_PLAINTEXT: &[u8] = b"atomica golden bid v2";

type Tx = UnsignedWeightedTranscriptv2<Bls12_381>;

fn hx(b: &[u8]) -> String {
    hex::encode(b)
}

fn unhex(v: &Value) -> Vec<u8> {
    hex::decode(v.as_str().expect("hex string")).expect("valid hex")
}

fn flip(mut v: Vec<u8>, i: usize) -> Vec<u8> {
    v[i] ^= 1;
    v
}

fn bid_ad(bidder: u8, auction: u64) -> Vec<u8> {
    bcs::to_bytes(&(
        "atomica/timelock-bid/v2",
        CHAIN_ID,
        EPOCH,
        TIMELOCK_ID,
        auction,
        [bidder; 32],
        DEADLINE_US,
    ))
    .unwrap()
}

fn raw_g1(p: &G1Affine) -> Vec<u8> {
    let mut v = vec![];
    p.serialize_with_mode(&mut v, Compress::Yes).unwrap();
    v
}

fn raw_g2(p: &G2Affine) -> Vec<u8> {
    let mut v = vec![];
    p.serialize_with_mode(&mut v, Compress::Yes).unwrap();
    v
}

/// On the curve, outside the prime-order subgroup.
fn non_subgroup_g1() -> G1Affine {
    let mut x = Fq::from(1u64);
    loop {
        if let Some(p) = G1Affine::get_point_from_x_unchecked(x, false)
            .filter(|p| p.is_on_curve() && !p.is_in_correct_subgroup_assuming_on_curve())
        {
            return p;
        }
        x += Fq::ONE;
    }
}

fn non_subgroup_g2() -> G2Affine {
    let mut c0 = Fq::from(1u64);
    loop {
        let x = Fq2::new(c0, Fq::from(1u64));
        if let Some(p) = G2Affine::get_point_from_x_unchecked(x, false)
            .filter(|p| p.is_on_curve() && !p.is_in_correct_subgroup_assuming_on_curve())
        {
            return p;
        }
        c0 += Fq::ONE;
    }
}

/// A compressed G1 encoding whose x has no y on the curve.
fn off_curve_g1() -> Vec<u8> {
    let mut x = Fq::from(1u64);
    loop {
        if G1Affine::get_point_from_x_unchecked(x, false).is_none() {
            let mut v = vec![];
            x.serialize_compressed(&mut v).unwrap();
            v.reverse();
            v[0] |= 0x80;
            return v;
        }
        x += Fq::ONE;
    }
}

fn identity_g1_encoding() -> Vec<u8> {
    let mut v = vec![0u8; G1_COMPRESSED_LEN];
    v[0] = 0xc0;
    v
}

fn failure_code(r: Result<(), ShareCheckFailure>) -> String {
    match r {
        Ok(()) => "ok".to_string(),
        Err(ShareCheckFailure::CountMismatch { .. }) => "count_mismatch".to_string(),
        Err(ShareCheckFailure::Identity) => "identity".to_string(),
        Err(ShareCheckFailure::InvalidShare { index, .. }) => format!("invalid_share:{index}"),
        Err(ShareCheckFailure::InvalidVerificationKey { index, .. }) => {
            format!("invalid_vk:{index}")
        },
        Err(ShareCheckFailure::PairingMismatch { index }) => format!("pairing_mismatch:{index}"),
    }
}

// ---------------------------------------------------------------------------------------------
// The chunky dealing
// ---------------------------------------------------------------------------------------------

/// Output of the real chunky dealing for the timelock session.
struct ChunkyDealing {
    mpk: G2Affine,
    /// `vks[validator][j] = g2^{s_p}` from `get_public_key_share`.
    vks: Vec<Vec<G2Affine>>,
    /// `shares[validator][j] = s_p` from `decrypt_own_share`.
    shares: Vec<Vec<Fr>>,
    subtranscript_bcs: Vec<u8>,
}

fn session_id() -> (&'static str, u8, u64) {
    // Stands in for `TimelockDKGSessionId` (aptos-types). The dealing binds whatever serializable
    // value it is given, so the vectors only need a fixed one.
    ("atomica/timelock-chunky-dkg/v1", CHAIN_ID, EPOCH)
}

fn chunky_dealing() -> ChunkyDealing {
    let mut rng = StdRng::seed_from_u64(DEALING_SEED);
    let total: usize = WEIGHTS.iter().sum();
    let sc = WeightedConfigArkworks::<Fr>::new(THRESHOLD, WEIGHTS.to_vec()).unwrap();

    // The standard BLS12-381 G2 generator is the commitment base, so `V0 = g2^s` (spec section 4).
    let pp = PublicParameters::<Bls12_381>::new_for_testing(
        total,
        DEFAULT_ELL_FOR_TESTING,
        166,
        G2Affine::generator(),
        &mut rng,
    );
    assert_eq!(pp.get_commitment_base(), G2Affine::generator());

    let (ssks, spks, dks, eks, iss, _s, _dsk, _dpk) =
        generate_keys_and_secrets::<Tx, _>(&sc, &pp, &mut rng);

    // Every validator is a dealer. Each transcript is verified under the session id.
    let sid = session_id();
    let mut subtranscripts = vec![];
    for (i, input) in iss.iter().enumerate() {
        let dealer = Player { id: i };
        let t = Tx::deal(
            &sc, &pp, &ssks[i], &spks[i], &eks, input, &sid, &dealer, &mut rng,
        );
        HasAggregatableSubtranscript::verify(&t, &sc, &pp, &spks, &eks, &sid, &mut rng)
            .expect("a fresh dealing verifies");
        subtranscripts.push(t.get_subtranscript());
    }
    let aggregated = WeightedSubtranscript::<Bls12_381>::aggregate(&sc, subtranscripts).unwrap();

    let mpk = aggregated.get_dealt_public_key().as_g2();
    let mut vks = vec![];
    let mut shares = vec![];
    for (i, dk) in dks.iter().enumerate() {
        let player = Player { id: i };
        let (sk_shares, pk_shares) = aggregated.decrypt_own_share(&sc, &player, dk, &pp);
        assert_eq!(pk_shares, aggregated.get_public_key_share(&sc, &player));
        vks.push(pk_shares.iter().map(|p| p.as_g2()).collect::<Vec<_>>());
        shares.push(sk_shares.iter().map(|s| s.0).collect::<Vec<_>>());
    }
    ChunkyDealing {
        mpk,
        vks,
        shares,
        subtranscript_bcs: bcs::to_bytes(&aggregated).unwrap(),
    }
}

fn nested_dk_shares(d: &ChunkyDealing, id: &[u8]) -> Vec<Vec<Vec<u8>>> {
    d.shares
        .iter()
        .map(|per_validator| {
            per_validator
                .iter()
                .map(|s| encode_g1(&derive_decryption_key(s, id).unwrap()).to_vec())
                .collect()
        })
        .collect()
}

fn pick<T: Clone>(nested: &[Vec<T>], who: &[usize]) -> Vec<Vec<T>> {
    who.iter().map(|&v| nested[v].clone()).collect()
}

fn reconstruct(nested: &[Vec<Vec<u8>>], who: &[usize]) -> Result<G1Affine, IbeError> {
    let idx: Vec<u64> = who.iter().map(|v| *v as u64).collect();
    let weights: Vec<u64> = WEIGHTS.iter().map(|w| *w as u64).collect();
    reconstruct_ibe_dk_from_g1_shares(
        &idx,
        &pick(nested, who),
        &weights,
        THRESHOLD as u64,
        WEIGHTS.iter().sum::<usize>() as u64,
    )
}

// ---------------------------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------------------------

fn identity_vectors() -> Value {
    let cases: [(&str, u8, u64, u64, u64); 7] = [
        ("basic", 1, 0, 0, 1_000_000_000_000),
        ("other chain", 2, 0, 0, 1_000_000_000_000),
        ("other epoch", 1, 1, 0, 1_000_000_000_000),
        ("other timelock id", 1, 0, 1, 1_000_000_000_000),
        ("other deadline", 1, 0, 0, 2_000_000_000_000),
        (
            "session of the dealing",
            CHAIN_ID,
            EPOCH,
            TIMELOCK_ID,
            DEADLINE_US,
        ),
        ("maximum values", 255, u64::MAX, u64::MAX, u64::MAX),
    ];
    Value::Array(
        cases
            .iter()
            .map(|(name, chain, epoch, id, deadline)| {
                let identity = compute_identity(*chain, *epoch, *id, *deadline);
                json!({
                    "name": name,
                    "chain_id": chain,
                    "epoch": epoch,
                    "timelock_id": id,
                    "deadline_us": deadline,
                    "identity": hx(&identity),
                    "h1": hx(&encode_g1(&hash_to_g1(&identity).unwrap())),
                })
            })
            .collect(),
    )
}

fn cross_library_vectors() -> Value {
    let mut rng = StdRng::seed_from_u64(1000);
    Value::Array(
        (0..8u64)
            .map(|seed| {
                let id = compute_identity(seed as u8, seed, seed * 3 + 1, 1_000_000 * (seed + 1));
                let s = random_scalar(&mut rng);
                let r = random_scalar(&mut rng);
                let mut s_le = [0u8; 32];
                s.serialize_compressed(&mut s_le[..]).unwrap();
                let mut r_le = [0u8; 32];
                r.serialize_compressed(&mut r_le[..]).unwrap();
                let mpk = master_public_key(&s);
                let (u, _k) = encapsulate_with_scalar(&mpk, &id, &r).unwrap();
                json!({
                    "chain_id": seed as u8,
                    "epoch": seed,
                    "timelock_id": seed * 3 + 1,
                    "deadline_us": 1_000_000 * (seed + 1),
                    "identity": hx(&id),
                    "secret_le": hx(&s_le),
                    "r_le": hx(&r_le),
                    "h1": hx(&encode_g1(&hash_to_g1(&id).unwrap())),
                    "mpk": hx(&encode_g2(&mpk)),
                    "dk": hx(&encode_g1(&derive_decryption_key(&s, &id).unwrap())),
                    "u": hx(&encode_g2(&u)),
                })
            })
            .collect(),
    )
}

fn share_cases(d: &ChunkyDealing, id: &[u8], shares: &[Vec<Vec<u8>>]) -> Value {
    let vk_bytes =
        |v: usize| -> Vec<Vec<u8>> { d.vks[v].iter().map(|p| encode_g2(p).to_vec()).collect() };
    let other_id = compute_identity(CHAIN_ID, EPOCH, TIMELOCK_ID + 1, DEADLINE_US);
    let other_shares = nested_dk_shares(d, &other_id);

    // (name, validator the vks come from, shares, vks)
    let mut cases: Vec<(&str, usize, Vec<Vec<u8>>, Vec<Vec<u8>>)> = vec![
        ("valid", 0, shares[0].clone(), vk_bytes(0)),
        (
            "valid, weight one validator",
            1,
            shares[1].clone(),
            vk_bytes(1),
        ),
        (
            "shares swapped between virtual players",
            0,
            vec![shares[0][1].clone(), shares[0][0].clone()],
            vk_bytes(0),
        ),
        (
            "share of another identity",
            0,
            other_shares[0].clone(),
            vk_bytes(0),
        ),
        (
            "second share belongs to another identity",
            0,
            vec![shares[0][0].clone(), other_shares[0][1].clone()],
            vk_bytes(0),
        ),
        (
            "shares of validator 2 under the keys of validator 0",
            0,
            shares[2].clone(),
            vk_bytes(0),
        ),
        (
            "share is not in the subgroup",
            0,
            vec![shares[0][0].clone(), raw_g1(&non_subgroup_g1())],
            vk_bytes(0),
        ),
        (
            "share is not on the curve",
            0,
            vec![off_curve_g1(), shares[0][1].clone()],
            vk_bytes(0),
        ),
        (
            "share is the identity element",
            0,
            vec![identity_g1_encoding(), shares[0][1].clone()],
            vk_bytes(0),
        ),
        (
            "share is truncated",
            0,
            vec![shares[0][0].clone(), shares[0][1][..47].to_vec()],
            vk_bytes(0),
        ),
        (
            "fewer shares than keys",
            0,
            vec![shares[0][0].clone()],
            vk_bytes(0),
        ),
        ("no shares", 0, vec![], vec![]),
    ];
    let mut bad_vk = vk_bytes(0);
    bad_vk[1] = raw_g2(&non_subgroup_g2());
    cases.push((
        "verification key is not in the subgroup",
        0,
        shares[0].clone(),
        bad_vk,
    ));

    Value::Array(
        cases
            .into_iter()
            .map(|(name, validator, sh, vk)| {
                let expect = failure_code(verify_player_shares(id, &sh, &vk));
                json!({
                    "name": name,
                    "validator": validator,
                    "shares": sh.iter().map(|s| hx(s)).collect::<Vec<_>>(),
                    "verification_keys": vk.iter().map(|s| hx(s)).collect::<Vec<_>>(),
                    "expect": expect,
                })
            })
            .collect(),
    )
}

fn reconstruction_cases(d: &ChunkyDealing, id: &[u8], shares: &[Vec<Vec<u8>>]) -> Value {
    let sets: [(&str, &[usize]); 5] = [
        ("validators 0 and 2 (weight 4)", &[0, 2]),
        ("validators 0 and 1 (weight 3)", &[0, 1]),
        ("validators 1 and 2 (weight 3)", &[1, 2]),
        ("all validators", &[0, 1, 2]),
        ("validators 2 and 0 (order does not matter)", &[2, 0]),
    ];
    let mut out: Vec<Value> = vec![];
    for (name, who) in sets {
        let dk = reconstruct(shares, who).expect("a threshold set reconstructs");
        assert!(verify_decryption_key(&dk, id, &d.mpk));
        out.push(json!({
            "name": name,
            "validators": who,
            "expect": "ok",
            "dk": hx(&encode_g1(&dk)),
            "key_check": true,
        }));
    }
    // Below the threshold weight: an error, never a panic.
    let err = reconstruct(shares, &[1]).unwrap_err();
    assert!(matches!(err, IbeError::BelowThreshold { .. }));
    out.push(json!({
        "name": "validator 1 alone (weight 1 of 3)",
        "validators": [1],
        "expect": "below_threshold",
    }));
    // One wrong share (taken from another identity) rebuilds a key that fails the key check.
    let other_id = compute_identity(CHAIN_ID, EPOCH, TIMELOCK_ID + 1, DEADLINE_US);
    let other_shares = nested_dk_shares(d, &other_id);
    let mut poisoned = shares.to_vec();
    poisoned[0][0] = other_shares[0][0].clone();
    let bad = reconstruct(&poisoned, &[0, 2]).expect("shares are well formed");
    assert!(!verify_decryption_key(&bad, id, &d.mpk));
    let poisoned_shares = pick(&poisoned, &[0, 2]);
    out.push(json!({
        "name": "one share of another identity: key check fails",
        "validators": [0, 2],
        "expect": "ok",
        "shares": poisoned_shares
            .iter()
            .map(|v| v.iter().map(|s| hx(s)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        "dk": hx(&encode_g1(&bad)),
        "key_check": false,
    }));
    Value::Array(out)
}

fn key_check_cases(d: &ChunkyDealing, id: &[u8], dk: &G1Affine, other_dk: &G1Affine) -> Value {
    let other_id = compute_identity(CHAIN_ID, EPOCH, TIMELOCK_ID + 1, DEADLINE_US);
    let wrong_mpk = d.vks[0][0];
    let cases: Vec<(&str, Vec<u8>, Vec<u8>, Vec<u8>)> = vec![
        (
            "right key",
            encode_g1(dk).to_vec(),
            id.to_vec(),
            encode_g2(&d.mpk).to_vec(),
        ),
        (
            "key of another identity",
            encode_g1(other_dk).to_vec(),
            id.to_vec(),
            encode_g2(&d.mpk).to_vec(),
        ),
        (
            "right key, other identity",
            encode_g1(dk).to_vec(),
            other_id.to_vec(),
            encode_g2(&d.mpk).to_vec(),
        ),
        (
            "right key, wrong master public key",
            encode_g1(dk).to_vec(),
            id.to_vec(),
            encode_g2(&wrong_mpk).to_vec(),
        ),
        (
            "key is not in the subgroup",
            raw_g1(&non_subgroup_g1()),
            id.to_vec(),
            encode_g2(&d.mpk).to_vec(),
        ),
        (
            "key is the identity element",
            identity_g1_encoding(),
            id.to_vec(),
            encode_g2(&d.mpk).to_vec(),
        ),
    ];
    Value::Array(
        cases
            .into_iter()
            .map(|(name, dk, identity, mpk)| {
                let expect = match (decode_g1(&dk), decode_g2(&mpk)) {
                    (Ok(k), Ok(m)) => verify_decryption_key(&k, &identity, &m),
                    _ => false,
                };
                json!({
                    "name": name,
                    "dk": hx(&dk),
                    "identity": hx(&identity),
                    "mpk": hx(&mpk),
                    "expect": expect,
                })
            })
            .collect(),
    )
}

fn bid_cases(
    d: &ChunkyDealing,
    id: &[u8; 32],
    dk: &G1Affine,
    other_dk: &G1Affine,
) -> (Value, Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(DEALING_SEED ^ 0xB1D);
    let ad = bid_ad(1, 5);
    let ct = encrypt_bid(&d.mpk, id, &ad, BID_PLAINTEXT, &mut rng).unwrap();
    let wire = ct.to_bytes();
    assert_eq!(wire.len(), 33 + 97 + 13 + 1 + BID_PLAINTEXT.len() + 16 + 65);

    let tamper = |f: &dyn Fn(&mut BidCiphertext)| -> Vec<u8> {
        let mut c = ct.clone();
        f(&mut c);
        c.to_bytes()
    };

    // A correctly signed ciphertext whose U is on the curve but outside the subgroup.
    let sk = SigningKey::from_bytes(&[5u8; 32]);
    let mut non_subgroup_u = ct.clone();
    non_subgroup_u.vk = sk.verifying_key().to_bytes();
    non_subgroup_u.u = raw_g2(&non_subgroup_g2()).try_into().unwrap();
    non_subgroup_u.sig = sk.sign(&non_subgroup_u.signed_message(&ad)).to_bytes();

    // A copier who re-signs the body under a new one time key. It passes the signature check.
    // The key derivation binds `vk`, so the body does not decrypt.
    let mut resigned = ct.clone();
    resigned.vk = sk.verifying_key().to_bytes();
    resigned.sig = sk.sign(&resigned.signed_message(&ad)).to_bytes();

    let mut truncated = wire.clone();
    truncated.pop();
    let mut extended = wire.clone();
    extended.push(0);

    // (name, ct bytes, ad, dk used to open)
    let other_ad = bid_ad(2, 5);
    let cases: Vec<(&str, Vec<u8>, Vec<u8>, G1Affine)> = vec![
        ("valid", wire.clone(), ad.clone(), *dk),
        (
            "valid, opened with the key of another identity",
            wire.clone(),
            ad.clone(),
            *other_dk,
        ),
        ("tampered vk", tamper(&|c| c.vk[0] ^= 1), ad.clone(), *dk),
        ("tampered U", tamper(&|c| c.u[10] ^= 1), ad.clone(), *dk),
        (
            "tampered nonce",
            tamper(&|c| c.nonce[0] ^= 1),
            ad.clone(),
            *dk,
        ),
        ("tampered body", tamper(&|c| c.c[0] ^= 1), ad.clone(), *dk),
        (
            "tampered tag",
            tamper(&|c| {
                let n = c.c.len();
                c.c[n - 1] ^= 1
            }),
            ad.clone(),
            *dk,
        ),
        (
            "tampered signature",
            tamper(&|c| c.sig[0] ^= 1),
            ad.clone(),
            *dk,
        ),
        (
            "another bidder in the associated data",
            wire.clone(),
            other_ad,
            *dk,
        ),
        (
            "last byte of the associated data flipped",
            wire.clone(),
            flip(ad.clone(), ad.len() - 1),
            *dk,
        ),
        (
            "U outside the subgroup, validly signed",
            non_subgroup_u.to_bytes(),
            ad.clone(),
            *dk,
        ),
        (
            "body re-signed under a new one time key",
            resigned.to_bytes(),
            ad.clone(),
            *dk,
        ),
        ("truncated wire", truncated, ad.clone(), *dk),
        ("extended wire", extended, ad.clone(), *dk),
        ("empty wire", vec![], ad.clone(), *dk),
    ];

    let out = cases
        .into_iter()
        .map(|(name, ct_bytes, ad_bytes, key)| {
            let verifies = verify_bid_ct(&ct_bytes, &ad_bytes).is_ok();
            let opens = BidCiphertext::from_bytes(&ct_bytes)
                .ok()
                .and_then(|c| decrypt_bid(&key, id, &ad_bytes, &c).ok());
            // The key a case opens with is `dk` unless it says otherwise. Move re-derives it from
            // the `opens_with` field.
            let opens_with = if key == *dk { "dk" } else { "other_dk" };
            json!({
                "name": name,
                "ct": hx(&ct_bytes),
                "ad": hx(&ad_bytes),
                "verifies": verifies,
                "opens_with": opens_with,
                "opens": opens.is_some(),
                "plaintext": opens.map(|p| hx(&p)),
            })
        })
        .collect();
    (Value::Array(out), ad)
}

/// Builds the whole vector set from the seeds. Pure and deterministic.
pub fn generate_json() -> String {
    let d = chunky_dealing();
    let id = compute_identity(CHAIN_ID, EPOCH, TIMELOCK_ID, DEADLINE_US);
    let other_id = compute_identity(CHAIN_ID, EPOCH, TIMELOCK_ID + 1, DEADLINE_US);
    let shares = nested_dk_shares(&d, &id);
    let other_shares = nested_dk_shares(&d, &other_id);
    let dk = reconstruct(&shares, &[0, 2]).unwrap();
    let other_dk = reconstruct(&other_shares, &[0, 2]).unwrap();
    assert!(verify_decryption_key(&dk, &id, &d.mpk));
    assert!(verify_decryption_key(&other_dk, &other_id, &d.mpk));

    // Every verification key matches its share: the node's startup check `vk = g2 * share`.
    for (v, per_validator) in d.shares.iter().enumerate() {
        for (j, s) in per_validator.iter().enumerate() {
            assert_eq!(master_public_key(s), d.vks[v][j]);
        }
    }

    let (bids, bid_ad_bytes) = bid_cases(&d, &id, &dk, &other_dk);

    let doc = json!({
        "version": "3.0.0",
        "scheme": "Boneh-Franklin timelock V2 (arkworks), chunky-derived keys, CCA bid wrapper",
        "spec": "atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md sections 5 and 6",
        "generator": GENERATOR_COMMAND,
        "encoding": {
            "g1": "ZCash compressed, 48 bytes, validated decoding",
            "g2": "ZCash compressed, 96 bytes, validated decoding",
            "identity": "SHA3-256(\"ATOMICA_TIMELOCK_IBE_V2_ID\" || chain_id u8 || epoch u64 le || timelock_id u64 le || deadline_us u64 le)",
            "h1": "RFC 9380 BLS12381G1_XMD:SHA-256_SSWU_RO_, DST \"ATOMICA_TIMELOCK_IBE_V2_H1\"",
            "bid_ct": "BCS of five byte vectors: vk(32), U(96), nonce(12), C(body + 16 byte tag), sig(64)",
        },
        "identity_vectors": identity_vectors(),
        "cross_library_vectors": cross_library_vectors(),
        "dealing": {
            "seed": DEALING_SEED,
            "chain_id": CHAIN_ID,
            "epoch": EPOCH,
            "timelock_id": TIMELOCK_ID,
            "deadline_us": DEADLINE_US,
            "weights": WEIGHTS,
            "threshold_weight": THRESHOLD,
            "total_weight": WEIGHTS.iter().sum::<usize>(),
            "ell": DEFAULT_ELL_FOR_TESTING,
            "dealers": WEIGHTS.len(),
            "session_id": "BCS((\"atomica/timelock-chunky-dkg/v1\", chain_id, epoch))",
            "aggregated_subtranscript_bcs_len": d.subtranscript_bcs.len(),
            "aggregated_subtranscript_bcs_sha3_256": hx(&Sha3_256::digest(&d.subtranscript_bcs)),
            "mpk": hx(&encode_g2(&d.mpk)),
            "verification_keys": d.vks.iter()
                .map(|v| v.iter().map(|p| hx(&encode_g2(p))).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "identity": hx(&id),
            "h1": hx(&encode_g1(&hash_to_g1(&id).unwrap())),
            "other_identity": hx(&other_id),
            "dk_shares": shares.iter()
                .map(|v| v.iter().map(|s| hx(s)).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "other_dk_shares": other_shares.iter()
                .map(|v| v.iter().map(|s| hx(s)).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "dk": hx(&encode_g1(&dk)),
            "other_dk": hx(&encode_g1(&other_dk)),
            "share_cases": share_cases(&d, &id, &shares),
            "reconstruction_cases": reconstruction_cases(&d, &id, &shares),
            "key_check_cases": key_check_cases(&d, &id, &dk, &other_dk),
        },
        "bids": {
            "plaintext": hx(BID_PLAINTEXT),
            "ad": hx(&bid_ad_bytes),
            "cases": bids,
        },
    });
    let mut s = serde_json::to_string_pretty(&doc).unwrap();
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------------------------
// Move fixtures
// ---------------------------------------------------------------------------------------------

fn mv_bytes(v: &Value) -> String {
    format!("x\"{}\"", v.as_str().unwrap())
}

fn mv_vec<T: AsRef<str>>(items: &[T]) -> String {
    format!(
        "vector[{}]",
        items
            .iter()
            .map(|i| i.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn mv_nested(v: &Value) -> String {
    let rows: Vec<String> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let cells: Vec<String> = row.as_array().unwrap().iter().map(mv_bytes).collect();
            format!("            {}", mv_vec(&cells))
        })
        .collect();
    format!("vector[\n{},\n        ]", rows.join(",\n"))
}

/// Renders the Move fixture module from the JSON. The accessors keep the names the registry tests
/// use (`timelock_basic_*`).
pub fn generate_move(json_text: &str) -> String {
    let doc: Value = serde_json::from_str(json_text).unwrap();
    let dealing = &doc["dealing"];
    let bids = &doc["bids"];
    let weights: Vec<String> = dealing["weights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.to_string())
        .collect();

    let mut o = String::new();
    o.push_str(&format!(
        "/// IBE golden vector fixtures (test vectors for the hardened Boneh-Franklin timelock, V2).\n\
///\n\
/// GENERATED. Do not edit by hand. Source of truth: `atomica/golden_vectors/ibe_v2_golden_vectors.json`.\n\
/// Regenerate with\n\
/// `{GENERATOR_COMMAND}`.\n\
///\n\
/// The master public key, the verification keys and the shares come from a seeded real chunky DKG\n\
/// dealing (weights {w:?}, threshold weight {t}). Identity: `ibe_config::compute_identity(chain {c}, epoch {e}, id {i}, deadline {dl})`.\n\
/// The legacy V1 vectors (old identity hash, blstrs shares) are kept as history in\n\
/// `atomica/golden_vectors/legacy-v1/` and are not used by any test.\n\
module aptos_framework::ibe_golden_vector_fixtures {{\n\
    // ---- dealing\n\
    public fun chain_id(): u8 {{ {c} }}\n\
    public fun epoch(): u64 {{ {e} }}\n\
    public fun timelock_id(): u64 {{ {i} }}\n\
    public fun deadline_us(): u64 {{ {dl} }}\n\
    public fun threshold(): u64 {{ {t} }}\n\
    public fun total_weight(): u64 {{ {tw} }}\n\
    public fun validator_weights(): vector<u64> {{ {ws} }}\n\
\n\
    public fun identity(): vector<u8> {{ {id} }}\n\
    public fun other_identity(): vector<u8> {{ {oid} }}\n\
    /// `H1(identity())`, RFC 9380 BLS12381G1_XMD:SHA-256_SSWU_RO_ with DST `ATOMICA_TIMELOCK_IBE_V2_H1`.\n\
    public fun h1(): vector<u8> {{ {h1} }}\n\
    public fun mpk(): vector<u8> {{ {mpk} }}\n\
    public fun dk(): vector<u8> {{ {dk} }}\n\
    public fun other_dk(): vector<u8> {{ {odk} }}\n\
\n\
    /// `verification_keys()[validator][j] = g2^(s_p)` for the validator's virtual player `j`.\n\
    public fun verification_keys(): vector<vector<vector<u8>>> {{\n        {vks}\n    }}\n\
\n\
    /// `dk_shares()[validator][j] = s_p * H1(identity)`.\n\
    public fun dk_shares(): vector<vector<vector<u8>>> {{\n        {sh}\n    }}\n\
\n\
    /// Same shares for `other_identity()`.\n\
    public fun other_dk_shares(): vector<vector<vector<u8>>> {{\n        {osh}\n    }}\n\
\n\
    // ---- names the registry tests use\n\
    public fun timelock_basic_mpk(): vector<u8> {{ mpk() }}\n\
    public fun timelock_basic_reconstructed_dk(): vector<u8> {{ dk() }}\n\
\n",
        w = WEIGHTS,
        t = THRESHOLD,
        c = CHAIN_ID,
        e = EPOCH,
        i = TIMELOCK_ID,
        dl = DEADLINE_US,
        tw = dealing["total_weight"],
        ws = mv_vec(&weights),
        id = mv_bytes(&dealing["identity"]),
        oid = mv_bytes(&dealing["other_identity"]),
        h1 = mv_bytes(&dealing["h1"]),
        mpk = mv_bytes(&dealing["mpk"]),
        dk = mv_bytes(&dealing["dk"]),
        odk = mv_bytes(&dealing["other_dk"]),
        vks = mv_nested(&dealing["verification_keys"]),
        sh = mv_nested(&dealing["dk_shares"]),
        osh = mv_nested(&dealing["other_dk_shares"]),
    ));

    // identity vectors
    let iv = doc["identity_vectors"].as_array().unwrap();
    o.push_str("    // ---- identity and H1 vectors, index aligned\n");
    for (name, field, ty, bytes) in [
        ("identity_vector_chains", "chain_id", "u8", false),
        ("identity_vector_epochs", "epoch", "u64", false),
        ("identity_vector_timelock_ids", "timelock_id", "u64", false),
        ("identity_vector_deadlines", "deadline_us", "u64", false),
        ("identity_vector_identities", "identity", "vector<u8>", true),
        ("identity_vector_h1s", "h1", "vector<u8>", true),
    ] {
        let items: Vec<String> = iv
            .iter()
            .map(|c| {
                if bytes {
                    mv_bytes(&c[field])
                } else {
                    c[field].to_string()
                }
            })
            .collect();
        o.push_str(&format!(
            "    public fun {name}(): vector<{ty}> {{ {} }}\n",
            mv_vec(&items)
        ));
    }
    o.push('\n');

    // reconstruction cases: validator index lists and expected key checks
    let recon: Vec<&Value> = dealing["reconstruction_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["expect"] == "ok" && c.get("shares").is_none())
        .collect();
    o.push_str("    // ---- reconstruction: validator sets that rebuild `dk()`\n");
    o.push_str(&format!(
        "    public fun reconstruction_sets(): vector<vector<u64>> {{ {} }}\n",
        mv_vec(
            &recon
                .iter()
                .map(|c| mv_vec(
                    &c["validators"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| format!("{v}"))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        )
    ));
    let below: Vec<&Value> = dealing["reconstruction_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["expect"] == "below_threshold")
        .collect();
    o.push_str(&format!(
        "    public fun below_threshold_sets(): vector<vector<u64>> {{ {} }}\n\n",
        mv_vec(
            &below
                .iter()
                .map(|c| mv_vec(
                    &c["validators"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| format!("{v}"))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        )
    ));
    let poisoned = dealing["reconstruction_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key_check"] == false)
        .unwrap();
    o.push_str("    /// Validators 0 and 2 with one share taken from another identity.\n");
    o.push_str(&format!(
        "    public fun poisoned_shares(): vector<vector<vector<u8>>> {{\n        {}\n    }}\n",
        mv_nested(&poisoned["shares"])
    ));
    o.push_str(&format!(
        "    public fun poisoned_dk(): vector<u8> {{ {} }}\n\n",
        mv_bytes(&poisoned["dk"])
    ));

    // share cases
    let cases = dealing["share_cases"].as_array().unwrap();
    o.push_str("    // ---- per validator share checks. `share_case_expect_ok()[i]` is true only for valid sets.\n");
    o.push_str(&format!(
        "    public fun share_case_validators(): vector<u64> {{ {} }}\n",
        mv_vec(
            &cases
                .iter()
                .map(|c| c["validator"].to_string())
                .collect::<Vec<_>>()
        )
    ));
    o.push_str(&format!(
        "    public fun share_case_expect_ok(): vector<bool> {{ {} }}\n",
        mv_vec(
            &cases
                .iter()
                .map(|c| (c["expect"] == "ok").to_string())
                .collect::<Vec<_>>()
        )
    ));
    o.push_str(&format!(
        "    public fun share_case_shares(): vector<vector<vector<u8>>> {{\n        {}\n    }}\n",
        mv_nested(&Value::Array(
            cases.iter().map(|c| c["shares"].clone()).collect()
        ))
    ));
    o.push_str(&format!(
        "    public fun share_case_keys(): vector<vector<vector<u8>>> {{\n        {}\n    }}\n\n",
        mv_nested(&Value::Array(
            cases
                .iter()
                .map(|c| c["verification_keys"].clone())
                .collect()
        ))
    ));

    // key check cases
    let kc = dealing["key_check_cases"].as_array().unwrap();
    o.push_str("    // ---- final key check `e(dk, g2) = e(H1(id), mpk)`\n");
    for (name, field, is_bool) in [
        ("key_check_dks", "dk", false),
        ("key_check_identities", "identity", false),
        ("key_check_mpks", "mpk", false),
        ("key_check_expect", "expect", true),
    ] {
        let items: Vec<String> = kc
            .iter()
            .map(|c| {
                if is_bool {
                    c[field].to_string()
                } else {
                    mv_bytes(&c[field])
                }
            })
            .collect();
        let ty = if is_bool { "bool" } else { "vector<u8>" };
        o.push_str(&format!(
            "    public fun {name}(): vector<{ty}> {{ {} }}\n",
            mv_vec(&items)
        ));
    }

    // bids
    let bc = bids["cases"].as_array().unwrap();
    o.push_str(&format!(
        "\n    // ---- bid ciphertexts. Case 0 is valid.\n    public fun bid_plaintext(): vector<u8> {{ {} }}\n    public fun bid_ad(): vector<u8> {{ {} }}\n",
        mv_bytes(&bids["plaintext"]),
        mv_bytes(&bids["ad"])
    ));
    for (name, field, kind) in [
        ("bid_case_cts", "ct", "bytes"),
        ("bid_case_ads", "ad", "bytes"),
        ("bid_case_verifies", "verifies", "bool"),
        ("bid_case_opens", "opens", "bool"),
        ("bid_case_opens_with_other_dk", "opens_with", "other"),
    ] {
        let items: Vec<String> = bc
            .iter()
            .map(|c| match kind {
                "bytes" => mv_bytes(&c[field]),
                "bool" => c[field].to_string(),
                _ => (c[field] == "other_dk").to_string(),
            })
            .collect();
        let ty = if kind == "bytes" {
            "vector<u8>"
        } else {
            "bool"
        };
        o.push_str(&format!(
            "    public fun {name}(): vector<{ty}> {{ {} }}\n",
            mv_vec(&items)
        ));
    }
    o.push_str("}\n");
    o
}

// ---------------------------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------------------------

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Writes both outputs. Deterministic: seeded dealing, seeded bids.
#[test]
#[ignore]
fn write_golden_vectors() {
    let json_text = generate_json();
    let move_text = generate_move(&json_text);
    std::fs::write(repo_root().join(JSON_PATH), &json_text).unwrap();
    std::fs::write(repo_root().join(MOVE_PATH), &move_text).unwrap();
    println!("wrote {JSON_PATH} ({} bytes)", json_text.len());
    println!("wrote {MOVE_PATH} ({} bytes)", move_text.len());
}

// ---------------------------------------------------------------------------------------------
// Tests against the checked-in bytes
// ---------------------------------------------------------------------------------------------

#[test]
fn regeneration_reproduces_checked_in_bytes() {
    let json_text = generate_json();
    assert!(
        json_text == CHECKED_IN_JSON,
        "the checked-in JSON differs from a fresh generation: rerun `{GENERATOR_COMMAND}`"
    );
    assert!(
        generate_move(&json_text) == CHECKED_IN_MOVE,
        "the checked-in Move fixtures differ from a fresh generation: rerun `{GENERATOR_COMMAND}`"
    );
    // A second generation in the same process gives the same bytes (no hidden entropy).
    assert!(generate_json() == json_text);
}

#[test]
fn checked_in_vectors_verify() {
    let doc: Value = serde_json::from_str(CHECKED_IN_JSON).unwrap();

    // Identity and H1.
    for v in doc["identity_vectors"].as_array().unwrap() {
        let identity = compute_identity(
            v["chain_id"].as_u64().unwrap() as u8,
            v["epoch"].as_u64().unwrap(),
            v["timelock_id"].as_u64().unwrap(),
            v["deadline_us"].as_u64().unwrap(),
        );
        assert_eq!(identity.to_vec(), unhex(&v["identity"]), "{}", v["name"]);
        let h1 = hash_to_g1(&identity).unwrap();
        assert_eq!(encode_g1(&h1).to_vec(), unhex(&v["h1"]), "{}", v["name"]);
        assert_eq!(unhex(&v["h1"]).len(), G1_COMPRESSED_LEN);
    }

    let dl = &doc["dealing"];
    let id = unhex(&dl["identity"]);
    assert_eq!(
        id,
        compute_identity(
            dl["chain_id"].as_u64().unwrap() as u8,
            dl["epoch"].as_u64().unwrap(),
            dl["timelock_id"].as_u64().unwrap(),
            dl["deadline_us"].as_u64().unwrap()
        )
        .to_vec()
    );
    let mpk_bytes = unhex(&dl["mpk"]);
    let mpk = decode_g2(&mpk_bytes).unwrap();
    assert_eq!(mpk_bytes.len(), G2_COMPRESSED_LEN);
    assert_eq!(
        encode_g1(&hash_to_g1(&id).unwrap()).to_vec(),
        unhex(&dl["h1"])
    );

    // Verification keys: shape follows the weights, every key decodes with validation.
    let weights: Vec<u64> = dl["weights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_u64().unwrap())
        .collect();
    let threshold = dl["threshold_weight"].as_u64().unwrap();
    let total = dl["total_weight"].as_u64().unwrap();
    let vks = dl["verification_keys"].as_array().unwrap();
    assert_eq!(vks.len(), weights.len());
    for (v, row) in vks.iter().enumerate() {
        assert_eq!(row.as_array().unwrap().len() as u64, weights[v]);
        for k in row.as_array().unwrap() {
            let _ = decode_g2(&unhex(k)).unwrap();
        }
    }

    // Per validator share checks of the honest shares.
    let to_bytes = |v: &Value| -> Vec<Vec<Vec<u8>>> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|row| row.as_array().unwrap().iter().map(unhex).collect())
            .collect()
    };
    let shares = to_bytes(&dl["dk_shares"]);
    for (v, row) in shares.iter().enumerate() {
        let keys: Vec<Vec<u8>> = vks[v].as_array().unwrap().iter().map(unhex).collect();
        assert_eq!(verify_player_shares(&id, row, &keys), Ok(()));
    }

    // The checked-in share cases give the recorded verdicts.
    for c in dl["share_cases"].as_array().unwrap() {
        let sh: Vec<Vec<u8>> = c["shares"].as_array().unwrap().iter().map(unhex).collect();
        let keys: Vec<Vec<u8>> = c["verification_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(unhex)
            .collect();
        assert_eq!(
            failure_code(verify_player_shares(&id, &sh, &keys)),
            c["expect"].as_str().unwrap(),
            "share case {}",
            c["name"]
        );
    }
    let valid_cases = dl["share_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["expect"] == "ok")
        .count();
    assert_eq!(valid_cases, 2);

    // Reconstruction and the key check.
    let dk = decode_g1(&unhex(&dl["dk"])).unwrap();
    assert!(verify_decryption_key(&dk, &id, &mpk));
    for c in dl["reconstruction_cases"].as_array().unwrap() {
        let who: Vec<u64> = c["validators"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        let input: Vec<Vec<Vec<u8>>> = match c.get("shares") {
            Some(s) => to_bytes(s),
            None => who.iter().map(|v| shares[*v as usize].clone()).collect(),
        };
        let got = reconstruct_ibe_dk_from_g1_shares(&who, &input, &weights, threshold, total);
        match c["expect"].as_str().unwrap() {
            "below_threshold" => {
                assert!(
                    matches!(got, Err(IbeError::BelowThreshold { .. })),
                    "{}",
                    c["name"]
                )
            },
            _ => {
                let got = got.unwrap();
                assert_eq!(encode_g1(&got).to_vec(), unhex(&c["dk"]), "{}", c["name"]);
                assert_eq!(
                    verify_decryption_key(&got, &id, &mpk),
                    c["key_check"].as_bool().unwrap(),
                    "{}",
                    c["name"]
                );
            },
        }
    }
    for c in dl["key_check_cases"].as_array().unwrap() {
        let got = match (decode_g1(&unhex(&c["dk"])), decode_g2(&unhex(&c["mpk"]))) {
            (Ok(k), Ok(m)) => verify_decryption_key(&k, &unhex(&c["identity"]), &m),
            _ => false,
        };
        assert_eq!(got, c["expect"].as_bool().unwrap(), "{}", c["name"]);
    }

    // Bids: the submission check, then opening with the revealed key.
    let id32: [u8; 32] = id.clone().try_into().unwrap();
    let other_dk = decode_g1(&unhex(&dl["other_dk"])).unwrap();
    let plaintext = unhex(&doc["bids"]["plaintext"]);
    let mut verified = 0;
    for c in doc["bids"]["cases"].as_array().unwrap() {
        let ct = unhex(&c["ct"]);
        let ad = unhex(&c["ad"]);
        let verifies = verify_bid_ct(&ct, &ad).is_ok();
        assert_eq!(verifies, c["verifies"].as_bool().unwrap(), "{}", c["name"]);
        let key = if c["opens_with"] == "other_dk" {
            other_dk
        } else {
            dk
        };
        let opened = BidCiphertext::from_bytes(&ct)
            .ok()
            .and_then(|b| decrypt_bid(&key, &id32, &ad, &b).ok());
        assert_eq!(
            opened.is_some(),
            c["opens"].as_bool().unwrap(),
            "{}",
            c["name"]
        );
        if let Some(p) = opened {
            assert_eq!(p, plaintext, "{}", c["name"]);
        }
        verified += 1;
    }
    assert_eq!(verified, 15);
    // Only the first case opens with the right key.
    let opens: Vec<bool> = doc["bids"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["opens"].as_bool().unwrap())
        .collect();
    assert_eq!(opens.iter().filter(|b| **b).count(), 1);
    assert!(opens[0]);
}

#[test]
fn checked_in_cross_library_vectors_match_both_libraries() {
    use group::{Curve, Group};
    use std::ops::Mul as _;

    let doc: Value = serde_json::from_str(CHECKED_IN_JSON).unwrap();
    let cases = doc["cross_library_vectors"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for c in cases {
        let id = unhex(&c["identity"]);
        let s_le: [u8; 32] = unhex(&c["secret_le"]).try_into().unwrap();
        let r_le: [u8; 32] = unhex(&c["r_le"]).try_into().unwrap();
        let (h1, mpk, dk, u) = (
            unhex(&c["h1"]),
            unhex(&c["mpk"]),
            unhex(&c["dk"]),
            unhex(&c["u"]),
        );

        // blstrs (the legacy library).
        let bs = blstrs::Scalar::from_bytes_le(&s_le).unwrap();
        let br = blstrs::Scalar::from_bytes_le(&r_le).unwrap();
        let q_b = blstrs::G1Projective::hash_to_curve(&id, IBE_H1_DST, b"").to_affine();
        assert_eq!(q_b.to_compressed().to_vec(), h1, "blstrs H1");
        assert_eq!(
            blstrs::G2Projective::generator()
                .mul(bs)
                .to_affine()
                .to_compressed()
                .to_vec(),
            mpk,
            "blstrs mpk"
        );
        assert_eq!(
            blstrs::G1Projective::from(q_b)
                .mul(bs)
                .to_affine()
                .to_compressed()
                .to_vec(),
            dk,
            "blstrs dk"
        );
        assert_eq!(
            blstrs::G2Projective::generator()
                .mul(br)
                .to_affine()
                .to_compressed()
                .to_vec(),
            u,
            "blstrs U"
        );

        // arkworks (the port).
        let s = Fr::from_le_bytes_mod_order(&s_le);
        let r = Fr::from_le_bytes_mod_order(&r_le);
        assert_eq!(
            encode_g1(&hash_to_g1(&id).unwrap()).to_vec(),
            h1,
            "arkworks H1"
        );
        assert_eq!(
            encode_g2(&master_public_key(&s)).to_vec(),
            mpk,
            "arkworks mpk"
        );
        assert_eq!(
            encode_g1(&derive_decryption_key(&s, &id).unwrap()).to_vec(),
            dk,
            "arkworks dk"
        );
        let (u_a, k_a) = encapsulate_with_scalar(&master_public_key(&s), &id, &r).unwrap();
        assert_eq!(encode_g2(&u_a).to_vec(), u, "arkworks U");
        assert_eq!(
            decapsulate(&derive_decryption_key(&s, &id).unwrap(), &u_a),
            k_a
        );
    }
}
