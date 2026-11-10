#[test_only]
/// Rules of `ethereum_headers` and `ethereum_header_config`: only the `@aptos_framework` signer writes, a
/// certified header never changes, and the views read what was stored.
module aptos_framework::ethereum_headers_tests {
    use std::option;
    use aptos_framework::account;
    use aptos_framework::ethereum_header_config;
    use aptos_framework::ethereum_headers::{Self, HeaderCertified};
    use aptos_framework::event;

    const CHAIN: u64 = 31337;

    fun framework(): signer {
        account::create_signer_for_test(@aptos_framework)
    }

    fun hash(b: u8): vector<u8> {
        let v = vector[];
        let i = 0;
        while (i < 32) {
            v.push_back(b);
            i += 1;
        };
        v
    }

    #[test]
    fun store_then_read_views() {
        let fx = framework();
        assert!(ethereum_headers::latest_block_number() == option::none(), 0);
        assert!(!ethereum_headers::is_certified(10), 1);

        ethereum_headers::store_certified_header(&fx, CHAIN, 10, hash(1), hash(2));
        assert!(ethereum_headers::is_certified(10), 2);
        assert!(ethereum_headers::get_block_hash(10) == hash(1), 3);
        assert!(ethereum_headers::get_state_root(10) == hash(2), 4);
        assert!(ethereum_headers::latest_block_number() == option::some(10), 5);

        // An older block is stored too and does not lower the latest block number.
        ethereum_headers::store_certified_header(&fx, CHAIN, 5, hash(3), hash(4));
        assert!(ethereum_headers::get_state_root(5) == hash(4), 6);
        assert!(ethereum_headers::latest_block_number() == option::some(10), 7);
        assert!(event::emitted_events<HeaderCertified>().length() == 2, 8);
    }

    #[test]
    #[expected_failure(abort_code = 0x80002, location = aptos_framework::ethereum_headers)]
    fun same_header_twice_aborts() {
        let fx = framework();
        ethereum_headers::store_certified_header(&fx, CHAIN, 10, hash(1), hash(2));
        ethereum_headers::store_certified_header(&fx, CHAIN, 10, hash(1), hash(2));
    }

    #[test]
    #[expected_failure(abort_code = 0x30003, location = aptos_framework::ethereum_headers)]
    fun conflicting_root_aborts() {
        let fx = framework();
        ethereum_headers::store_certified_header(&fx, CHAIN, 10, hash(1), hash(2));
        ethereum_headers::store_certified_header(&fx, CHAIN, 10, hash(1), hash(9));
    }

    #[test(other = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun non_framework_signer_aborts(other: &signer) {
        ethereum_headers::store_certified_header(other, CHAIN, 10, hash(1), hash(2));
    }

    #[test]
    #[expected_failure(abort_code = 0x10001, location = aptos_framework::ethereum_headers)]
    fun short_root_aborts() {
        ethereum_headers::store_certified_header(&framework(), CHAIN, 10, hash(1), vector[1u8]);
    }

    #[test]
    #[expected_failure(abort_code = 0x60004, location = aptos_framework::ethereum_headers)]
    fun unknown_block_aborts() {
        ethereum_headers::set_header_for_test(10, hash(1), hash(2));
        ethereum_headers::get_state_root(11);
    }

    #[test]
    fun test_only_setter_stores_header() {
        ethereum_headers::set_header_for_test(7, hash(5), hash(6));
        assert!(ethereum_headers::get_block_hash(7) == hash(5), 0);
        assert!(ethereum_headers::get_state_root(7) == hash(6), 1);
        assert!(ethereum_headers::latest_block_number() == option::some(7), 2);
    }

    #[test]
    fun config_set_switch_and_disable() {
        let fx = framework();
        assert!(ethereum_header_config::eth_chain_id() == option::none(), 0);
        ethereum_header_config::set(&fx, CHAIN);
        assert!(ethereum_header_config::eth_chain_id() == option::some(CHAIN), 1);
        ethereum_header_config::set(&fx, 1);
        assert!(ethereum_header_config::eth_chain_id() == option::some(1), 2);
        ethereum_header_config::disable(&fx);
        assert!(ethereum_header_config::eth_chain_id() == option::none(), 3);
    }

    #[test(other = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun config_non_framework_signer_aborts(other: &signer) {
        ethereum_header_config::set(other, CHAIN);
    }
}
