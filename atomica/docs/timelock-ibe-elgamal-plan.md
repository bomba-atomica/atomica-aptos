# Timelock Encryption Implementation Plan

## IBE + ElGamal PVSS Integration

**Version:** 3.0  
**Date:** January 20, 2026  
**Branch:** `timelock-vpss`  
**Status:** RE-EVALUATED - Implementation largely complete, Phase 5.1 pending

---

## Executive Summary

This document is the **single source of truth** for the Atomica timelock encryption implementation. It consolidates findings from code analysis, testing, and risk assessment.

**Key Finding:** The implementation is **largely complete** (~4000+ lines of Rust/Move/TypeScript). The critical pending item is **Phase 5.1: Linear Pairing Check** for aggregated transcript verification.

---

## Architecture Overview

```
InputSecret (scalar a)
        │
        ├─────────────────────┬────────────────────────────┐
        │                     │                            │
        ▼                     ▼                            ▼
   ┌─────────┐      ┌─────────────────────┐      ┌─────────────────┐
   │DAS PVSS │      │Chunked Lifted       │      │   MPK = g2^a    │
   │  (G1)   │      │ElGamal PVSS (Scalar)│      │  (on-chain)     │
   └────┬────┘      └──────────┬──────────┘      └─────────────────┘
        │                      │
        ▼                      ▼
   WVUF/Randomness        IBE Secrets
```

---

## What Has Been Built

### Cryptographic Layer

| Component           | Location                                    | Status      | Lines |
| ------------------- | ------------------------------------------- | ----------- | ----- |
| IBE Primitives      | `crates/aptos-dkg/src/ibe/mod.rs`           | ✅ Complete | ~415  |
| Fp12 Serialization  | `crates/aptos-dkg/src/ibe/fp12_*.rs`        | ✅ Complete | ~300  |
| IBE DKG Protocol    | `dkg/src/ibe_dkg.rs`                        | ✅ Complete | ~1027 |
| Scalar ElGamal PVSS | `crates/aptos-dkg/src/pvss/scalar_elgamal/` | ✅ Complete | ~2100 |

### On-Chain Layer (Move)

| Component         | Location             | Status      |
| ----------------- | -------------------- | ----------- |
| Timelock Registry | `timelock.move`      | ✅ Complete |
| Threshold DSA     | `threshold_dsa.move` | ✅ Complete |
| IBE Config        | `ibe_config.move`    | ✅ Complete |

### Validator Services

| Component          | Location                     | Status      |
| ------------------ | ---------------------------- | ----------- |
| DKG Manager        | `dkg/src/dkg_manager/mod.rs` | ✅ Complete |
| Epoch Manager      | `dkg/src/epoch_manager.rs`   | ✅ Complete |
| Persistent Storage | `SafetyStorage`              | ✅ Complete |

### Testing

| Component            | Location                             | Tests |
| -------------------- | ------------------------------------ | ----- |
| Rust Unit Tests      | `crates/aptos-dkg/src/ibe/tests.rs`  | 5+    |
| Scalar ElGamal Tests | `scalar_elgamal/transcript.rs`       | 15    |
| Smoke Tests          | `testsuite/smoke-test/src/timelock/` | 30+   |
| TypeScript Tests     | `atomica/timelock-tests/test/`       | 6     |

---

## Two IBE Implementations

### 1. Boneh-Franklin IBE (`ibe/mod.rs`)

Uses BLS12-381 G1/G2 pairing.

| Function                                  | Purpose                          |
| ----------------------------------------- | -------------------------------- |
| `ibe_encrypt(mpk, identity, message)`     | Encrypt using MPK and identity   |
| `ibe_decrypt(dk, ciphertext)`             | Decrypt using decryption key     |
| `derive_decryption_key(msk, identity)`    | Derive DK from MSK and identity  |
| `compute_timelock_identity(id, deadline)` | Compute canonical identity bytes |

### 2. Scalar ElGamal PVSS (`pvss/scalar_elgamal/`)

Dual-output DKG producing scalar shares.

| Function                          | Purpose                                   |
| --------------------------------- | ----------------------------------------- |
| `Transcript::deal()`              | Deal scalar shares via chunked encryption |
| `Transcript::verify()`            | ⚠️ Stub - needs linear pairing check      |
| `Transcript::decrypt_own_share()` | BSGS discrete log decryption              |
| `WeightedTranscript::deal()`      | Weighted protocol wrapper                 |

**Why Two Implementations?**

- Boneh-Franklin: Simpler, proven crypto, cross-language reference
- Scalar ElGamal: Matches DKG architecture, primary production path

---

## Protocol Flow

### Phase 1: Setup (DKG)

1. Genesis → `timelock::initialize()` → emit `StartKeyGenEvent`
2. Validators run DKG (Dual-Output: DAS + Scalar ElGamal)
3. Transcript aggregated, MPK published
4. Validators extract and store secret shares via `process_timelock_key_published()`

### Phase 2: Client Encryption

1. Fetch MPK: `threshold_dsa::get_master_public_key(1)`
2. Compute identity: `Keccak256("timelock_id:{id}:deadline:{deadline}")`
3. Encrypt: `ibe_encrypt(mpk, identity, message)`

### Phase 3: Registration

1. Client calls `timelock::register(deadline)` → gets `timelock_id`
2. Chain stores mapping, emits `TimelockRegisteredEvent`

### Phase 4: Deadline Detection

1. `on_new_block()` checks pending deadlines
2. Emits `RequestRevealEvent(deadline, [timelock_ids])`

### Phase 5: Share Submission (Validators)

1. Validators receive event → `process_request_reveal()`
2. `reveal_one_timelock()`:
   - Retrieve stored secret share
   - Compute identity from timelock_id + deadline
   - Derive `DK_i = SK_i * H(ID)`
   - Submit via `ValidatorTransaction::TimelockShare`

### Phase 6: On-Chain Aggregation

1. Collect shares until threshold (2f+1)
2. Lagrange interpolation: `DK = Σ λ_i * DK_i`
3. Store DK, emit `SecretRevealedEvent`

### Phase 7: Client Decryption

1. Poll `get_decryption_key(timelock_id)`
2. Decrypt: `ibe_decrypt(dk, ciphertext)`

---

## Implementation Status

| Phase   | Description                  | Status         |
| ------- | ---------------------------- | -------------- |
| 0       | Feasibility Test             | ✅ Complete    |
| 1A-1D   | IBE Primitives + MPK Storage | ✅ Complete    |
| 2       | Scalar ElGamal PVSS          | ✅ Complete    |
| 2.0.5   | Unit Tests                   | ✅ Complete    |
| 2.1     | Integration into RealDKG     | ✅ Complete    |
| 2.2     | Aggregation Bug Fix          | ✅ Complete    |
| 1E      | IBE Integration Tests        | ✅ Complete    |
| 2.3     | Transcript Verification      | ✅ Complete    |
| 2.4     | Serialization                | ✅ Complete    |
| 2.5     | Error Handling               | ✅ Complete    |
| 2.6     | DLEQ Proof Verification      | ✅ Complete    |
| 3       | Timelock Registry            | ✅ Complete    |
| 4       | DK Share Submission          | ✅ Complete    |
| 5       | E2E Integration              | ✅ Complete    |
| **5.1** | **Linear Pairing Check**     | 🔲 **PENDING** |

---

## Risk Assessment

### Critical (Must Address)

| Risk                                           | Severity | Location                       | Mitigation               |
| ---------------------------------------------- | -------- | ------------------------------ | ------------------------ |
| **No verification for aggregated transcripts** | 🔴 High  | `scalar_elgamal/transcript.rs` | Implement Phase 5.1      |
| DKG channel panic                              | 🔴 High  | `dkg_manager/mod.rs`           | Fix at commit 77784ca8bf |
| Concurrent DKG sessions                        | 🔴 High  | `epoch_manager.rs`             | HashMap routing          |

### High (Should Prioritize)

| Risk                           | Severity  | Mitigation          |
| ------------------------------ | --------- | ------------------- |
| No Chaum-Pedersen share proofs | 🟠 High   | Add DLOG proofs     |
| Test flakiness                 | 🟠 Medium | Timeout adjustments |

### Immediate Priority: Phase 5.1

**Linear Pairing Check** - Verify aggregated transcripts without per-dealer DLEQ proofs.

**Reference:** Upstream `chunky/transcript.rs:303-313`

---

## Key Files

### Rust Cryptography

| File                                                     | Purpose                      |
| -------------------------------------------------------- | ---------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                        | Boneh-Franklin IBE           |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` | Scalar ElGamal (2100+ lines) |
| `dkg/src/ibe_dkg.rs`                                     | IBE DKG protocol             |
| `dkg/src/epoch_manager.rs`                               | Validator event handling     |

### Move Contracts

| File                 | Purpose                  |
| -------------------- | ------------------------ |
| `timelock.move`      | Registry and aggregation |
| `threshold_dsa.move` | MPK and threshold DSA    |
| `ibe_config.move`    | IBE configuration        |

### Tests

| File                                 | Purpose          |
| ------------------------------------ | ---------------- |
| `testsuite/smoke-test/src/timelock/` | 30+ smoke tests  |
| `atomica/timelock-tests/test/`       | TypeScript tests |

---

## Testing Strategy

### Test Coverage

| Layer              | Tests       | Status             |
| ------------------ | ----------- | ------------------ |
| IBE Primitives     | 5+          | ✅ Complete        |
| Scalar ElGamal     | 15          | ✅ Complete        |
| Move Contracts     | Integration | ✅ Complete        |
| Validator Services | Smoke       | ✅ Complete        |
| E2E Flow           | Partial     | ⚠️ Needs Phase 5.1 |
| Cross-Language     | 6           | ✅ Complete        |

### Key Test Files

- `crates/aptos-dkg/src/ibe/tests.rs`
- `testsuite/smoke-test/src/timelock/mod.rs`
- `testsuite/smoke-test/src/timelock/deadline_reveal.rs`
- `testsuite/smoke-test/src/timelock/register_and_query.rs`
- `atomica/timelock-tests/test/cross-lang-ibe-verification.test.ts`

---

## Success Criteria

### Production Ready When:

- [ ] Phase 5.1 (Linear Pairing Check) complete
- [ ] No `TODO` comments in security-critical paths
- [ ] Chaum-Pedersen proofs for share verification
- [ ] Security review completed
- [ ] E2E smoke test passes
- [ ] Cross-language serialization verified

### Full Project Complete When:

- [ ] All phases 0-5 complete
- [ ] No regressions in randomness tests
- [ ] Documentation complete
- [ ] Deployment guide written

---

## Related Documentation

| Document                                                                                         | Purpose               |
| ------------------------------------------------------------------------------------------------ | --------------------- |
| [definitions.md](definitions.md)                                                                 | Core terminology      |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md)                                         | Architecture decision |
| [technical/chunked-elgamal-scalar-generation.md](technical/chunked-elgamal-scalar-generation.md) | Technical deep dive   |

---

## Changelog

- **v3.0** (Jan 20, 2026): Consolidated to single source of truth, re-evaluated with risk assessment
- **v2.13** (Jan 19, 2026): Last multi-document version
- **v1.0-v2.0**: Development iterations (see git history)
