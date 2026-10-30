module 0xbeef::ibe_gas {
    use std::vector;
    use aptos_std::bls12381_algebra::G1;
    use aptos_std::ibe;

    public entry fun noop() {}

    public entry fun bid_ct(ct: vector<u8>, ad: vector<u8>) {
        assert!(ibe::verify_bid_ct(ct, ad), 1);
    }

    /// Does not assert the result: only the metering is under test.
    public entry fun dk_check(dk: vector<u8>, identity: vector<u8>, mpk: vector<u8>) {
        ibe::verify_decryption_key(dk, identity, mpk);
    }

    /// `flat` is a flat list of 48-byte shares, `counts[i]` of them for validator i.
    public entry fun reconstruct(
        flat: vector<vector<u8>>, counts: vector<u64>, weights: vector<u64>, threshold: u64, total_weight: u64,
        identity: vector<u8>
    ) {
        let n = counts.length();
        let indices = vector::empty<u64>();
        let shares = vector::empty<vector<vector<u8>>>();
        let k = 0;
        for (i in 0..n) {
            indices.push_back(i);
            let v = vector::empty<vector<u8>>();
            for (_j in 0..*counts.borrow(i)) {
                v.push_back(*flat.borrow(k));
                k += 1;
            };
            shares.push_back(v);
        };
        ibe::reconstruct_ibe_dk<G1>(indices, shares, weights, threshold, total_weight, identity);
    }
}
