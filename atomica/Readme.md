# Atomica Aptos Timelock & IBE

## Overview

This project implements a distributed key generation (DKG) based time-lock encryption system integrated into the Aptos blockchain for Atomica. It enables time-locked encryption where messages can only be decrypted after a specific blockchain interval has passed.

The system uses **BLS12-381** cryptography and Identity-Based Encryption (IBE) to derive keys based on time intervals.

### Key Features

*   **Timelock DKG**: Distributed key generation with time-based intervals.
*   **IBE Encryption**: Messages encrypted with identity-based keys derived from timelock intervals.
*   **Move Framework Integration**: Full Aptos blockchain integration with Move smart contracts.
*   **Comprehensive Testing**: Automated Docker-based testnet infrastructure.
*   **Framework Verification**: Ability to test custom framework loading vs Docker image defaults.
*   **Custom Genesis**: Support for injecting modified Move frameworks (e.g., shorter intervals) into ephemeral testnets via `move-framework-fixtures/head.mrb`.

### Status (as of Jan 1, 2026)

*   **Core Logic**: ~85% Complete (MVP Assessment).
*   **Infrastructure**: Fully functional automated Docker testnets with custom genesis support.
*   **Cryptographic Ops**: IBE encrypt/decrypt cycle functional using Boneh-Franklin scheme.
*   **Pending**: Critical bug fixes (invalid share counting, threshold storage) and full verification flow.

---

## Architecture

The system spans Move smart contracts, Rust validator node code, and cryptographic primitives.

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         ON-CHAIN (Move)                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌──────────────────┐                    │
│  │ timelock.move   │◄───│ timelock_config  │                    │
│  │                 │    │     .move        │                    │
│  │ - TimelockState │    │                  │                    │
│  │ - Events        │    │ - Interval cfg   │                    │
│  │ - Aggregation   │    │ - Mainnet guard  │                    │
│  └────────┬────────┘    └──────────────────┘                    │
│           │                                                      │
│  ┌────────▼────────┐                                            │
│  │   block.move    │ ◄── on_new_block() triggers rotation       │
│  └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Events
┌─────────────────────────────────────────────────────────────────┐
│                      VALIDATOR NODE (Rust)                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌──────────────────┐                    │
│  │ epoch_manager   │───►│   dkg_manager    │                    │
│  │     .rs         │    │      .rs         │                    │
│  │                 │    │                  │                    │
│  │ - Event handler │    │ - DKG execution  │                    │
│  │ - Share storage │    │ - Aggregation    │                    │
│  │ - Key extraction│    │ - Pool submit    │                    │
│  └────────┬────────┘    └──────────────────┘                    │
│           │                                                      │
│  ┌────────▼────────┐    ┌──────────────────┐                    │
│  │ persistent_     │    │ validator_txns/  │                    │
│  │ safety_storage  │    │ timelock.rs      │                    │
│  │                 │    │                  │                    │
│  │ - Share persist │    │ - VM execution   │                    │
│  └─────────────────┘    └──────────────────┘                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      CRYPTOGRAPHY (Rust)                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    aptos-dkg/src/ibe/                       ││
│  │                                                              ││
│  │  - ibe_encrypt(mpk, identity, message) -> Ciphertext        ││
│  │  - ibe_decrypt(dk, ciphertext) -> Plaintext                 ││
│  │  - compute_timelock_identity(interval, chain_id) -> bytes   ││
│  │  - serialize_g1/g2, deserialize_g1/g2                       ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1.  **Interval Rotation**: `on_new_block` triggers `StartKeyGenEvent`.
2.  **DKG Execution**: Validators run DKG (`epoch_manager`).
3.  **Submission**: Aggregated transcript submitted via `TimelockDKGResult`.
4.  **Publication**: `publish_public_key` stores the Master Public Key (MPK) on-chain.
5.  **Reveal Request**: Next rotation triggers `RequestRevealEvent` for previous interval.
6.  **Share Reveal**: Validators reveal shares; `publish_secret_share` aggregates them.
7.  **Final Secret**: `SecretRevealedEvent` emitted when threshold is met.

Client usage involves fetching the MPK for interval `N`, encrypting for that identity, and waiting for interval `N+1` to fetch the revealed secret for decryption.

---

## Data Structures

### Move: TimelockState

```move
struct TimelockState has key {
    current_interval: u64,
    last_rotation_time: u64,
    public_keys: Table<u64, vector<u8>>,        // interval -> MPK (transcript)
    validator_shares: Table<u64, vector<ValidatorShare>>,  // interval -> shares
    revealed_secrets: Table<u64, vector<u8>>,   // interval -> aggregated DK
    // Event handles...
}
```

### Rust: TimelockShare

```rust
pub struct TimelockShare {
    pub interval: u64,
    pub author: AccountAddress,
    pub share: Vec<u8>,  // Serialized G1 point (48 bytes compressed)
}
```

---

## Project Structure

*   **`aptos-move/framework/aptos-framework/sources/timelock.move`**: Main timelock logic.
*   **`atomica/`**: Project documentation and specific test suites.
    *   `docker-test-harness/`: SDK for managing Docker testnets.
    *   `timelock-tests/`: TypeScript-based integration tests.
    *   `move-framework-fixtures/`: Test artifacts (compiled `.mrb` files).

---

For further details on testing, see [`testing.md`](./testing.md).
For development plans and roadmap, see [`development.plan`](./development.plan).
