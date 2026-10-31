// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    ciphertext::{decrypt_bid, encrypt_bid, verify_bid_ct, BidCiphertext, SIG_LEN},
    *,
};
use ark_bls12_381::{Fq, Fq2};
use ark_ff::Field;
use ark_serialize::Compress;
use ed25519_dalek::{Signer, SigningKey};
use rand::{rngs::StdRng, SeedableRng};

fn rng(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

/// A chunky-DKG-shaped dealing: master secret, per virtual player shares and verification keys
/// `g2^{s_p}`. The product never holds `s`. A test may.
struct Dealing {
    secret: Fr,
    mpk: G2Affine,
    shares: Vec<Fr>,
    vks: Vec<G2Affine>,
    weights: Vec<u64>,
    threshold: usize,
    total: usize,
}

fn deal(weights: &[u64], threshold: usize, seed: u64) -> Dealing {
    let total: usize = weights.iter().sum::<u64>() as usize;
    let mut r = rng(seed);
    let secret = random_scalar(&mut r);
    let config = ShamirThresholdConfig::<Fr>::new(threshold, total);
    let (_, shares) = config.sample_polynomial_and_compute_shares(secret, &mut r);
    let vks = shares.iter().map(master_public_key).collect();
    Dealing {
        secret,
        mpk: master_public_key(&secret),
        shares,
        vks,
        weights: weights.to_vec(),
        threshold,
        total,
    }
}

impl Dealing {
    fn start(&self, validator: usize) -> usize {
        self.weights[..validator].iter().sum::<u64>() as usize
    }

    /// Shares of the validators in `who`, in the nested layout the native takes.
    fn dk_shares(&self, who: &[usize], id: &[u8]) -> Vec<Vec<Vec<u8>>> {
        who.iter()
            .map(|&v| {
                (0..self.weights[v] as usize)
                    .map(|j| {
                        let s = &self.shares[self.start(v) + j];
                        encode_g1(&derive_decryption_key(s, id).unwrap()).to_vec()
                    })
                    .collect()
            })
            .collect()
    }

    fn reconstruct(&self, who: &[usize], id: &[u8]) -> Result<G1Affine, IbeError> {
        let idx: Vec<u64> = who.iter().map(|v| *v as u64).collect();
        reconstruct_ibe_dk_from_g1_shares(
            &idx,
            &self.dk_shares(who, id),
            &self.weights,
            self.threshold as u64,
            self.total as u64,
        )
    }
}

fn identity() -> [u8; 32] {
    compute_identity(7, 3, 11, 1_700_000_000_000_000)
}

/// A point on the curve but outside the prime-order subgroup.
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

/// A compressed encoding whose x has no y on the curve.
fn off_curve_g1() -> Vec<u8> {
    let mut x = Fq::from(1u64);
    loop {
        if G1Affine::get_point_from_x_unchecked(x, false).is_none() {
            let mut v = vec![];
            x.serialize_compressed(&mut v).unwrap();
            v.reverse(); // big endian, zcash layout
            v[0] |= 0x80; // compressed flag
            return v;
        }
        x += Fq::ONE;
    }
}

// ---------------------------------------------------------------------------------------------
// Identity, H1 and domain separation
// ---------------------------------------------------------------------------------------------

#[test]
fn identity_layout_and_sensitivity() {
    let mut h = Sha3_256::new();
    h.update(b"ATOMICA_TIMELOCK_IBE_V2_ID");
    h.update([7u8]);
    h.update(3u64.to_le_bytes());
    h.update(11u64.to_le_bytes());
    h.update(1_700_000_000_000_000u64.to_le_bytes());
    let expected: [u8; 32] = h.finalize().into();
    assert_eq!(identity(), expected);
    let base = identity();
    assert_ne!(base, compute_identity(8, 3, 11, 1_700_000_000_000_000));
    assert_ne!(base, compute_identity(7, 4, 11, 1_700_000_000_000_000));
    assert_ne!(base, compute_identity(7, 3, 12, 1_700_000_000_000_000));
    assert_ne!(base, compute_identity(7, 3, 11, 1_700_000_000_000_001));
}

#[test]
fn domain_separation() {
    // Distinct tags and salts, none shared with the mempool batch-encryption or the retired code.
    let tags: [&[u8]; 6] = [
        IBE_H1_DST,
        IBE_IDENTITY_TAG,
        IBE_KDF_SALT,
        b"APTOS_BATCH_ENCRYPTION_HASH_G2_ELEMENT",
        b"APTOS_BATCH_ENCRYPTION_OTP",
        b"APTOS_IBE_IDENTITY_DST",
    ];
    for (i, a) in tags.iter().enumerate() {
        for b in &tags[i + 1..] {
            assert_ne!(a, b);
        }
    }
    // The same input hashed under the mempool DST lands on a different point.
    let id = identity();
    let other = G1Hasher::new(b"APTOS_BATCH_ENCRYPTION_HASH_G2_ELEMENT")
        .unwrap()
        .hash(&id)
        .unwrap();
    assert_ne!(other, hash_to_g1(&id).unwrap());
    // H1 output is a valid subgroup point.
    let q = hash_to_g1(&id).unwrap();
    assert!(q.is_on_curve() && q.is_in_correct_subgroup_assuming_on_curve() && !q.is_zero());
}

// ---------------------------------------------------------------------------------------------
// Cross library: arkworks port against the blstrs reference on shared vectors
// ---------------------------------------------------------------------------------------------

#[test]
fn cross_library_vectors_match_blstrs() {
    use group::{Curve, Group};
    use std::ops::Mul as _;

    for seed in 0..8u64 {
        let mut r = rng(1000 + seed);
        let id = compute_identity(seed as u8, seed, seed * 3 + 1, 1_000_000 * (seed + 1));
        let s = random_scalar(&mut r);
        let rr = random_scalar(&mut r);

        let mut s_le = [0u8; 32];
        s.serialize_compressed(&mut s_le[..]).unwrap();
        let mut r_le = [0u8; 32];
        rr.serialize_compressed(&mut r_le[..]).unwrap();
        let bs = blstrs::Scalar::from_bytes_le(&s_le).unwrap();
        let br = blstrs::Scalar::from_bytes_le(&r_le).unwrap();

        // H1: same RFC 9380 suite and DST, empty augmentation.
        let q_b = blstrs::G1Projective::hash_to_curve(&id, IBE_H1_DST, b"").to_affine();
        let q_a = hash_to_g1(&id).unwrap();
        assert_eq!(
            q_b.to_compressed().to_vec(),
            encode_g1(&q_a).to_vec(),
            "H1 seed {seed}"
        );

        // mpk = g2^s
        let mpk_b = blstrs::G2Projective::generator().mul(bs).to_affine();
        assert_eq!(
            mpk_b.to_compressed().to_vec(),
            encode_g2(&master_public_key(&s)).to_vec(),
            "mpk seed {seed}"
        );

        // dk = s * H1(id)
        let dk_b = blstrs::G1Projective::from(q_b).mul(bs).to_affine();
        let dk_a = derive_decryption_key(&s, &id).unwrap();
        assert_eq!(
            dk_b.to_compressed().to_vec(),
            encode_g1(&dk_a).to_vec(),
            "dk seed {seed}"
        );

        // U = g2^r
        let u_b = blstrs::G2Projective::generator().mul(br).to_affine();
        let (u_a, k_a) = encapsulate_with_scalar(&master_public_key(&s), &id, &rr).unwrap();
        assert_eq!(
            u_b.to_compressed().to_vec(),
            encode_g2(&u_a).to_vec(),
            "U seed {seed}"
        );

        // K = e(Q, mpk)^r agrees under both libraries: e(dk_b, U) from blstrs equals the arkworks
        // K when both are compared as pairings of identical inputs.
        assert_eq!(decapsulate(&dk_a, &u_a), k_a);
        let lhs = blstrs::pairing(&dk_b, &u_b);
        let rhs = blstrs::pairing(&q_b, &mpk_b).mul(br);
        assert_eq!(lhs, rhs);

        // Cross decode: blstrs bytes decode under the validated arkworks decoder.
        assert_eq!(decode_g1(&dk_b.to_compressed()).unwrap(), dk_a);
        assert_eq!(decode_g2(&u_b.to_compressed()).unwrap(), u_a);
    }
}

// ---------------------------------------------------------------------------------------------
// Threshold round trip, share checks, key checks
// ---------------------------------------------------------------------------------------------

#[test]
fn threshold_round_trip_with_share_and_key_checks() {
    let d = deal(&[3, 1, 2, 4], 7, 1);
    let id = identity();
    // Every share verifies against its verification key.
    for (p, s) in d.shares.iter().enumerate() {
        let share = derive_decryption_key(s, &id).unwrap();
        assert!(verify_dk_share(&share, &id, &d.vks[p]), "share {p}");
    }
    // Validators 0 and 3 hold weight 7 = threshold.
    let dk = d.reconstruct(&[0, 3], &id).unwrap();
    assert_eq!(dk, derive_decryption_key(&d.secret, &id).unwrap());
    assert!(verify_decryption_key(&dk, &id, &d.mpk));
    // A different qualified subset gives the same key, and a superset does too.
    assert_eq!(dk, d.reconstruct(&[1, 2, 3], &id).unwrap());
    assert_eq!(dk, d.reconstruct(&[3, 2, 1, 0], &id).unwrap());
}

#[test]
fn below_threshold_is_an_error_not_a_panic() {
    let d = deal(&[3, 1, 2, 4], 7, 2);
    let id = identity();
    // Validators 1 and 2 hold weight 3 only.
    assert!(matches!(
        d.reconstruct(&[1, 2], &id),
        Err(IbeError::BelowThreshold { have: 3, need: 7 })
    ));
}

#[test]
fn bad_share_and_wrong_key_are_caught() {
    let d = deal(&[2, 2, 2, 2], 5, 3);
    let id = identity();
    let share = derive_decryption_key(&d.shares[0], &id).unwrap();

    // Tampered share.
    let bad = (G1Projective::from(share) + G1Projective::from(G1Affine::generator())).into_affine();
    assert!(!verify_dk_share(&bad, &id, &d.vks[0]));
    // Share verified against another player's verification key.
    assert!(!verify_dk_share(&share, &id, &d.vks[1]));
    // Share for another identity.
    let other_id = compute_identity(7, 3, 12, 1_700_000_000_000_000);
    let other = derive_decryption_key(&d.shares[0], &other_id).unwrap();
    assert!(!verify_dk_share(&other, &id, &d.vks[0]));
    // Share of another epoch's secret.
    let d2 = deal(&[2, 2, 2, 2], 5, 4);
    let foreign = derive_decryption_key(&d2.shares[0], &id).unwrap();
    assert!(!verify_dk_share(&foreign, &id, &d.vks[0]));

    // A rebuilt key from a bad share set fails the key check.
    let mut nested = d.dk_shares(&[0, 1, 2], &id);
    nested[1][0] = encode_g1(&bad).to_vec();
    let rebuilt = reconstruct_ibe_dk_from_g1_shares(&[0, 1, 2], &nested, &d.weights, 5, 8).unwrap();
    assert!(!verify_decryption_key(&rebuilt, &id, &d.mpk));
    // The key of another identity or another mpk fails too.
    let good = derive_decryption_key(&d.secret, &id).unwrap();
    assert!(verify_decryption_key(&good, &id, &d.mpk));
    assert!(!verify_decryption_key(&good, &other_id, &d.mpk));
    assert!(!verify_decryption_key(&good, &id, &d2.mpk));
}

// ---------------------------------------------------------------------------------------------
// Validated deserialization and malformed input
// ---------------------------------------------------------------------------------------------

#[test]
fn decoders_reject_malformed_points() {
    let g1 = G1Affine::generator();
    let g2 = G2Affine::generator();
    assert_eq!(decode_g1(&encode_g1(&g1)).unwrap(), g1);
    assert_eq!(decode_g2(&encode_g2(&g2)).unwrap(), g2);

    // Lengths.
    for n in [0usize, 1, 47, 49, 96] {
        assert!(matches!(
            decode_g1(&vec![0u8; n]),
            Err(IbeError::InvalidLength { .. })
        ));
    }
    for n in [0usize, 48, 95, 97] {
        assert!(matches!(
            decode_g2(&vec![0u8; n]),
            Err(IbeError::InvalidLength { .. })
        ));
    }
    // Identity.
    let mut id1 = vec![0u8; 48];
    id1[0] = 0xc0;
    assert!(matches!(decode_g1(&id1), Err(IbeError::InvalidPoint(_))));
    let mut id2 = vec![0u8; 96];
    id2[0] = 0xc0;
    assert!(matches!(decode_g2(&id2), Err(IbeError::InvalidPoint(_))));
    // Not on the curve.
    assert!(matches!(
        decode_g1(&off_curve_g1()),
        Err(IbeError::InvalidPoint(_))
    ));
    // Uncompressed flag cleared.
    let mut no_flag = encode_g1(&g1);
    no_flag[0] &= 0x7f;
    assert!(decode_g1(&no_flag).is_err());
    // On the curve, wrong subgroup.
    let ns1 = non_subgroup_g1();
    assert!(ns1.is_on_curve() && !ns1.is_in_correct_subgroup_assuming_on_curve());
    assert!(matches!(
        decode_g1(&raw_g1(&ns1)),
        Err(IbeError::InvalidPoint(_))
    ));
    let ns2 = non_subgroup_g2();
    assert!(ns2.is_on_curve() && !ns2.is_in_correct_subgroup_assuming_on_curve());
    assert!(matches!(
        decode_g2(&raw_g2(&ns2)),
        Err(IbeError::InvalidPoint(_))
    ));
}

#[test]
fn reconstruction_rejects_bad_inputs_without_panicking() {
    let d = deal(&[2, 2, 2, 2], 5, 5);
    let id = identity();
    let nested = d.dk_shares(&[0, 1, 2], &id);
    let ok = |idx: &[u64], sh: &[Vec<Vec<u8>>], w: &[u64], t: u64, tw: u64| {
        reconstruct_ibe_dk_from_g1_shares(idx, sh, w, t, tw)
    };

    // Out of bounds player id.
    assert!(matches!(
        ok(&[0, 1, 9], &nested, &d.weights, 5, 8),
        Err(IbeError::PlayerOutOfBounds { .. })
    ));
    assert!(matches!(
        ok(&[0, 1, u64::MAX], &nested, &d.weights, 5, 8),
        Err(IbeError::PlayerOutOfBounds { .. })
    ));
    // Duplicate.
    assert!(matches!(
        ok(&[0, 1, 1], &nested, &d.weights, 5, 8),
        Err(IbeError::DuplicatePlayer(1))
    ));
    // Length mismatch.
    assert!(matches!(
        ok(&[0, 1], &nested, &d.weights, 5, 8),
        Err(IbeError::ValidatorIndicesSharesMismatch { .. })
    ));
    // Empty.
    assert!(matches!(
        ok(&[], &[], &d.weights, 5, 8),
        Err(IbeError::EmptyShares)
    ));
    // Weight sum and bounds.
    assert!(matches!(
        ok(&[0, 1, 2], &nested, &d.weights, 5, 9),
        Err(IbeError::WeightSumMismatch { .. })
    ));
    assert!(matches!(
        ok(&[0, 1, 2], &nested, &[u64::MAX, 1, 1, 1], 5, 3),
        Err(IbeError::InvalidWeightedConfig(_))
    ));
    assert!(matches!(
        ok(&[0, 1, 2], &nested, &d.weights, 0, 8),
        Err(IbeError::InvalidWeightedConfig(_))
    ));
    assert!(matches!(
        ok(&[0, 1, 2], &nested, &d.weights, 9, 8),
        Err(IbeError::InvalidWeightedConfig(_))
    ));
    let huge = vec![MAX_TOTAL_WEIGHT as u64 + 1];
    assert!(matches!(
        ok(&[0], &nested[..1], &huge, 1, huge[0]),
        Err(IbeError::InvalidWeightedConfig(_))
    ));
    // Wrong number of shares for the validator's weight.
    let mut short = nested.clone();
    short[0].pop();
    assert!(matches!(
        ok(&[0, 1, 2], &short, &d.weights, 5, 8),
        Err(IbeError::ShareWeightMismatch { .. })
    ));
    // Malformed share bytes of each kind.
    for bad in [vec![0u8; 47], off_curve_g1(), raw_g1(&non_subgroup_g1()), {
        let mut v = vec![0u8; 48];
        v[0] = 0xc0;
        v
    }] {
        let mut n = nested.clone();
        n[2][1] = bad;
        assert!(matches!(
            ok(&[0, 1, 2], &n, &d.weights, 5, 8),
            Err(IbeError::InvalidLength { .. } | IbeError::InvalidPoint(_))
        ));
    }
    // Lower level entry: ids out of the domain, below threshold.
    let g = G1Affine::generator();
    assert!(matches!(
        reconstruct_dk(&[0, 8], &[g, g], 2, 8),
        Err(IbeError::PlayerOutOfBounds { .. })
    ));
    assert!(matches!(
        reconstruct_dk(&[0], &[g], 2, 8),
        Err(IbeError::BelowThreshold { .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// Bid wrapper
// ---------------------------------------------------------------------------------------------

fn bid_ad(bidder: u8, auction: u64, epoch: u64, timelock: u64) -> Vec<u8> {
    bcs::to_bytes(&(
        "atomica/timelock-bid/v2",
        7u8,
        epoch,
        timelock,
        auction,
        [bidder; 32],
        1_700_000_000_000_000u64,
    ))
    .unwrap()
}

struct Setup {
    d: Dealing,
    id: [u8; 32],
    ad: Vec<u8>,
    dk: G1Affine,
}

fn setup() -> Setup {
    let d = deal(&[2, 3, 2], 5, 77);
    let id = compute_identity(7, 3, 11, 1_700_000_000_000_000);
    let dk = d.reconstruct(&[0, 1], &id).unwrap();
    Setup {
        d,
        id,
        ad: bid_ad(1, 5, 3, 11),
        dk,
    }
}

#[test]
fn bid_round_trip() {
    let s = setup();
    let msg = b"bid: 12345 units";
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, msg, &mut rng(9)).unwrap();
    verify_bid_ct(&ct.to_bytes(), &s.ad).unwrap();
    assert_eq!(BidCiphertext::from_bytes(&ct.to_bytes()).unwrap(), ct);
    assert_eq!(decrypt_bid(&s.dk, &s.id, &s.ad, &ct).unwrap(), msg);
    // Empty and large plaintexts.
    for m in [vec![], vec![0xab; 4096]] {
        let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, &m, &mut rng(10)).unwrap();
        assert_eq!(decrypt_bid(&s.dk, &s.id, &s.ad, &ct).unwrap(), m);
    }
    // Fresh randomness per bid.
    let a = encrypt_bid(&s.d.mpk, &s.id, &s.ad, msg, &mut rng(11)).unwrap();
    let b = encrypt_bid(&s.d.mpk, &s.id, &s.ad, msg, &mut rng(12)).unwrap();
    assert_ne!(a.u, b.u);
    assert_ne!(a.vk, b.vk);
    assert_ne!(a.id(&s.ad), b.id(&s.ad));
}

#[test]
fn bid_every_field_and_ad_flip_fails_verification() {
    let s = setup();
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, b"hello", &mut rng(13)).unwrap();
    assert!(ct.verify(&s.ad).is_ok());

    let check = |c: &BidCiphertext, what: &str| {
        assert!(
            verify_bid_ct(&c.to_bytes(), &s.ad).is_err(),
            "flipped {what}"
        );
    };
    let mut c = ct.clone();
    c.vk[0] ^= 1;
    check(&c, "vk");
    let mut c = ct.clone();
    c.u[40] ^= 1;
    check(&c, "U");
    let mut c = ct.clone();
    c.nonce[3] ^= 1;
    check(&c, "nonce");
    let mut c = ct.clone();
    c.c[0] ^= 1;
    check(&c, "C");
    let last = c.c.len() - 1;
    c.c[0] ^= 1;
    c.c[last] ^= 1;
    check(&c, "C tag");
    let mut c = ct.clone();
    c.sig[SIG_LEN - 1] ^= 1;
    check(&c, "sig");
    // AD.
    let mut ad = s.ad.clone();
    ad[0] ^= 1;
    assert!(verify_bid_ct(&ct.to_bytes(), &ad).is_err());
    let mut ad = s.ad.clone();
    let l = ad.len() - 1;
    ad[l] ^= 1;
    assert!(verify_bid_ct(&ct.to_bytes(), &ad).is_err());
    assert!(verify_bid_ct(&ct.to_bytes(), &[]).is_err());
}

#[test]
fn bid_wire_every_byte_flip_and_truncation_fails() {
    let s = setup();
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, b"x", &mut rng(14)).unwrap();
    let wire = ct.to_bytes();
    for i in 0..wire.len() {
        let mut w = wire.clone();
        w[i] ^= 0x01;
        assert!(verify_bid_ct(&w, &s.ad).is_err(), "byte {i}");
    }
    for n in 0..wire.len() {
        assert!(
            verify_bid_ct(&wire[..n], &s.ad).is_err(),
            "truncated to {n}"
        );
    }
    let mut trailing = wire.clone();
    trailing.push(0);
    assert!(verify_bid_ct(&trailing, &s.ad).is_err());
    assert!(verify_bid_ct(&vec![0xffu8; MAX_CIPHERTEXT_BYTES + 1], &s.ad).is_err());
    assert!(verify_bid_ct(&[], &s.ad).is_err());
}

#[test]
fn bid_with_malformed_or_non_subgroup_u_is_rejected_at_submission() {
    let s = setup();
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, b"x", &mut rng(15)).unwrap();
    let resign = |u: [u8; 96]| {
        // A correctly signed ciphertext, so only the group check can reject it.
        let sk = SigningKey::from_bytes(&[5u8; 32]);
        let mut c = ct.clone();
        c.vk = sk.verifying_key().to_bytes();
        c.u = u;
        c.sig = sk.sign(&c.signed_message(&s.ad)).to_bytes();
        c
    };
    // Sanity: the same resigning with a valid U passes the signature check.
    assert!(resign(ct.u).verify(&s.ad).is_ok());

    let mut id2 = [0u8; 96];
    id2[0] = 0xc0;
    let ns: [u8; 96] = raw_g2(&non_subgroup_g2()).try_into().unwrap();
    let mut off = ct.u;
    // Search a mutation of U that is not a curve point.
    let mut n = 0u8;
    loop {
        off[95] = n;
        if decode_g2(&off).is_err() && off != id2 {
            break;
        }
        n = n.wrapping_add(1);
    }
    for bad in [id2, ns, off] {
        let c = resign(bad);
        assert!(matches!(c.verify(&s.ad), Err(IbeError::InvalidPoint(_))));
        assert!(verify_bid_ct(&c.to_bytes(), &s.ad).is_err());
    }
}

#[test]
fn bid_weak_one_time_key_is_rejected() {
    let s = setup();
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, b"x", &mut rng(16)).unwrap();
    let mut c = ct.clone();
    // The identity point of Ed25519 is a small-order key. Strict verification refuses it.
    c.vk = [0u8; 32];
    c.vk[0] = 1;
    c.sig = [0u8; 64];
    c.sig[0] = 1;
    assert!(c.verify(&s.ad).is_err());
}

#[test]
fn bid_copy_under_another_context_does_not_decrypt() {
    let s = setup();
    let msg = b"secret bid";
    let ct = encrypt_bid(&s.d.mpk, &s.id, &s.ad, msg, &mut rng(17)).unwrap();
    assert_eq!(decrypt_bid(&s.dk, &s.id, &s.ad, &ct).unwrap(), msg);

    // Another bidder, auction, epoch or timelock in AD: the original signature no longer holds.
    for ad in [
        bid_ad(2, 5, 3, 11),
        bid_ad(1, 6, 3, 11),
        bid_ad(1, 5, 4, 11),
        bid_ad(1, 5, 3, 12),
    ] {
        assert!(ct.verify(&ad).is_err());
        assert!(decrypt_bid(&s.dk, &s.id, &ad, &ct).is_err());
    }

    // A copier who re-signs under a fresh key and the new AD passes submission,
    // but the body does not decrypt: the key binds vk and AD.
    let copier_ad = bid_ad(2, 5, 3, 11);
    let sk = SigningKey::from_bytes(&[9u8; 32]);
    let mut copy = ct.clone();
    copy.vk = sk.verifying_key().to_bytes();
    copy.sig = sk.sign(&copy.signed_message(&copier_ad)).to_bytes();
    assert!(copy.verify(&copier_ad).is_ok());
    assert!(decrypt_bid(&s.dk, &s.id, &copier_ad, &copy).is_err());
    // Same with only the vk changed and the old AD.
    let mut copy2 = ct.clone();
    copy2.vk = sk.verifying_key().to_bytes();
    copy2.sig = sk.sign(&copy2.signed_message(&s.ad)).to_bytes();
    assert!(decrypt_bid(&s.dk, &s.id, &s.ad, &copy2).is_err());

    // The key of another timelock or epoch (another identity) does not open it.
    for (epoch, tl) in [(4u64, 11u64), (3, 12)] {
        let other = compute_identity(7, epoch, tl, 1_700_000_000_000_000);
        let other_dk = s.d.reconstruct(&[0, 1], &other).unwrap();
        assert!(decrypt_bid(&other_dk, &s.id, &s.ad, &ct).is_err());
        assert!(decrypt_bid(&other_dk, &other, &s.ad, &ct).is_err());
    }
    // A key from another master secret does not open it.
    let d2 = deal(&[2, 3, 2], 5, 78);
    let foreign = d2.reconstruct(&[0, 1], &s.id).unwrap();
    assert!(decrypt_bid(&foreign, &s.id, &s.ad, &ct).is_err());
}

#[test]
fn one_undecryptable_bid_does_not_stop_the_others() {
    let s = setup();
    let msgs: [&[u8]; 3] = [b"first", b"second", b"third"];
    let mut cts: Vec<BidCiphertext> = msgs
        .iter()
        .enumerate()
        .map(|(i, m)| encrypt_bid(&s.d.mpk, &s.id, &s.ad, m, &mut rng(20 + i as u64)).unwrap())
        .collect();
    cts[1].c[0] ^= 1;
    let results: Vec<_> = cts
        .iter()
        .map(|c| decrypt_bid(&s.dk, &s.id, &s.ad, c))
        .collect();
    assert_eq!(results[0].as_ref().unwrap(), msgs[0]);
    assert!(results[1].is_err());
    assert_eq!(results[2].as_ref().unwrap(), msgs[2]);
}

#[test]
fn full_flow_chunky_shaped_shares_to_bid_open() {
    // Dealing, bid, verify each share, reconstruct, key check, open: the handler's order.
    let d = deal(&[3, 3, 3, 1], 7, 31);
    let id = identity();
    let ad = bid_ad(3, 1, 3, 11);
    let ct = encrypt_bid(&d.mpk, &id, &ad, b"flow", &mut rng(32)).unwrap();
    verify_bid_ct(&ct.to_bytes(), &ad).unwrap();

    let who = [0usize, 1, 3];
    let nested = d.dk_shares(&who, &id);
    for (k, &v) in who.iter().enumerate() {
        for (j, bytes) in nested[k].iter().enumerate() {
            let share = decode_g1(bytes).unwrap();
            assert!(verify_dk_share(&share, &id, &d.vks[d.start(v) + j]));
        }
    }
    let dk = d.reconstruct(&who, &id).unwrap();
    assert!(verify_decryption_key(&dk, &id, &d.mpk));
    assert_eq!(decrypt_bid(&dk, &id, &ad, &ct).unwrap(), b"flow");
}

// ---------------------------------------------------------------------------------------------
// Fixture generator for `aptos-move/framework/aptos-framework/tests/ibe_native_test.move`
// ---------------------------------------------------------------------------------------------

/// Prints `NAME=hex` lines. Run with
/// `cargo test -p aptos-dkg --lib ibe::tests::generate_move_fixtures -- --ignored --nocapture`.
/// Deterministic: fixed seeds, so the output is reproducible.
#[test]
#[ignore]
fn generate_move_fixtures() {
    let d = deal(&[2, 3, 2], 5, 4242);
    let id = compute_identity(4, 2, 1, 1_000_000);
    let ad = bid_ad(1, 5, 2, 1);
    let who = [0usize, 1];
    let nested = d.dk_shares(&who, &id);
    let dk = d.reconstruct(&who, &id).unwrap();
    assert!(verify_decryption_key(&dk, &id, &d.mpk));
    let ct = encrypt_bid(&d.mpk, &id, &ad, b"hello", &mut rng(4243)).unwrap();
    let wire = ct.to_bytes();
    assert_eq!(wire.len(), 230);
    assert_eq!(wire[0], 32);
    assert_eq!(wire[33], 96);
    assert_eq!(wire[130], 12);
    assert_eq!(wire[143], 21);
    assert_eq!(wire[165], 64);

    // A correctly signed ciphertext whose U is on the curve but outside the subgroup.
    let sk = SigningKey::from_bytes(&[5u8; 32]);
    let mut bad = ct.clone();
    bad.vk = sk.verifying_key().to_bytes();
    bad.u = raw_g2(&non_subgroup_g2()).try_into().unwrap();
    bad.sig = sk.sign(&bad.signed_message(&ad)).to_bytes();
    let ns_share = raw_g1(&non_subgroup_g1());
    let other_id = compute_identity(4, 2, 2, 1_000_000);
    let other_dk = d.reconstruct(&who, &other_id).unwrap();

    println!("IDENTITY={}", hex::encode(id));
    println!("OTHER_IDENTITY={}", hex::encode(other_id));
    println!("MPK={}", hex::encode(encode_g2(&d.mpk)));
    println!("DK={}", hex::encode(encode_g1(&dk)));
    println!("OTHER_DK={}", hex::encode(encode_g1(&other_dk)));
    for (v, shares) in nested.iter().enumerate() {
        for (j, s) in shares.iter().enumerate() {
            println!("SHARE_{v}_{j}={}", hex::encode(s));
        }
    }
    println!("AD={}", hex::encode(&ad));
    println!("CT={}", hex::encode(&wire));
    println!("CT_NON_SUBGROUP_U={}", hex::encode(bad.to_bytes()));
    println!("NON_SUBGROUP_G1={}", hex::encode(ns_share));
}

// ---------------------------------------------------------------------------------------------
// Per-validator share check (`verify_player_shares`)
// ---------------------------------------------------------------------------------------------

fn player_vks(d: &Dealing, validator: usize) -> Vec<Vec<u8>> {
    (0..d.weights[validator] as usize)
        .map(|j| encode_g2(&d.vks[d.start(validator) + j]).to_vec())
        .collect()
}

#[test]
fn player_shares_valid_set_verifies_and_rebuilds() {
    let d = deal(&[2, 3, 1, 2], 5, 21);
    let id = identity();
    let shares = d.dk_shares(&[0, 1, 2, 3], &id);
    for v in 0..4 {
        assert_eq!(
            verify_player_shares(&id, &shares[v], &player_vks(&d, v)),
            Ok(())
        );
    }
    // The verified shares of a threshold set rebuild a key that passes the key check.
    let rebuilt = d.reconstruct(&[0, 1, 3], &id).unwrap();
    assert!(verify_decryption_key(&rebuilt, &id, &d.mpk));
}

#[test]
fn player_shares_negative_cases() {
    let d = deal(&[2, 2, 2, 2], 5, 22);
    let id = identity();
    let shares = d.dk_shares(&[0, 1], &id);
    let vks0 = player_vks(&d, 0);

    // Wrong share: one point shifted by the generator.
    let good0 = decode_g1(&shares[0][0]).unwrap();
    let bad = (G1Projective::from(good0) + G1Projective::from(G1Affine::generator())).into_affine();
    let mut wrong = shares[0].clone();
    wrong[1] = encode_g1(&bad).to_vec();
    assert_eq!(
        verify_player_shares(&id, &wrong, &vks0),
        Err(ShareCheckFailure::PairingMismatch { index: 1 })
    );
    // Shares of another validator under this validator's keys.
    assert_eq!(
        verify_player_shares(&id, &shares[1], &vks0),
        Err(ShareCheckFailure::PairingMismatch { index: 0 })
    );
    // Swapped players of the same validator.
    let swapped = vec![shares[0][1].clone(), shares[0][0].clone()];
    assert_eq!(
        verify_player_shares(&id, &swapped, &vks0),
        Err(ShareCheckFailure::PairingMismatch { index: 0 })
    );
    // Share for another identity (timelock id), and for another epoch.
    for other in [
        compute_identity(7, 3, 12, 1_700_000_000_000_000),
        compute_identity(7, 4, 11, 1_700_000_000_000_000),
    ] {
        let other_shares = d.dk_shares(&[0], &other);
        assert_eq!(
            verify_player_shares(&id, &other_shares[0], &vks0),
            Err(ShareCheckFailure::PairingMismatch { index: 0 })
        );
    }
    // Share for another DKG secret.
    let d2 = deal(&[2, 2, 2, 2], 5, 23);
    let foreign = d2.dk_shares(&[0], &id);
    assert_eq!(
        verify_player_shares(&id, &foreign[0], &vks0),
        Err(ShareCheckFailure::PairingMismatch { index: 0 })
    );
    // Non-subgroup point and the identity point are refused by validated decoding.
    let mut ns = shares[0].clone();
    ns[0] = raw_g1(&non_subgroup_g1());
    assert!(matches!(
        verify_player_shares(&id, &ns, &vks0),
        Err(ShareCheckFailure::InvalidShare { index: 0, .. })
    ));
    let mut ident = vec![0u8; G1_COMPRESSED_LEN];
    ident[0] = 0xc0;
    let mut zero = shares[0].clone();
    zero[1] = ident;
    assert!(matches!(
        verify_player_shares(&id, &zero, &vks0),
        Err(ShareCheckFailure::InvalidShare { index: 1, .. })
    ));
    // A bad verification key is refused too.
    let mut bad_vks = vks0.clone();
    bad_vks[0] = raw_g2(&non_subgroup_g2());
    assert!(matches!(
        verify_player_shares(&id, &shares[0], &bad_vks),
        Err(ShareCheckFailure::InvalidVerificationKey { index: 0, .. })
    ));
    // Count mismatch and empty input.
    assert_eq!(
        verify_player_shares(&id, &shares[0][..1], &vks0),
        Err(ShareCheckFailure::CountMismatch { shares: 1, keys: 2 })
    );
    assert!(verify_player_shares(&id, &[], &[]).is_err());
}

/// Measures the cost of the per-share check for validator weights in the range the chunky DKG rounding
/// can produce. Run with:
/// `cargo test -p aptos-dkg --release --lib ibe::tests::measure_share_check_cost -- --ignored --nocapture`
#[test]
#[ignore]
fn measure_share_check_cost() {
    let id = identity();
    for weight in [1u64, 8, 32, 128] {
        let d = deal(&[weight, 1], (weight as usize + 2) / 2 + 1, 99);
        let shares = d.dk_shares(&[0], &id).remove(0);
        let vks = player_vks(&d, 0);
        let reps = 5;
        let start = std::time::Instant::now();
        for _ in 0..reps {
            assert_eq!(verify_player_shares(&id, &shares, &vks), Ok(()));
        }
        let per_author = start.elapsed() / reps;
        println!(
            "share check: weight {:>4}: {:>10.2?} per author, {:>8.2?} per share",
            weight,
            per_author,
            per_author / weight as u32
        );
    }
}

// ---- V0 versus interpolated verification keys (timelock DKG result acceptance) ----

#[test]
fn mpk_matches_interpolated_verification_keys() {
    let d = deal(&[3, 2, 1, 4], 5, 21);
    assert_eq!(
        check_mpk_matches_verification_keys(&d.mpk, &d.vks, d.threshold),
        Ok(())
    );
    // Interpolating any threshold set gives mpk.
    let ids = [9usize, 0, 4, 7, 2];
    let pts: Vec<G2Affine> = ids.iter().map(|&i| d.vks[i]).collect();
    assert_eq!(
        interpolate_g2_at_zero(&ids, &pts, d.threshold, d.total),
        Ok(d.mpk)
    );
}

#[test]
fn mismatched_mpk_is_rejected() {
    let d = deal(&[3, 2, 1, 4], 5, 21);
    // An unrelated master public key, as in a subtranscript whose V0 belongs to another dealing.
    let other = deal(&[3, 2, 1, 4], 5, 22);
    assert_eq!(
        check_mpk_matches_verification_keys(&other.mpk, &d.vks, d.threshold),
        Err(IbeError::MpkVerificationKeyMismatch)
    );
    // One corrupted verification key breaks at least the set that contains it. Player 0 is in
    // the first set, the last player is in the last set.
    for bad in [0usize, d.total - 1] {
        let mut vks = d.vks.clone();
        vks[bad] = master_public_key(&Fr::from(12345u64));
        assert_eq!(
            check_mpk_matches_verification_keys(&d.mpk, &vks, d.threshold),
            Err(IbeError::MpkVerificationKeyMismatch),
            "corrupted key {bad}"
        );
    }
}

#[test]
fn interpolation_rejects_bad_inputs() {
    let d = deal(&[2, 2], 3, 5);
    assert!(interpolate_g2_at_zero(&[0, 0, 1], &d.vks[..3], 3, 4).is_err());
    assert!(interpolate_g2_at_zero(&[0, 1, 9], &d.vks[..3], 3, 4).is_err());
    assert!(interpolate_g2_at_zero(&[0, 1], &d.vks[..2], 3, 4).is_err());
    assert!(check_mpk_matches_verification_keys(&d.mpk, &d.vks, 0).is_err());
    assert!(check_mpk_matches_verification_keys(&d.mpk, &d.vks, 5).is_err());
}
