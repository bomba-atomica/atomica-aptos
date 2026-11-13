module 0xbeef::ibe_gas {
    use aptos_std::ibe;

    public entry fun noop() {}

    public entry fun bid_ct(ct: vector<u8>, ad: vector<u8>) {
        assert!(ibe::verify_bid_ct(ct, ad), 1);
    }

    /// Does not assert the result: only the metering is under test.
    public entry fun bid_ct_any(ct: vector<u8>, ad: vector<u8>) {
        ibe::verify_bid_ct(ct, ad);
    }

    /// Does not assert the result: only the metering is under test.
    public entry fun decrypt(dk: vector<u8>, identity: vector<u8>, ad: vector<u8>, ct: vector<u8>) {
        ibe::decrypt_bid(dk, identity, ad, ct);
    }

    /// Decrypts every bid it is given, at any batch size, and asserts each one opens.
    public entry fun decrypt_many(
        dk: vector<u8>, identity: vector<u8>, ads: vector<vector<u8>>, cts: vector<vector<u8>>
    ) {
        let n = cts.length();
        assert!(ads.length() == n, 2);
        let i = 0;
        while (i < n) {
            let got = ibe::decrypt_bid(dk, identity, *ads.borrow(i), *cts.borrow(i));
            assert!(got.is_some(), 3);
            i += 1;
        };
    }

    /// Does not assert the result: only the metering is under test.
    public entry fun dk_check(dk: vector<u8>, identity: vector<u8>, mpk: vector<u8>) {
        ibe::verify_decryption_key(dk, identity, mpk);
    }
}
