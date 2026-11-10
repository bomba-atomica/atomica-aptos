/// Which Ethereum network the validators attest headers of (fork only).
///
/// Design: `atomica/docs/decisions/eth-header-attestation.md`. While `EthereumHeaderConfig` exists,
/// validators with `eth_header_observer` enabled observe that network and certify its headers into
/// `ethereum_headers`. Validators read the config at the start of each epoch. The VM checks every certified
/// header against the current value.
///
/// Only the `@aptos_framework` signer (genesis or a governance proposal) can change it.
module aptos_framework::ethereum_header_config {
    use std::option::{Self, Option};
    use aptos_framework::system_addresses;

    struct EthereumHeaderConfig has key, drop {
        /// EIP-155 chain id of the attested Ethereum network.
        eth_chain_id: u64
    }

    /// Turns header attestation on for `eth_chain_id`, or switches it to that chain.
    public fun set(framework: &signer, eth_chain_id: u64) acquires EthereumHeaderConfig {
        system_addresses::assert_aptos_framework(framework);
        if (exists<EthereumHeaderConfig>(@aptos_framework)) {
            borrow_global_mut<EthereumHeaderConfig>(@aptos_framework).eth_chain_id = eth_chain_id;
        } else {
            move_to(framework, EthereumHeaderConfig { eth_chain_id });
        }
    }

    /// Turns header attestation off. Certified headers stay readable.
    public fun disable(framework: &signer) acquires EthereumHeaderConfig {
        system_addresses::assert_aptos_framework(framework);
        if (exists<EthereumHeaderConfig>(@aptos_framework)) {
            move_from<EthereumHeaderConfig>(@aptos_framework);
        }
    }

    #[view]
    /// The attested Ethereum chain id, or none while attestation is off.
    public fun eth_chain_id(): Option<u64> acquires EthereumHeaderConfig {
        if (exists<EthereumHeaderConfig>(@aptos_framework)) {
            option::some(borrow_global<EthereumHeaderConfig>(@aptos_framework).eth_chain_id)
        } else {
            option::none()
        }
    }
}
