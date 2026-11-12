module 0xbeef::ibe_gas {
    use aptos_std::ibe;

    public entry fun noop() {}

    public entry fun bid_ct(ct: vector<u8>, ad: vector<u8>) {
        assert!(ibe::verify_bid_ct(ct, ad), 1);
    }

    // SCOUT(#227) ADD: entry `decrypt(dk, identity, ad, ct)` that calls ibe::decrypt_bid once, and entry `decrypt_many(dk, identity, ads, cts)` that decrypts 100 bids in one transaction.
    /// Does not assert the result: only the metering is under test.
    public entry fun dk_check(dk: vector<u8>, identity: vector<u8>, mpk: vector<u8>) {
        ibe::verify_decryption_key(dk, identity, mpk);
    }
}
