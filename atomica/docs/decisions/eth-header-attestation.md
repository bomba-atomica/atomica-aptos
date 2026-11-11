# Ethereum Header Attestation

Status: implemented (atomica issue 196). Canonical context: the product ADR `docs/decisions/trustless-v0-1-unwinding.md` and PRD line 224 (v0.1 Beta: trusted Aptos validators plus BLS signatures over Ethereum state).

## Purpose

Aptos needs Ethereum state roots to verify Ethereum lock proofs. Before this change only the `@atomica` admin could submit them (`eth_proof::submit_header`), so one key decided which locks existed. Now the validator set certifies Ethereum headers with a 2/3+ voting-power quorum, the same trust the chain already places in its validators. No relayer, user transaction or special account is involved, and no entry function writes headers.

## Flow

1. Each validator with `eth_header_observer.enabled` polls its own Ethereum JSON-RPC (`eth_chainId`, `eth_getBlockByNumber(block_tag)`). It skips the head when the RPC is on another chain than `EthereumHeaderConfig.eth_chain_id`, or when the block is at or below the latest certified block.
2. It builds an `EthHeaderObservation { epoch, eth_chain_id, block_number, block_hash, state_root }` and reliably broadcasts a `SignRequest` to all validators (itself included) over the validator network (`ProtocolId::EthHeaderConsensusRpcBcs`).
3. A peer signs with its consensus BLS key only when: the epoch and chain id match its own, its own RPC is on that chain, the block number is at or below its own head at its configured `block_tag`, and its own RPC returns the same block hash and state root for that number. Otherwise it answers with an error.
4. Signatures over the same observation that hold 2/3+ of the voting power aggregate into a `QuorumCertifiedEthHeader`. It goes into the validator transaction pool (topic `ETH_HEADER`) and a proposer includes it as `ValidatorTransaction::ObservedEthHeader` (BCS tag 5).
5. The VM handler (`aptos-vm/src/validator_txns/eth_header.rs`) checks: `EthereumHeaderConfig` exists, the epoch is current, the chain id matches the config, the signers hold 2/3+ of the current `ValidatorSet`, and the multi-signature verifies. It then calls `ethereum_headers::store_certified_header`, which stores `block_number -> (block_hash, state_root)` and emits `HeaderCertified`. Any failure discards the transaction and changes no state.
6. A validator drops its pooled certificate when a `HeaderCertified` event shows that block (or a later one) on chain.

The signed message is the BCS of `EthHeaderObservation` under its own `CryptoHasher` salt, so the signature is valid for nothing else.

## On-chain API

`aptos_framework::ethereum_headers`:

- `store_certified_header(framework: &signer, eth_chain_id, block_number, block_hash, state_root)`: the only writer. `public(friend)`, asserts the `@aptos_framework` signer, called by the VM. A second certificate for a stored block aborts (`E_ALREADY_CERTIFIED` when equal, `E_CONFLICTING_HEADER` when different). A certified header never changes.
- Views: `is_certified(block_number)`, `get_state_root(block_number)` and `get_block_hash(block_number)` (abort `E_HEADER_NOT_CERTIFIED` when absent), `latest_block_number(): Option<u64>`.
- `#[test_only] set_header_for_test(block_number, block_hash, state_root)` for Move tests of packages that read headers.

`aptos_framework::ethereum_header_config`: `set(framework, eth_chain_id)`, `disable(framework)`, view `eth_chain_id(): Option<u64>`. Attestation is on while the resource exists. Validators read it at the start of each epoch. The VM reads it on every certificate.

## Configuration

- Genesis: layout field `eth_chain_id` (`GenesisConfiguration::eth_chain_id`) calls `ethereum_header_config::set` at genesis. The docker script reads `ETH_CHAIN_ID`, `ETH_RPC_URL` and `ETH_BLOCK_TAG`.
- Node: `eth_header_observer { enabled, rpc_url, block_tag (default finalized), poll_interval_ms (default 12000), max_network_channel_size }`. A devnet without finality (anvil) uses `block_tag: latest`.
- Validator transactions must be on (`OnChainConsensusConfig::is_vtxn_enabled`).

## Trust assumptions

- 2/3+ of the voting power runs honest validators whose own Ethereum RPCs are honest. Each validator trusts only its own RPC. A validator with a lying RPC cannot make others sign a false header.
- The `@aptos_framework` signer (genesis, governance proposals) can change or remove `EthereumHeaderConfig` and can upgrade the framework, including `ethereum_headers`. Governance can therefore change header rules. It cannot write a header through any existing function: `store_certified_header` is friend-only.
- Changing `eth_chain_id` mid-epoch rejects certificates until the next epoch, because validators read the config at epoch start. Switching to another Ethereum network is out of scope: headers of the old chain stay stored under their block numbers.

## Out of scope

Pruning certified headers, slashing wrong observations, an Ethereum sync-committee or ZK light client (v1.0), and Ethereum-side verification of Aptos outcomes.

## Tests

| Behaviour | Test | CI |
| --- | --- | --- |
| Tag 5 and golden bytes of tags 0-5 | `cargo test -p aptos-types validator_txn` | Crypto Crates Tests |
| A peer with a different header refuses to sign; aggregation at quorum only | `cargo test -p aptos-eth-header-consensus` | Crypto Crates Tests |
| Quorum accepted; under-quorum, wrong epoch, wrong chain, forged signature, conflicting root, disabled rejected | `cargo test -p e2e-move-tests --lib eth_header::` | Crypto Crates Tests |
| Store, views, conflict, non-framework signer, test-only setter | `cargo test -p aptos-framework --test move_unit_test -- ethereum_headers` | Crypto Crates Tests |
| 4 validators certify a stub RPC header | `cargo test --release -p smoke-test --lib eth_header_e2e::certified_header_on_live_swarm` | Debug Smoke Tests |
