#[test_only]
/// Move side of the V2 timelock golden vectors.
///
/// The fixtures are generated from a seeded real chunky DKG dealing (see
/// `ibe_golden_vector_fixtures.move` for the regeneration command). These tests check the checked-in
/// bytes with the framework's own code: the identity layout, `H1` through `crypto_algebra`, every
/// per validator share through a pairing check, the final key check native, and the bid submission
/// native. The Rust golden tests cover the weighted reconstruction.
module aptos_framework::ibe_golden_vector_tests {
    use std::bcs;
    use std::features;
    use std::vector;
    use aptos_std::bls12381_algebra::{G1, G2, Gt, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo};
    use aptos_std::crypto_algebra;
    use aptos_std::ibe;
    use aptos_framework::ibe_config;
    use aptos_framework::ibe_golden_vector_fixtures as fx;

    const H1_DST: vector<u8> = b"ATOMICA_TIMELOCK_IBE_V2_H1";

    fun enable_algebra(fw: &signer) {
        features::change_feature_flags_for_testing(
            fw, vector[features::get_bls12_381_strutures_feature()], vector[]
        );
    }

    /// `e(H1(id), vk) = e(share, g2)` for every virtual player, with validated decoding.
    fun shares_verify(identity: vector<u8>, shares: vector<vector<u8>>, keys: vector<vector<u8>>): bool {
        if (shares.length() == 0 || shares.length() != keys.length()) return false;
        let q = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&H1_DST, &identity);
        let g2 = crypto_algebra::one<G2>();
        let i = 0;
        while (i < shares.length()) {
            let share = crypto_algebra::deserialize<G1, FormatG1Compr>(shares.borrow(i));
            let vk = crypto_algebra::deserialize<G2, FormatG2Compr>(keys.borrow(i));
            if (share.is_none() || vk.is_none()) return false;
            let share = share.extract();
            // The identity element is not a valid share.
            if (crypto_algebra::eq(&share, &crypto_algebra::zero<G1>())) return false;
            let vk = vk.extract();
            let lhs = crypto_algebra::pairing<G1, G2, Gt>(&q, &vk);
            let rhs = crypto_algebra::pairing<G1, G2, Gt>(&share, &g2);
            if (!crypto_algebra::eq(&lhs, &rhs)) return false;
            i += 1;
        };
        true
    }

    // ------------------------------------------------------------------ identity and H1

    #[test(fw = @aptos_framework)]
    fun identity_and_h1_vectors(fw: &signer) {
        enable_algebra(fw);
        let chains = fx::identity_vector_chains();
        let epochs = fx::identity_vector_epochs();
        let ids = fx::identity_vector_timelock_ids();
        let deadlines = fx::identity_vector_deadlines();
        let identities = fx::identity_vector_identities();
        let h1s = fx::identity_vector_h1s();
        assert!(chains.length() == 7, 0);
        let i = 0;
        while (i < chains.length()) {
            let identity = ibe_config::compute_identity(
                *chains.borrow(i), *epochs.borrow(i), *ids.borrow(i), *deadlines.borrow(i)
            );
            assert!(identity == *identities.borrow(i), 100 + i);
            let q = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&H1_DST, &identity);
            assert!(crypto_algebra::serialize<G1, FormatG1Compr>(&q) == *h1s.borrow(i), 200 + i);
            i += 1;
        };
    }

    #[test(fw = @aptos_framework)]
    fun dealing_identity_matches(fw: &signer) {
        enable_algebra(fw);
        let identity = ibe_config::compute_identity(
            fx::chain_id(), fx::epoch(), fx::timelock_id(), fx::deadline_us()
        );
        assert!(identity == fx::identity(), 0);
        let q = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&H1_DST, &identity);
        assert!(crypto_algebra::serialize<G1, FormatG1Compr>(&q) == fx::h1(), 1);
        // The other identity differs.
        assert!(fx::other_identity() != fx::identity(), 2);
    }

    // ------------------------------------------------------------------ keys from the chunky dealing

    #[test(fw = @aptos_framework)]
    fun keys_decode_and_have_the_documented_sizes(fw: &signer) {
        enable_algebra(fw);
        assert!(fx::mpk().length() == 96, 0);
        assert!(fx::dk().length() == 48, 1);
        assert!(crypto_algebra::deserialize<G2, FormatG2Compr>(&fx::mpk()).is_some(), 2);
        let vks = fx::verification_keys();
        let weights = fx::validator_weights();
        assert!(vks.length() == weights.length(), 3);
        let total = 0;
        let v = 0;
        while (v < vks.length()) {
            let row = vks.borrow(v);
            assert!(row.length() == *weights.borrow(v), 4);
            row.for_each_ref(|k| {
                assert!(k.length() == 96, 5);
                assert!(crypto_algebra::deserialize<G2, FormatG2Compr>(k).is_some(), 6);
            });
            total += row.length();
            v += 1;
        };
        assert!(total == fx::total_weight(), 7);
    }

    /// `mpk` is the interpolation in G2 of the verification keys of any threshold set: here, the
    /// check `e(dk, g2) = e(H1(id), mpk)` that follows from it.
    #[test(fw = @aptos_framework)]
    fun key_check_pairing_matches_native(fw: &signer) {
        enable_algebra(fw);
        let q = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&H1_DST, &fx::identity());
        let dk = crypto_algebra::deserialize<G1, FormatG1Compr>(&fx::dk()).extract();
        let mpk = crypto_algebra::deserialize<G2, FormatG2Compr>(&fx::mpk()).extract();
        let lhs = crypto_algebra::pairing<G1, G2, Gt>(&dk, &crypto_algebra::one<G2>());
        let rhs = crypto_algebra::pairing<G1, G2, Gt>(&q, &mpk);
        assert!(crypto_algebra::eq(&lhs, &rhs), 0);
        assert!(ibe::verify_decryption_key(fx::dk(), fx::identity(), fx::mpk()), 1);
    }

    // ------------------------------------------------------------------ share verification

    #[test(fw = @aptos_framework)]
    fun every_honest_share_verifies(fw: &signer) {
        enable_algebra(fw);
        let shares = fx::dk_shares();
        let vks = fx::verification_keys();
        let v = 0;
        while (v < shares.length()) {
            assert!(shares_verify(fx::identity(), *shares.borrow(v), *vks.borrow(v)), v);
            v += 1;
        };
        // Shares of the other identity do not verify for this identity, and verify for their own.
        let other = fx::other_dk_shares();
        assert!(!shares_verify(fx::identity(), *other.borrow(0), *vks.borrow(0)), 10);
        assert!(shares_verify(fx::other_identity(), *other.borrow(0), *vks.borrow(0)), 11);
    }

    #[test(fw = @aptos_framework)]
    fun share_cases_give_the_recorded_verdicts(fw: &signer) {
        enable_algebra(fw);
        let expect_ok = fx::share_case_expect_ok();
        let shares = fx::share_case_shares();
        let keys = fx::share_case_keys();
        assert!(expect_ok.length() == shares.length() && shares.length() == keys.length(), 0);
        assert!(expect_ok.length() == 13, 1);
        let i = 0;
        while (i < expect_ok.length()) {
            let got = shares_verify(fx::identity(), *shares.borrow(i), *keys.borrow(i));
            assert!(got == *expect_ok.borrow(i), 100 + i);
            i += 1;
        };
    }

    // ------------------------------------------------------------------ final key check

    #[test]
    fun key_check_cases() {
        let dks = fx::key_check_dks();
        let ids = fx::key_check_identities();
        let mpks = fx::key_check_mpks();
        let expect = fx::key_check_expect();
        let i = 0;
        while (i < dks.length()) {
            let got = ibe::verify_decryption_key(*dks.borrow(i), *ids.borrow(i), *mpks.borrow(i));
            assert!(got == *expect.borrow(i), 100 + i);
            i += 1;
        };
        assert!(*expect.borrow(0) && !*expect.borrow(1), 1);
    }

    // ------------------------------------------------------------------ bids

    #[test]
    fun bid_cases_give_the_recorded_verdicts() {
        let cts = fx::bid_case_cts();
        let ads = fx::bid_case_ads();
        let verifies = fx::bid_case_verifies();
        assert!(cts.length() == 15, 0);
        let i = 0;
        while (i < cts.length()) {
            let got = ibe::verify_bid_ct(*cts.borrow(i), *ads.borrow(i));
            assert!(got == *verifies.borrow(i), 100 + i);
            i += 1;
        };
        // The honest bid passes and the submitted associated data is the contract's.
        assert!(*verifies.borrow(0), 1);
        assert!(*ads.borrow(0) == fx::bid_ad(), 2);
    }

    #[test]
    fun bid_that_passes_submission_but_does_not_open() {
        // Case 11 is the body re-signed under a new one time key. It passes the signature check.
        // Its key derivation binds the new key, so the native does not open it.
        let verifies = fx::bid_case_verifies();
        let opens = fx::bid_case_opens();
        assert!(*verifies.borrow(11), 0);
        assert!(!*opens.borrow(11), 1);
        let got = ibe::decrypt_bid(fx::dk(), fx::identity(), *fx::bid_case_ads().borrow(11), *fx::bid_case_cts().borrow(11));
        assert!(got.is_none(), 4);
        let n = opens.length();
        let count = 0;
        let i = 0;
        while (i < n) {
            if (*opens.borrow(i)) count += 1;
            i += 1;
        };
        assert!(count == 1 && *opens.borrow(0), 2);
        assert!(!*fx::bid_case_opens_with_other_dk().borrow(0), 3);
    }

    #[test]
    fun decrypt_bid_agrees_with_bid_case_opens_for_every_golden_case() {
        let cts = fx::bid_case_cts();
        let ads = fx::bid_case_ads();
        let opens = fx::bid_case_opens();
        let with_other = fx::bid_case_opens_with_other_dk();
        assert!(cts.length() == 15, 0);
        let i = 0;
        while (i < cts.length()) {
            let dk = if (*with_other.borrow(i)) fx::other_dk() else fx::dk();
            let got = ibe::decrypt_bid(dk, fx::identity(), *ads.borrow(i), *cts.borrow(i));
            assert!(got.is_some() == *opens.borrow(i), 100 + i);
            if (got.is_some()) {
                assert!(got.destroy_some() == fx::bid_plaintext(), 200 + i);
            };
            i += 1;
        };
        assert!(*opens.borrow(0), 1);
    }

    /// Reads a little-endian u64 from exactly 8 bytes.
    fun le_u64(b: vector<u8>): u64 {
        assert!(b.length() == 8, 0);
        let v = 0u64;
        let i = 8;
        while (i > 0) {
            i -= 1;
            v = (v << 8) | (*b.borrow(i) as u64);
        };
        v
    }

    #[test]
    fun price_bids_decrypt_to_their_little_endian_price_under_their_own_bidder_only() {
        let bidders = fx::price_bid_bidders();
        let prices = fx::price_bid_prices();
        let cts = fx::price_bid_cts();
        let ads = fx::price_bid_ads();
        let n = cts.length();
        assert!(n == 3 && bidders.length() == n && prices.length() == n && ads.length() == n, 0);
        let i = 0;
        while (i < n) {
            let ct = *cts.borrow(i);
            // The associated data names the bidder: BCS of tag (24), chain id (1), epoch, timelock
            // id and auction id (8 each), then the 32-byte bidder address.
            assert!(vector::slice(ads.borrow(i), 49, 81) == bcs::to_bytes(bidders.borrow(i)), 600 + i);
            assert!(ibe::verify_bid_ct(ct, *ads.borrow(i)), 100 + i);
            let got = ibe::decrypt_bid(fx::dk(), fx::identity(), *ads.borrow(i), ct);
            assert!(got.is_some(), 200 + i);
            assert!(le_u64(got.destroy_some()) == *prices.borrow(i), 300 + i);
            // Under another bidder's associated data, or with the other key, it does not open.
            let j = 0;
            while (j < n) {
                if (j != i) {
                    assert!(ibe::decrypt_bid(fx::dk(), fx::identity(), *ads.borrow(j), ct).is_none(), 400 + i * 10 + j);
                };
                j += 1;
            };
            assert!(ibe::decrypt_bid(fx::other_dk(), fx::identity(), *ads.borrow(i), ct).is_none(), 500 + i);
            i += 1;
        };
        // The bad-length bid opens, but its plaintext is not an 8-byte price.
        let got = ibe::decrypt_bid(fx::dk(), fx::identity(), fx::bad_length_bid_ad(), fx::bad_length_bid_ct());
        assert!(got.is_some(), 1);
        assert!(got.destroy_some().length() != 8, 2);
    }
}
