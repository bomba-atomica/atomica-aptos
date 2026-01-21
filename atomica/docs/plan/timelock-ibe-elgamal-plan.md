# Timelock Encryption Implementation Plan

## IBE + DKG with Chunked Lifted ElGamal PVSS

**Version:** 3.2  
**Date:** January 21, 2026  
**Branch:** `timelock-elgamal-pvss`  
**Status:** Phase 4 complete, E2E tests ready for CLI build

**Reference:** [ADR-001: Dual-Output DKG](adr-001-dual-output-dkg.md)

---

## Architecture Overview

The system uses **one IBE protocol** (Boneh-Franklin style). The DKG produces scalar key material using **Chunked Lifted ElGamal PVSS**, which becomes the IBE master secret.

```
DKG (Dual-Output)
        │
        ├─────────────────────┬────────────────────────────┐
        │                     │                            │
        ▼                     ▼                            ▼
   ┌─────────┐      ┌─────────────────────┐      ┌─────────────────┐
   │DAS PVSS │      │Chunked Lifted       │      │   MPK = g2^a    │
   │  (G1)   │      │ElGamal PVSS         │      │  (on-chain)     │
   └────┬────┘      │   (Scalar)          │      └─────────────────┘
        │           └──────────┬──────────┘
        ▼                      ▼
   WVUF/Randomness        IBE Master Secret (Scalar a)
                          DK = a × H(identity)

```

### Key Clarification

| Component                       | Output              | Purpose                     |
| ------------------------------- | ------------------- | --------------------------- |
| **DAS PVSS**                    | G1 shares           | WVUF/randomness (unchanged) |
| **Chunked Lifted ElGamal PVSS** | Scalar shares       | **IBE master secret**       |
| **IBE protocol**                | Encrypt/Decrypt ops | Uses scalar-derived keys    |

The `ibe/mod.rs` file provides the **IBE cryptographic primitives** (encrypt/decrypt). The `pvss/scalar_elgamal/` module provides the **DKG/PVSS protocol** that generates the scalar key material.

---

## IBE Protocol (Boneh-Franklin Style)

Given a master secret scalar `s` and identity string:

```
Identity = Keccak256("timelock_id:{id}:deadline_timestamp_microseconds:{deadline}")
DK = s × H(identity)    // G1 point
MPK = g2^s              // G2 point (on-chain)
```

### Key Derivation

- **Master Secret**: Scalar `s` (reconstructed from threshold shares)
- **Master Public Key**: G2 point `g2^s`
- **Decryption Key**: G1 point `s × H(identity)`
- **DK Share**: G1 point `s_i × H(identity)` from validator `i`

### On-Chain Aggregation

```
DK = Σ (λ_i × DK_i)    // Lagrange interpolation of validator shares
```

---

## Protocol Flow

### Phase 1: DKG Setup (Dual-Output)

```
InputSecret (scalar a)
        │
        ├───────────────────────────────────────────────┐
        │                                               │
        ▼                                               ▼
   ┌─────────────┐                             ┌─────────────────────┐
   │  DAS PVSS   │                             │ Chunked Lifted      │
   │  (unchanged)│                             │ ElGamal PVSS        │
   └──────┬──────┘                             └──────────┬──────────┘
          │                                               │
          ▼                                               ▼
     G1 shares                                      Scalar shares
     (for WVUF)                                     (for IBE ← THIS IS THE MASTER SECRET)

MPK = g2^a (published on-chain)
```

1. Validators run DKG with dual-output
2. DAS PVSS produces G1 shares for randomness (unchanged)
3. **Chunked Lifted ElGamal produces scalar shares** → this is the IBE master secret
4. Each validator stores their scalar share `s_i`

### Phase 2: Client Encryption

```
Client
  │
  ├─→ Fetch MPK (g2^a) from chain
  │
  ├─→ Compute identity:
  │   ID = Keccak256("timelock_id:{id}:deadline_timestamp_microseconds:{deadline}")
  │
  └─→ Encrypt:
     CT = IBE.Encrypt(MPK=g2^a, identity=ID, message)
```

Uses `ibe_encrypt()` from `ibe/mod.rs`.

### Phase 3: Timelock Registration

```
Client → timelock::register(deadline) → gets timelock_id
```

Chain stores deadline, emits `TimelockRegisteredEvent`.

### Phase 4: Deadline Detection

```
on_new_block()
  │
  └─→ If block.timestamp ≥ deadline:
      emit RequestRevealEvent(deadline, [timelock_ids])
```

### Phase 5: Validator Share Derivation

When validators receive `RequestRevealEvent`:

```
For each timelock_id:
  │
  ├─→ Retrieve stored scalar share s_i
  │
  ├─→ Compute identity ID (same as client)
  │
  ├─→ Derive DK share:
  │   DK_i = s_i × H(ID)    // G1 point
  │
  └─→ Submit ValidatorTransaction::TimelockShare(DK_i)
```

### Phase 6: On-Chain Aggregation

```
timelock::publish_decryption_key_share()
  │
  ├─→ Collect DK_i from validators
  │
  ├─→ When threshold (2f+1) reached:
  │   DK = Σ (λ_i × DK_i)    // Lagrange interpolation
  │
  └─→ Store DK, emit SecretRevealedEvent
```

### Phase 7: Client Decryption

```
Client
  │
  ├─→ Poll get_decryption_key(timelock_id)
  │
  └─→ Decrypt:
     plaintext = IBE.Decrypt(DK, CT)
```

---

## Implementation Status

### Complete ✅

| Component                   | Location                                    | Description                           |
| --------------------------- | ------------------------------------------- | ------------------------------------- |
| IBE Primitives              | `crates/aptos-dkg/src/ibe/mod.rs`           | encrypt, decrypt, derive_dk, identity |
| Chunked Lifted ElGamal PVSS | `crates/aptos-dkg/src/pvss/scalar_elgamal/` | deal, aggregate, decrypt shares       |
| IBE DKG Integration         | `dkg/src/ibe_dkg.rs`                        | Dual-output DKG                       |
| DKG Manager                 | `dkg/src/dkg_manager/mod.rs`                | DKG lifecycle                         |
| Epoch Manager               | `dkg/src/epoch_manager.rs`                  | Event handlers, share submission      |
| Move: timelock              | `timelock.move`                             | Registry, aggregation                 |
| Move: threshold_dsa         | `threshold_dsa.move`                        | MPK, threshold ops                    |
| Move: ibe_config            | `ibe_config.move`                           | IBE configuration                     |
| Move: timelock handler      | `aptos-vm/.../timelock.rs`                  | ValidatorTransaction handler          |
| Tests (30+)                 | `testsuite/smoke-test/src/timelock/`        | Integration tests                     |
| Linear Pairing Check        | `transcript.rs`                             | Aggregated verification               |

### Pending 🔲

| Phase   | Description                              | Priority |
| ------- | ---------------------------------------- | -------- |
| **5.2** | 100% Move test coverage for IBE/Timelock | High     |
| **5.3** | Full E2E Smoke Test (Reveal Flow)        | High     |
| **-**   | Project Polishing and Security Audit     | Medium   |

---

## Success Criteria

- [x] Phase 5.1 complete
- [x] Golden test vectors generated and integrated
- [x] Native function for DK reconstruction implemented
- [ ] 100% Move test coverage for `ibe.move` and `ibe_config.move`
- [ ] E2E smoke test passes (IBE flow + on-chain reconstruction)
- [ ] Timelock deadline passing and event flow implemented
- [ ] Security review completed

---

## Testing Roadmap

### Phase 5: Move Language Tests (100% Coverage) ⏳ IN PROGRESS

**Location:** `aptos-move/framework/aptos-framework/sources/ibe_config.move` and `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move`

**Status:** Tests added, verifying with `aptos` CLI

**Tasks:**

- [x] Build `aptos` CLI and install to `~/.cargo/bin`
- [ ] Run Move tests and check coverage
- [ ] Expand `ibe.move` tests for all native function edge cases
- [ ] Expand `ibe_config.move` tests for full registry lifecycle
- [ ] Add tests for weighted reconstruction in Move

### Phase 6: E2E Smoke Tests ⏳ PENDING

**Location:** `testsuite/smoke-test/src/timelock/`

**Goal:** Test full IBE flow on-chain with validator share submission and DK reconstruction

**Test Flow:**

1. User registers timelock with deadline
2. Validators decrypt their scalar shares from DKG
3. Each validator computes DK share: `dk_share_i = H(identity) * s_i`
4. User encrypts message with IBE (off-chain)
5. Deadline passes (on_new_block)
6. System emits `TimelockRevealEvent`
7. Validators submit `TimelockShare` transactions
8. Contract reconstructs DK via `reconstruct_ibe_dk_internal()`
9. User queries and decrypts message

**Pending Tasks:**

- [ ] Create smoke test module
- [ ] Implement full roundtrip test
- [ ] Test with localnet validator set

**Tasks:**

- [ ] Implement `test_ibe_encrypt_decrypt_onchain()`
- [ ] Implement `test_timelock_deadline_passes()`
- [ ] Implement `test_validator_submit_dk_shares()`
- [ ] Implement `test_dk_reconstruction_event_flow()`
- [ ] Test with multiple validator configurations

---

## Feature: Timelock Deadline Event Flow

### Current State

The timelock registry is implemented, but the event-driven reveal flow is not yet complete.

### Required Implementation

#### 1. Deadline Check in `on_new_block()`

**Location:** Likely in `aptos-framework` or `reconfiguration` module

**Logic:**

```rust
public fun on_new_block(block_height: u64) {
    // Check all timelocks
    let registry = borrow_global<TimelockRegistry>(@aptos_framework);
    let current_time = timestamp::now_microseconds();

    // For each timelock where deadline has passed but not revealed
    for (timelock_id, timelock_info) in registry.timelocks.iter() {
        if (!timelock_info.is_revealed && current_time >= timelock_info.deadline_us) {
            // Emit event to notify validators
            event::emit_event<TimelockExpiredEvent>(
                &mut registry.expired_events,
                TimelockExpiredEvent { timelock_id }
            );
        }
    }
}
```

#### 2. Validator Subscription

**Location:** Validator consensus/replication layer

**Logic:**

```rust
// Validator subscribes to TimelockExpiredEvent
fn handle_timelock_expired(event: TimelockExpiredEvent) {
    let timelock_id = event.timelock_id;

    // Get timelock info from registry
    let (deadline_us, identity, _, _) = ibe_config::get_timelock(timelock_id);

    // Compute DK share: dk_share = H(identity) * scalar_share
    let dk_share = compute_dk_share(validator_key, &identity);

    // Submit TimelockShare transaction
    submit_timelock_share(timelock_id, dk_share);
}
```

#### 3. On-Chain Reconstruction

**Location:** `ibe_config.move::submit_dk_share()`

**Updated Logic:**

```move
public(friend) fun submit_dk_share(
    timelock_id: u64,
    dk_share: vector<u8>,
    validator_address: address,
    weight: u64,
    total_weight: u64
) acquires TimelockRegistry {
    // Deserialize DK share to G1 element
    let dk_share_g1 = crypto_algebra::deserialize<G1, FormatG1Compr>(dk_share);

    // Get handles for reconstruction
    let shares_vector = vector::push_back(
        existing_shares_handles,
        dk_share_g1.handle
    );

    // Check if threshold reached
    let current_weight = get_current_weight(timelock_id) + weight;
    if (current_weight >= reveal_threshold) {
        // Reconstruct DK using native function
        let dk = ibe::reconstruct_ibe_dk_internal<G1>(
            validator_indices,  // All validators who submitted
            shares_vector,       // G1 element handles
            validator_weights,   // Weights
            threshold,           // Threshold
            total_weight         // Total weight
        );

        // Store reconstructed DK
        timelock_info.decryption_key = crypto_algebra::serialize(dk);
        timelock_info.is_revealed = true;

        // Emit reveal event
        event::emit_event(&mut registry.reveal_events, TimelockRevealEvent {
            timelock_id,
            timestamp_us: current_time,
        });
    }
}
```

### Tasks

- [ ] Add `TimelockExpiredEvent` to `ibe_config.move`
- [ ] Add `expired_events` to `TimelockRegistry`
- [ ] Implement deadline check in `on_new_block()` or similar
- [ ] Update validator code to subscribe to `TimelockExpiredEvent`
- [ ] Implement `submit_dk_share()` using new native function
- [ ] Add integration test for full event flow

---

## Files Modified (v3.2 Update)

| File                                       | Change                                                                                           |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `crates/aptos-dkg/src/ibe/tests.rs`        | Added `test_dk_share_aggregation_roundtrip()`                                                    |
| `crates/aptos-dkg/src/ibe/mod.rs`          | Updated module exports                                                                           |
| `testsuite/smoke-test/src/ibe/mod.rs`      | IBE E2E tests exist (elgamal_encrypt_decrypt, elgamal_encrypt_decrypt_with_different_identities) |
| `testsuite/smoke-test/src/timelock/mod.rs` | Timelock smoke tests (register_and_query, deadline_reveal)                                       |

---

## Existing Smoke Tests

### IBE Tests (`testsuite/smoke-test/src/ibe/mod.rs`)

| Test                                                | Description                                   | Status |
| --------------------------------------------------- | --------------------------------------------- | ------ |
| `elgamal_encrypt_decrypt`                           | IBE encrypt/decrypt with reconstructed secret | ✅     |
| `elgamal_encrypt_decrypt_with_different_identities` | Multiple identities with same secret          | ✅     |

### Timelock Tests (`testsuite/smoke-test/src/timelock/`)

| Test                 | Description                              | Status |
| -------------------- | ---------------------------------------- | ------ |
| `register_and_query` | Timelock registration and view functions | ✅     |
| `deadline_reveal`    | DK share submission infrastructure       | ✅     |

### Next Steps

1. Build `aptos` CLI: `cargo build --release -p aptos`
2. Run Move tests: `aptos move test --package-dir aptos-move/framework/aptos-framework`
3. Run smoke tests: `cargo test -p smoke-test --lib timelock`
4. Implement on-chain DK reconstruction test

---

## Changelog

- **v3.2** (Jan 21, 2026): Added DK share aggregation test
  - Implemented `test_dk_share_aggregation_roundtrip()` in `aptos-dkg`
  - Validates DKG share decryption, scalar reconstruction, IBE roundtrip
  - All 27 IBE tests passing
- **v3.1** (Jan 20, 2026): Completed IBE native function implementation and Move tests
- **v3.0** (Jan 20, 2026): Clarified single IBE protocol, DKG uses dual-output

---

## Related Documents

| Document                                                 | Purpose                      |
| -------------------------------------------------------- | ---------------------------- |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md) | Architecture decision record |
| [definitions.md](definitions.md)                         | Core terminology             |

---

## Changelog

- **v3.1** (Jan 20, 2026): Completed IBE native function implementation and Move tests
  - Fixed `reconstruct_ibe_dk_internal()` native function in `ibe.rs`
  - Created `aptos_std::ibe` Move wrapper module
  - Added Move tests with golden vectors in `ibe_config.move`
  - All 26 IBE tests passing in `aptos-dkg`
- **v3.0** (Jan 20, 2026): Clarified single IBE protocol, DKG uses dual-output (DAS + Chunked Lifted ElGamal)
- **v2.13** (Jan 19, 2026): Previous multi-document version
