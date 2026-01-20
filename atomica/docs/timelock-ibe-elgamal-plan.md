# Timelock Encryption Implementation Plan

## IBE + DKG with Chunked Lifted ElGamal PVSS

**Version:** 3.0  
**Date:** January 20, 2026  
**Branch:** `timelock-vpss`  
**Status:** Implementation largely complete, Phase 5.1 pending

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

### Pending 🔲

| Phase   | Description                                                 | Priority |
| ------- | ----------------------------------------------------------- | -------- |
| **5.1** | Linear Pairing Check for aggregated transcript verification | **HIGH** |

---

## Risk Assessment

### Critical

| Risk                                       | Mitigation                               |
| ------------------------------------------ | ---------------------------------------- |
| No verification for aggregated transcripts | Implement Phase 5.1 linear pairing check |
| DKG channel panic on termination           | Fix at commit 77784ca8bf                 |

### High

| Risk                                            | Mitigation      |
| ----------------------------------------------- | --------------- |
| No Chaum-Pedersen proofs for share verification | Add DLOG proofs |

---

## Key Files

### Cryptography

| File                                                     | Purpose                          |
| -------------------------------------------------------- | -------------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                        | IBE primitives (encrypt/decrypt) |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` | PVSS protocol (2100+ lines)      |
| `dkg/src/ibe_dkg.rs`                                     | Dual-output DKG                  |
| `dkg/src/epoch_manager.rs`                               | Validator event handling         |

### Move

| File                 | Purpose                  |
| -------------------- | ------------------------ |
| `timelock.move`      | Registry and aggregation |
| `threshold_dsa.move` | MPK and threshold DSA    |

### Tests

| File                                 | Purpose          |
| ------------------------------------ | ---------------- |
| `testsuite/smoke-test/src/timelock/` | 30+ smoke tests  |
| `atomica/timelock-tests/test/`       | TypeScript tests |

---

## Phase 5.1: Linear Pairing Check (Next Task)

**Goal:** Verify encryption correctness for aggregated transcripts without per-dealer DLEQ proofs.

**Reference:** Upstream `chunky/transcript.rs:303-313`

**Tasks:**

1. Derive linear equation for Chunked Lifted ElGamal
2. Implement `verify_linear_pairing_check()` in `transcript.rs`
3. Add unit tests

---

## Success Criteria

- [ ] Phase 5.1 complete
- [ ] E2E smoke test passes
- [ ] Security review completed

---

## Related Documents

| Document                                                 | Purpose                      |
| -------------------------------------------------------- | ---------------------------- |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md) | Architecture decision record |
| [definitions.md](definitions.md)                         | Core terminology             |

---

## Changelog

- **v3.0** (Jan 20, 2026): Clarified single IBE protocol, DKG uses dual-output (DAS + Chunked Lifted ElGamal)
- **v2.13** (Jan 19, 2026): Previous multi-document version
