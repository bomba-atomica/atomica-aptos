//! Option C prototype tests. See `atomica/docs/decisions/aptos-upgrade-option-c-spec.md`.

use proto_fptx_timelock::*;

// Weighted validators: 5 validators, total weight 10, threshold weight 6.
// Any set below weight 6 must not reconstruct.
const WEIGHTS: [usize; 5] = [1, 2, 2, 2, 3];
const THRESHOLD: usize = 6;

fn rng() -> ark_std::rand::rngs::StdRng {
    <ark_std::rand::rngs::StdRng as ark_std::rand::SeedableRng>::seed_from_u64(7)
}

fn setup(seed: u64, batch: usize, rounds: usize) -> TimelockSetup {
    TimelockSetup::for_testing(seed, batch, rounds, THRESHOLD, WEIGHTS.to_vec()).unwrap()
}

fn ad(bidder: u8) -> BidAd {
    BidAd::new(4, 9, 1001, 0, [bidder; 32], 1_700_000_000_000_000)
}

fn bid(i: u64) -> Bid {
    Bid {
        price: 100 + i,
        quantity: 1 + i % 7,
        salt: [i as u8; 16],
    }
}

fn seal(s: &TimelockSetup, n: usize) -> (Vec<Ciphertext>, Vec<Bid>) {
    let mut r = rng();
    let bids: Vec<Bid> = (0..n as u64).map(bid).collect();
    let cts = bids
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let ct = encrypt_bid(s, &mut r, b, &ad(i as u8)).unwrap();
            verify_bid(&ct, &ad(i as u8)).unwrap();
            ct
        })
        .collect();
    (cts, bids)
}

fn all_verified_shares(rv: &Reveal) -> Vec<Share> {
    (0..WEIGHTS.len())
        .map(|v| {
            let sh = rv.derive_share(v).unwrap();
            rv.verify_share(v, &sh).unwrap();
            sh
        })
        .collect()
}

#[test]
fn round_trip_encrypt_share_verify_combine_decrypt() {
    let s = setup(1, 16, 4);
    let (cts, bids) = seal(&s, 10);
    let rv = Reveal::new(&s, cts, 0).unwrap();
    let shares = all_verified_shares(&rv);
    let key = rv.combine(&shares).unwrap();
    assert_eq!(rv.decrypt_all(&key).unwrap(), bids);
}

#[test]
fn threshold_subset_reveals_and_any_subset_gives_same_key_result() {
    let s = setup(2, 16, 4);
    let (cts, bids) = seal(&s, 5);
    let rv = Reveal::new(&s, cts, 1).unwrap();
    let shares = all_verified_shares(&rv);
    // validators 4 (w3) + 1 (w2) + 0 (w1) = 6, exactly the threshold.
    let subset = vec![shares[4].clone(), shares[1].clone(), shares[0].clone()];
    let k = rv.combine(&subset).unwrap();
    assert_eq!(rv.decrypt_all(&k).unwrap(), bids);
    // A different qualifying subset: 3 + 2 + 2.
    let subset = vec![shares[4].clone(), shares[2].clone(), shares[3].clone()];
    let k = rv.combine(&subset).unwrap();
    assert_eq!(rv.decrypt_all(&k).unwrap(), bids);
}

#[test]
fn below_threshold_shares_cannot_reveal() {
    let s = setup(3, 16, 4);
    let (cts, _) = seal(&s, 4);
    let rv = Reveal::new(&s, cts, 0).unwrap();
    let shares = all_verified_shares(&rv);
    // weight 3 + 2 = 5 < 6
    let sub = vec![shares[4].clone(), shares[1].clone()];
    assert!(rv.combine(&sub).is_err(), "5 of 6 weight must not reveal");
    // single validator
    assert!(rv.combine(&shares[4..5]).is_err());
    assert!(rv.combine(&[]).is_err());
}

#[test]
fn duplicate_share_does_not_count_twice() {
    let s = setup(4, 16, 4);
    let (cts, _) = seal(&s, 3);
    let rv = Reveal::new(&s, cts, 0).unwrap();
    let shares = all_verified_shares(&rv);
    let dup = vec![shares[4].clone(), shares[4].clone()];
    assert!(matches!(
        rv.combine(&dup),
        Err(TimelockError::DuplicateShare(4))
    ));
}

#[test]
fn bit_flipped_ciphertext_is_rejected_at_every_byte() {
    let s = setup(5, 16, 4);
    let mut r = rng();
    let a = ad(1);
    let ct = encrypt_bid(&s, &mut r, &bid(1), &a).unwrap();
    verify_bid(&ct, &a).unwrap();
    let bytes = bcs::to_bytes(&ct).unwrap();
    let mut rejected = 0;
    for i in 0..bytes.len() {
        let mut t = bytes.clone();
        t[i] ^= 0x01;
        match bcs::from_bytes::<Ciphertext>(&t) {
            Err(_) => rejected += 1,
            Ok(c) if c == ct => panic!("flip at byte {i} left ciphertext unchanged"),
            Ok(c) => {
                assert!(
                    verify_bid(&c, &a).is_err(),
                    "flip at byte {i} passed verify_ct"
                );
                rejected += 1;
            },
        }
    }
    assert_eq!(rejected, bytes.len());
}

#[test]
fn ciphertext_rebound_to_other_bidder_auction_epoch_or_shard_is_rejected() {
    let s = setup(6, 16, 4);
    let mut r = rng();
    let a = ad(1);
    let ct = encrypt_bid(&s, &mut r, &bid(1), &a).unwrap();
    verify_bid(&ct, &a).unwrap();
    // copied by another bidder
    assert!(verify_bid(&ct, &ad(2)).is_err());
    let mut b = a.clone();
    b.auction_id += 1;
    assert!(verify_bid(&ct, &b).is_err());
    let mut b = a.clone();
    b.epoch += 1;
    assert!(verify_bid(&ct, &b).is_err());
    let mut b = a.clone();
    b.shard += 1;
    assert!(verify_bid(&ct, &b).is_err());
    let mut b = a.clone();
    b.deadline_us += 1;
    assert!(verify_bid(&ct, &b).is_err());
    let mut b = a.clone();
    b.tag = "atomica/timelock-bid/v2".into();
    assert!(verify_bid(&ct, &b).is_err());
}

#[test]
fn corrupted_or_misattributed_share_fails_verification_before_combination() {
    let s = setup(7, 16, 4);
    let (cts, _) = seal(&s, 4);
    let rv = Reveal::new(&s, cts, 0).unwrap();
    let shares = all_verified_shares(&rv);

    // 1. Wrong validator key: validator 1 submits validator 2's share.
    assert!(rv.verify_share(1, &shares[2]).is_err());
    // 2. Player id rewritten to impersonate validator 3 with validator 2's values.
    let mut forged = shares[2].clone();
    forged.0 = shares[3].0;
    assert!(rv.verify_share(3, &forged).is_err());
    // 3. A share for a different digest (other round and other batch).
    let (cts2, _) = seal(&s, 2);
    let rv2 = Reveal::new(&s, cts2, 1).unwrap();
    let other = rv2.derive_share(2).unwrap();
    assert!(rv.verify_share(2, &other).is_err());
    // 4. Byte corruption at every position: either not parseable or rejected.
    let bytes = bcs::to_bytes(&shares[2]).unwrap();
    for i in 0..bytes.len() {
        let mut t = bytes.clone();
        t[i] ^= 0x80;
        if let Ok(sh) = bcs::from_bytes::<Share>(&t) {
            assert_ne!(sh, shares[2], "flip at {i} unchanged");
            assert!(
                rv.verify_share(2, &sh).is_err(),
                "corrupt share at byte {i} verified"
            );
        }
    }
}

#[test]
fn unverified_bad_share_poisons_combination_and_final_check_catches_it() {
    // If a handler skipped verify_share, the final key check must still stop the reveal.
    let s = setup(8, 16, 4);
    let (cts, _) = seal(&s, 3);
    let rv = Reveal::new(&s, cts, 0).unwrap();
    let mut shares = all_verified_shares(&rv);
    let (cts2, _) = seal(&s, 2);
    let rv2 = Reveal::new(&s, cts2, 1).unwrap();
    // validator 4 sends a share for another digest; keep its player id.
    shares[4] = rv2.derive_share(4).unwrap();
    let subset = vec![shares[4].clone(), shares[1].clone(), shares[0].clone()];
    assert!(rv.combine(&subset).is_err());
    // Replace the bad share with an honest one from validator 2: reveal still works.
    shares[4] = rv.derive_share(4).unwrap();
    assert!(rv
        .combine(&[shares[4].clone(), shares[1].clone(), shares[2].clone()])
        .is_ok());
}

#[test]
fn wrong_key_rejection_separate_secrets_do_not_cross_decrypt() {
    // Key separation (spec section 6): timelock session B shares nothing with A.
    let a = setup(10, 16, 4);
    let b = setup(11, 16, 4);
    assert_ne!(
        bcs::to_bytes(&a.ek).unwrap(),
        bcs::to_bytes(&b.ek).unwrap(),
        "separate setups must give distinct encryption keys"
    );
    let (cts_a, bids_a) = seal(&a, 3);
    let rv_a = Reveal::new(&a, cts_a.clone(), 0).unwrap();
    let key_a = rv_a.combine(&all_verified_shares(&rv_a)).unwrap();
    assert_eq!(rv_a.decrypt_all(&key_a).unwrap(), bids_a);

    // Session B's honest shares over the same ciphertexts do not open them.
    let rv_b = Reveal::new(&b, cts_a, 0);
    if let Ok(rv_b) = rv_b {
        let shares_b = all_verified_shares(&rv_b);
        match rv_b.combine(&shares_b) {
            Err(_) => {},
            Ok(k) => assert!(rv_b.decrypt_all(&k).is_err(), "key B opened A's bids"),
        }
    }
    // Session A's key never verifies against B's epoch key.
    let (cts_b, _) = seal(&b, 2);
    let rv_b2 = Reveal::new(&b, cts_b, 0).unwrap();
    let key_b = rv_b2.combine(&all_verified_shares(&rv_b2)).unwrap();
    assert!(
        rv_b2.verify_key(&key_a).is_err(),
        "A's key verified under B"
    );
    let _ = key_b;
}

#[test]
fn full_256_bid_batch_reveals_and_257th_is_refused() {
    let s = setup(12, MAX_BATCH_SIZE, 2);
    let mut r = rng();
    let cts: Vec<Ciphertext> = (0..MAX_BATCH_SIZE as u64)
        .map(|i| {
            let a = BidAd::new(4, 9, 1001, 0, [(i % 251) as u8; 32], 5);
            let mut a = a;
            a.bidder[1] = (i / 251) as u8;
            let ct = encrypt_bid(&s, &mut r, &bid(i), &a).unwrap();
            verify_bid(&ct, &a).unwrap();
            admit(&[], &ct).unwrap();
            ct
        })
        .collect();
    // duplicate refused
    assert!(matches!(
        admit(&cts[..2], &cts[0]),
        Err(TimelockError::DuplicateBid)
    ));
    // 257th refused by the contract-side cap
    let extra = encrypt_bid(&s, &mut r, &bid(999), &ad(200)).unwrap();
    assert!(matches!(
        admit(&cts, &extra),
        Err(TimelockError::BatchFull(257))
    ));
    // and by the reveal itself
    let mut too_many = cts.clone();
    too_many.push(extra);
    assert!(Reveal::new(&s, too_many, 0).is_err());

    let rv = Reveal::new(&s, cts, 0).unwrap();
    let key = rv.combine(&all_verified_shares(&rv)).unwrap();
    let out = rv.decrypt_all(&key).unwrap();
    assert_eq!(out.len(), 256);
    for (i, b) in out.iter().enumerate() {
        assert_eq!(*b, bid(i as u64));
    }
}

#[test]
fn round_at_num_rounds_is_refused_and_rounds_bind_digests() {
    let s = setup(13, 16, 2);
    let (cts, _) = seal(&s, 3);
    assert!(
        Reveal::new(&s, cts.clone(), 2).is_err(),
        "round == num_rounds"
    );
    let r0 = Reveal::new(&s, cts.clone(), 0).unwrap();
    let r1 = Reveal::new(&s, cts.clone(), 1).unwrap();
    // The same batch under another round gives a share that does not verify.
    let sh1 = r1.derive_share(0).unwrap();
    assert!(r0.verify_share(0, &sh1).is_err());
    // A retry of one round recomputes the same digest, so shares carry over.
    let r0_again = Reveal::new(&s, cts, 0).unwrap();
    let sh = r0.derive_share(0).unwrap();
    r0_again.verify_share(0, &sh).unwrap();
}
