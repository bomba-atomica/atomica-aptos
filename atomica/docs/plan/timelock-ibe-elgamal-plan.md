# Timelock Encryption Implementation Plan

## IBE + DKG with Chunked Lifted ElGamal PVSS

**Version:** 3.3
**Date:** January 22, 2026
**Branch:** `timelock-elgamal-pvss`
**Status:** Phase 5.2 (IBE DK Reconstruction) COMPLETE

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
Identity = SHA3-256(APTOS_IBE_IDENTITY_DST || timelock_id || deadline_us)
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
  │   ID = SHA3-256("APTOS_IBE_IDENTITY_DST" || timelock_id || deadline_us)
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

### Phase 6: On-Chain DK Reconstruction ✅ IMPLEMENTED

**THIS PHASE IS NOW COMPLETE (Phase 5.2)**

The on-chain reconstruction uses the unified `reconstruct_ibe_dk()` path:

```
timelock::submit_dk_share()
  │
  ├─→ Collect DK_i from validators
  │
  ├─→ Parse scalar shares from Move byte vectors
  │
  ├─→ Call native function:
  │   reconstruct_ibe_dk_internal<G1>(
  │       validator_indices,
  │       scalar_shares,           // vector<vector<u8>>
  │       validator_weights,
  │       threshold,
  │       total_weight,
  │       identity
  │   )
  │
  ├─→ Native function delegates to apt-dkg:
  │   apt_dkg::ibe::reconstruct_ibe_dk()
  │     → WeightedConfig::new()
  │     → DealtSecretKey::reconstruct()
  │     → derive_decryption_key()
  │
  └─→ When threshold (2f+1) reached:
      DK = Σ (λ_i × DK_i)    // Lagrange interpolation
      Store DK, emit SecretRevealedEvent
```

#### Native Function Details

**Location:** `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`

**What it does:**

1. Parses `scalar_shares: vector<vector<u8>>` → `Vec<Vec<Scalar>>`
2. Converts `identity: vector<u8>` → `[u8; 32]`
3. Delegates to `aptos_dkg::ibe::reconstruct_ibe_dk()` for crypto
4. Stores result and returns handle

**What it does NOT do:**

- ❌ No custom cryptographic operations
- ❌ No custom Lagrange interpolation
- ❌ No custom hash-to-curve

**Security:** All crypto delegated to the canonical Rust SDK implementation.

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

| Component                   | Location                                         | Description                            |
| --------------------------- | ------------------------------------------------ | -------------------------------------- |
| IBE Primitives              | `crates/aptos-dkg/src/ibe/mod.rs`                | encrypt, decrypt, derive_dk, identity  |
| Chunked Lifted ElGamal PVSS | `crates/aptos-dkg/src/pvss/scalar_elgamal/`      | deal, aggregate, decrypt shares        |
| IBE DKG Integration         | `dkg/src/ibe_dkg.rs`                             | Dual-output DKG                        |
| DKG Manager                 | `dkg/src/dkg_manager/mod.rs`                     | DKG lifecycle                          |
| Epoch Manager               | `dkg/src/epoch_manager.rs`                       | Event handlers, share submission       |
| Move: timelock              | `timelock.move`                                  | Registry, aggregation                  |
| Move: threshold_dsa         | `threshold_dsa.move`                             | MPK, threshold ops                     |
| Move: ibe_config            | `ibe_config.move`                                | IBE configuration                      |
| Move: ibe                   | `ibe.move`                                       | **DK reconstruction API**              |
| Move: timelock handler      | `aptos-vm/.../timelock.rs`                       | ValidatorTransaction handler           |
| Native: ibe                 | `natives/cryptography/algebra/ibe.rs`            | **Native function (delegates to SDK)** |
| Tests (65+)                 | `crates/aptos-dkg/src/ibe/tests.rs`              | All tests pass                         |
| Move Tests (3)              | `natives/cryptography/algebra/ibe_tests.rs`      | Native function tests pass             |
| Golden Vectors              | `atomica/golden_vectors/ibe_golden_vectors.json` | Cryptographic test vectors             |
| **DK Reconstruction**       | `crates/aptos-dkg/src/ibe/mod.rs`                | **Unified `reconstruct_ibe_dk()`**     |

### Implementation Architecture: DK Reconstruction

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DK Reconstruction Implementation                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Move Script:                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  ibe::reconstruct_ibe_dk<G1>(                                        │    │
│  │      validator_indices,      // vector<u64>                          │    │
│  │      scalar_shares,          // vector<vector<u8>> (32-byte LE)     │    │
│  │      weights,                // vector<u64>                          │    │
│  │      threshold,              // u64                                  │    │
│  │      total_weight,           // u64                                  │    │
│  │      identity                // vector<u8> (32 bytes)                │    │
│  │  ) -> Element<G1>                                                       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                    │                                        │
│                                    ▼                                        │
│  Native Function (ibe.rs):                                                │
│  1. Parse scalar_shares from vector<vector<u8>> to Vec<Vec<Scalar>>        │
│  2. Convert identity Vec<u8> to [u8; 32]                                   │
│  3. Call apt_dkg::ibe::reconstruct_ibe_dk() [DELEGATION POINT]             │
│  4. Store result and return handle                                         │
│                                    │                                        │
│                                    ▼                                        │
│  Rust SDK (ibe/mod.rs):                                                    │
│  1. Create DealtSecretKeyShare from each scalar                            │
│  2. Call WeightedTranscript::DealtSecretKey::reconstruct()                 │
│  3. Derive DK = H(identity)^secret                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Pending 🔲

| Phase   | Description                          | Priority |
| ------- | ------------------------------------ | -------- |
| **5.3** | Full E2E Smoke Test (Reveal Flow)    | High     |
| **-**   | Project Polishing and Security Audit | Medium   |

---

## Success Criteria

- [x] Phase 5.1 complete (Linear Pairing Check)
- [x] Phase 5.2 complete (DK Reconstruction)
- [x] Golden test vectors generated and integrated
- [x] Native function for DK reconstruction implemented
- [x] Native function delegates to apt-dkg (no custom crypto)
- [x] All 65 IBE/DKG tests pass
- [ ] 100% Move test coverage for `ibe.move` and `ibe_config.move`
- [ ] E2E smoke test passes (IBE flow + on-chain reconstruction)
- [ ] Timelock deadline passing and event flow implemented
- [ ] Security review completed

---

## Testing Roadmap

### Phase 5.2: IBE DK Reconstruction ✅ COMPLETE

**Location:**

- Rust SDK: `crates/aptos-dkg/src/ibe/mod.rs`
- Native Function: `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`
- Move API: `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move`

**Status:** All tests pass

**Test Results:**

```
Rust SDK (aptos-dkg):
  - test_reconstruct_ibe_dk_single_share ... ok
  - test_reconstruct_ibe_dk_equal_weights ... ok
  - test_reconstruct_ibe_dk_sparse_indices ... ok
  - test_reconstruct_ibe_dk_unequal_weights ... ok
  - test_dk_share_aggregation_roundtrip ... ok
  - All 65 tests pass ✅

Framework (Move VM):
  - test_dk_reconstruction_basic ... ok
  - test_dk_reconstruction_sparse_indices ... ok
  - test_dk_reconstruction_unequal_weights ... ok
  - All 3 tests pass ✅
```

### Phase 5.3: E2E Smoke Tests ⏳ PENDING

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

---

## Feature: DK Reconstruction Native Function

### Implementation Summary

The native function `reconstruct_ibe_dk_internal()` in `ibe.rs` provides on-chain DK reconstruction for the Move VM.

**Key Design Principles:**

1. **Minimal Native Code**: The native function only handles:
   - Argument parsing from Move VM
   - Scalar deserialization
   - Delegation to Rust SDK
   - Result storage

2. **Delegation Pattern**: All cryptographic operations delegated to `aptos_dkg::ibe::reconstruct_ibe_dk()`

3. **No Custom Crypto**: Eliminates risk of divergence between Rust SDK and Move VM

### Function Signature

```rust
pub fn reconstruct_ibe_dk_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>>
```

### Arguments (from Move)

1. `identity` (Vec<u8>) - 32-byte IBE identity
2. `total_weight` (u64) - Sum of all validator weights
3. `threshold` (u64) - Minimum shares required
4. `weights` (Vec<u64>) - Full weights for ALL validators
5. `scalar_shares` (Vec<Vec<u8>>) - Scalar shares, each inner vector is 32-byte LE scalars
6. `validator_indices` (Vec<u64>) - Indices of participating validators

### Return Value

Returns a handle (`u64`) to the stored G1 element (the reconstructed DK).

### Error Handling

- `E_TOO_MUCH_MEMORY_USED`: If storing result exceeds memory limit
- Invariant violation: If arguments fail validation

---

## Files Modified (v3.3 Update)

| File                                                                 | Change                                                  |
| -------------------------------------------------------------------- | ------------------------------------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                                    | Added `reconstruct_ibe_dk()` with verbose documentation |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`       | Native function with delegation to apt-dkg              |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move`    | Updated Move API with new signature                     |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe_tests.rs` | Native function tests                                   |
| `atomica/docs/plan/implementation-plan-unified-dkg-ibe.md`           | Updated Phase 5.2 status                                |

---

## Related Documents

| Document                                                                         | Purpose                      |
| -------------------------------------------------------------------------------- | ---------------------------- |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md)                         | Architecture decision record |
| [definitions.md](definitions.md)                                                 | Core terminology             |
| [implementation-plan-unified-dkg-ibe.md](implementation-plan-unified-dkg-ibe.md) | Full implementation plan     |

---

## Changelog

- **v3.3** (Jan 22, 2026): **Phase 5.2 (DK Reconstruction) COMPLETE**
  - Implemented unified `reconstruct_ibe_dk()` in Rust SDK
  - Implemented native function `reconstruct_ibe_dk_internal()` with delegation
  - Updated Move API with new signature
  - All 65 IBE/DKG tests pass
  - Native function delegates to apt-dkg (no custom crypto)
- **v3.2** (Jan 21, 2026): Added DK share aggregation test
- **v3.1** (Jan 20, 2026): Completed IBE native function implementation and Move tests
- **v3.0** (Jan 20, 2026): Clarified single IBE protocol, DKG uses dual-output
