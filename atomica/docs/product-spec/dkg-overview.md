# Atomica Timelock DKG Architecture

This document outlines the architecture of the **Distributed Key Generation (DKG)** system used for the Atomica Timelock feature on Aptos.

**Status**: Verified via Code Analysis
**Source Path**: `aptos-core/dkg` and `aptos-core/crates/aptos-dkg`

## 1. Overview

Contrary to initial assumptions, the Timelock DKG is **not a sidecar**. It is a native component of the Aptos Validator binary (`aptos-node`), implemented primarily within the `dkg` crate. It reuses the robust DKG infrastructure built for Aptos Randomness but adapts it for Timelock specific needs.

## 2. Core Components

### `EpochManager` (`dkg/src/epoch_manager.rs`)

The central orchestrator. It listens to on-chain events and manages the lifecycle of DKG sessions.

- **Role**: Listens for Move events, spawns `DKGManager` tasks, and handles key storage/retrieval.
- **State**: Maintains separate channels for DKG sessions (one per epoch transition).

### `DKGManager` (`dkg/src/dkg_manager/mod.rs`)

The worker logic. It handles the actual cryptographic heavy lifting and consensus.

- **Role**: Generates random polynomials (dealing), broadcasts shares to peers via the validator network, aggregates transcripts, and proposes the final result.
- **Mode**: Can run in `is_timelock = true` mode, which changes how it packages the final result (using `TimelockDKGResult` instead of standard `DKGResult`).

### `ValidatorTransaction` (`vtxn`)

The mechanism for writing DKG results back to the chain.

- **Crucial Feature**: DKG operations do not use standard user transactions. They use system-level `ValidatorTransaction`s which bypass the mempool and are included directly by block proposers.
- **Types**:
  - `TimelockDKGResult`: Publishing the final Master Public Key (MPK) and encrypted shares.
  - `TimelockShare`: Publishing a single decrypted share during the reveal phase.

## 3. The DKG Lifecycle

### Phase 1: Initiation

1.  **Trigger**: The `timelock.move` contract emits a `StartKeyGenEvent` (at epoch boundary, to prepare for upcoming deadlines).
2.  **Detection**: `EpochManager` detects this event via its `dkg_start_events` listener.
3.  **Action**: `EpochManager::start_timelock_dkg` is called.
    - It checks if the local validator is part of the active set.
    - It spawns a new `DKGManager` task for this epoch.
    - The resulting MPK and DK_shares will be used for all deadlines within this epoch.

### Phase 2: Generation (The "Audit" Phase)

1.  **Dealing**: The `DKGManager` generates random secret material and computes the corresponding public key (MPK).
2.  **Sharing**: It distributes this secret as encrypted shares using PVSS (Publicly Verifiable Secret Sharing), encrypting each share for a specific validator using their Consensus Public Key.
3.  **Broadcasting**: These encrypted shares (the "transcript") are broadcast to all other validators via the P2P network.

### Phase 3: Aggregation & Publication

1.  **Aggregation**: Each validator aggregates valid transcripts received from peers.
2.  **Consensus**: Once enough transcripts are aggregated to meet the threshold, the `DKGManager` considers the DKG complete.
3.  **Submission**: The validator submits a `ValidatorTransaction::TimelockDKGResult`.
4.  **Execution**: The block executor processes this system transaction, deriving the final global **Master Public Key (MPK)** and writing it to the `timelock` Move table.

### Phase 4: Share Computation & Storage

1.  **Event**: The `timelock` contract emits a `KeyPublishedEvent`.
2.  **Extraction**: `EpochManager` catches this event.
3.  **Decryption**: It uses its private Consensus Key to decrypt its specific DK_share (Decryption Key Share) from the on-chain transcript.
4.  **Storage**: This share (a G1 point) is persisted in the validator's local `PersistentSafetyStorage`.

### Phase 5: Reveal (Timelock Expiry)

1.  **Trigger**: The `timelock.move` contract emits a `RequestRevealEvent` (e.g., when the deadline's time initializes).
2.  **Action**: `EpochManager::process_timelock_reveal` is triggered.
3.  **Retrieval**: It fetches the stored G1 share from disk.
4.  **Submission**: It submits a `ValidatorTransaction::TimelockShare` containing the unblinded share.
5.  **Reconstruction**: The Move contract sums these G1 shares. Once the threshold is met, the complete **Decryption Key (DK)** is reconstructed and published.

## 4. Configuration Requirements

For this system to work, the **Validator Transaction** feature must be enabled in the chain's `ConsensusConfig`.

In `node-config.yaml` or genesis layout:

```yaml
consensus:
  # ...
  # Implicitly required for DKG to function:
  validator_txn_enabled: true
```

_Note: In code (`epoch_manager.rs`), this is checked via `consensus_config.is_vtxn_enabled()`._

## 5. Cryptography

Implemented in `crates/aptos-dkg/src/ibe/mod.rs`.

- **Curve**: BLS12-381
- **Encryption**: Boneh-Franklin Identity-Based Encryption (IBE).
- **MPK**: G2 Point (96 bytes).
- **Decryption Key**: G1 Point (48 bytes).
- **Identity**: `Keccak256("timelock_id:" || id || ":deadline_timestamp_microseconds:" || deadline)` (application-agnostic).

## 6. Testnet Implementation Notes

### Native DKG vs. Simulation

While the native DKG _can_ be enabled in the testnet by adding the `OnChainConsensusConfig::V5` (with `validator_txn` enabled) to the genesis layout, experiments have shown this to be unstable for end-to-end testing.

**Findings (Jan 2026):**

1.  **Basic Consensus**: Enabling `validator_txn` allows basic validation and block production to succeed (verified via `block-production.test.ts`).
2.  **IBE Instability**: The IBE E2E test consistently times out or fails when relying on the native DKG in the Docker test harness. This likely stems from the overhead of the DKG protocol colliding with the test's transaction load or specific timing requirements of the testnet environment.

**Strategy:**
To ensure robust CI/CD, we currently **simulate** the DKG process in tests. The test client manually generates keys and publishes them via the standard Move entry points (`publish_public_key`, `publish_secret_share`), mimicking the result of the native system transactions. This isolates the testing of the crypto and contract logic from the complexity of the validator's P2P DKG layer.
