/// Ethereum headers certified by the validators (fork only).
///
/// Design: `atomica/docs/decisions/eth-header-attestation.md`. Validators observe Ethereum headers through
/// their own RPCs. A header signed by validators that hold 2/3+ of the voting power arrives as the
/// `ObservedEthHeader` validator transaction. The VM checks the epoch, the chain id of
/// `ethereum_header_config` and the multi-signature, then calls `store_certified_header`.
///
/// `store_certified_header` is the only writer and no entry function writes headers. A certified header never
/// changes: a second certificate for a stored block aborts, whether it agrees or conflicts.
module aptos_framework::ethereum_headers {
    use std::error;
    use std::option::{Self, Option};
    use aptos_std::table::{Self, Table};
    use aptos_framework::event;
    use aptos_framework::system_addresses;

    #[test_only]
    friend aptos_framework::ethereum_headers_tests;

    /// A block hash and a state root are 32 bytes.
    const E_INVALID_LENGTH: u64 = 1;
    /// The block already has this certified header.
    const E_ALREADY_CERTIFIED: u64 = 2;
    /// The block already has a different certified header.
    const E_CONFLICTING_HEADER: u64 = 3;
    /// The block has no certified header.
    const E_HEADER_NOT_CERTIFIED: u64 = 4;

    const HASH_LENGTH: u64 = 32;

    struct CertifiedHeader has copy, drop, store {
        block_hash: vector<u8>,
        state_root: vector<u8>
    }

    /// Created by the first certified header.
    struct EthereumHeaders has key {
        headers: Table<u64, CertifiedHeader>,
        /// Highest certified block number.
        latest_block_number: Option<u64>
    }

    #[event]
    /// Emitted once per certified block.
    struct HeaderCertified has drop, store {
        eth_chain_id: u64,
        block_number: u64,
        block_hash: vector<u8>,
        state_root: vector<u8>
    }

    /// Stores a header the VM verified against a validator quorum certificate. Called by the VM only.
    public(friend) fun store_certified_header(
        framework: &signer,
        eth_chain_id: u64,
        block_number: u64,
        block_hash: vector<u8>,
        state_root: vector<u8>
    ) acquires EthereumHeaders {
        system_addresses::assert_aptos_framework(framework);
        assert!(
            block_hash.length() == HASH_LENGTH && state_root.length() == HASH_LENGTH,
            error::invalid_argument(E_INVALID_LENGTH)
        );
        if (!exists<EthereumHeaders>(@aptos_framework)) {
            move_to(
                framework,
                EthereumHeaders { headers: table::new(), latest_block_number: option::none() }
            );
        };
        let store = borrow_global_mut<EthereumHeaders>(@aptos_framework);
        let header = CertifiedHeader { block_hash, state_root };
        if (store.headers.contains(block_number)) {
            assert!(
                *store.headers.borrow(block_number) == header,
                error::invalid_state(E_CONFLICTING_HEADER)
            );
            abort error::already_exists(E_ALREADY_CERTIFIED)
        };
        store.headers.add(block_number, header);
        if (store.latest_block_number.is_none()
            || *store.latest_block_number.borrow() < block_number) {
            store.latest_block_number = option::some(block_number);
        };
        event::emit(HeaderCertified { eth_chain_id, block_number, block_hash, state_root });
    }

    #[view]
    /// Whether `block_number` has a certified header.
    public fun is_certified(block_number: u64): bool acquires EthereumHeaders {
        exists<EthereumHeaders>(@aptos_framework)
            && borrow_global<EthereumHeaders>(@aptos_framework).headers.contains(block_number)
    }

    #[view]
    /// The certified state root of `block_number`. Aborts with `E_HEADER_NOT_CERTIFIED` when there is none.
    public fun get_state_root(block_number: u64): vector<u8> acquires EthereumHeaders {
        borrow_header(block_number).state_root
    }

    #[view]
    /// The certified block hash of `block_number`. Aborts with `E_HEADER_NOT_CERTIFIED` when there is none.
    public fun get_block_hash(block_number: u64): vector<u8> acquires EthereumHeaders {
        borrow_header(block_number).block_hash
    }

    #[view]
    /// The highest certified block number, or none before the first certified header.
    public fun latest_block_number(): Option<u64> acquires EthereumHeaders {
        if (exists<EthereumHeaders>(@aptos_framework)) {
            borrow_global<EthereumHeaders>(@aptos_framework).latest_block_number
        } else {
            option::none()
        }
    }

    fun borrow_header(block_number: u64): CertifiedHeader acquires EthereumHeaders {
        assert!(is_certified(block_number), error::not_found(E_HEADER_NOT_CERTIFIED));
        *borrow_global<EthereumHeaders>(@aptos_framework).headers.borrow(block_number)
    }

    #[test_only]
    /// Stores a header without a certificate, for Move tests of packages that read headers.
    public fun set_header_for_test(
        block_number: u64, block_hash: vector<u8>, state_root: vector<u8>
    ) acquires EthereumHeaders {
        let framework = aptos_framework::account::create_signer_for_test(@aptos_framework);
        store_certified_header(&framework, 0, block_number, block_hash, state_root);
    }
}
