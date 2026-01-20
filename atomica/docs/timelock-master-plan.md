# Atomica Timelock Implementation: Consolidated Plan

**Version:** 3.0  
**Date:** January 20, 2026  
**Branch:** `timelock-vpss`  
**Status:** RE-EVALUATED with risk assessment

---

## Executive Summary

This document consolidates the Atomica timelock encryption implementation into a single reference. It synthesizes findings from:

- 10+ markdown documents (~2,575 lines)
- Source code analysis (`ibe/mod.rs`, `scalar_elgamal/`, `epoch_manager.rs`)
- Smoke test suite (30+ tests)
- Cross-language verification tests

**Key Finding:** The implementation is **largely complete** but uses a **hybrid architecture** with two parallel IBE approaches. Critical next step is the **Linear Pairing Check (Phase 5.1)** for aggregated transcript verification.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Two IBE Implementations](#two-ibe-implementations)
3. [Current Implementation Status](#current-implementation-status)
4. [Risk Assessment & Priorities](#risk-assessment--priorities)
5. [Phase Roadmap](#phase-roadmap)
6. [Testing Strategy](#testing-strategy)
7. [Key Files Reference](#key-files-reference)
8. [Related Documentation](#related-documentation)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ATOMICA TIMELOCK ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      DISTRIBUTED KEY GENERATION                      │   │
│  │                                                                       │   │
│  │   InputSecret (scalar a)                                             │   │
│  │         │                                                            │   │
│  │         ├─────────────────────┬──────────────────────────────┐      │   │
│  │         │                     │                              │      │   │
│  │         ▼                     ▼                              ▼      │   │
│  │   ┌─────────────┐     ┌────────────────────┐     ┌─────────────────┐│   │
│  │   │  DAS PVSS   │     │ Scalar ElGamal PVSS│     │   Master Public ││   │
│  │   │   (G1)      │     │    (Scalar)        │     │     Key (G2)    ││   │
│  │   └──────┬──────┘     └─────────┬──────────┘     └─────────────────┘│   │
│  │          │                      │                                       │   │
│  │          ▼                      ▼                                       │   │
│  │   WVUF/Randomness          IBE Secrets                                  │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                          │
│                                    ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         VALIDATOR LAYER                               │   │
│  │                                                                       │   │
│  │   - Store encrypted shares from DKG                                  │   │
│  │   - On deadline: decrypt own shares                                  │   │
│  │   - Derive DK_share = scalar_share × H(identity)                     │   │
│  │   - Submit ValidatorTransaction::TimelockShare                       │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                          │
│                                    ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        ON-CHAIN LAYER (Move)                         │   │
│  │                                                                       │   │
│  │   timelock.move:                                                     │   │
│  │   - register(deadline) → assign timelock_id                          │   │
│  │   - on_new_block() → emit RequestRevealEvent                        │   │
│  │   - publish_decryption_key_share() → collect and aggregate          │   │
│  │                                                                       │   │
│  │   threshold_dsa.move:                                                │   │
│  │   - aggregate_timelock_shares() → Lagrange interpolation            │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                          │
│                                    ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          CLIENT LAYER                                 │   │
│  │                                                                       │   │
│  │   Encrypt:                                                           │   │
│  │   1. Fetch MPK from chain                                            │   │
│  │   2. Compute identity = hash(timelock_id, deadline)                  │   │
│  │   3. Encrypt(MPK, identity, message)                                 │   │
│  │                                                                       │   │
│  │   Decrypt (after deadline):                                          │   │
│  │   1. Poll get_decryption_key(timelock_id)                            │   │
│  │   2. Decrypt(DK, ciphertext)                                         │   │
│  │                                                                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Two IBE Implementations

### 1. Boneh-Franklin IBE (`crates/aptos-dkg/src/ibe/`)

**Type:** Pairing-based IBE using BLS12-381 G1/G2

| Component                     | Status      | Description                        |
| ----------------------------- | ----------- | ---------------------------------- |
| `ibe_encrypt()`               | ✅ Complete | Encrypt with MPK (G2) and identity |
| `ibe_decrypt()`               | ✅ Complete | Decrypt with DK (G1)               |
| `derive_decryption_key()`     | ✅ Complete | DK = MSK × H(identity)             |
| `compute_timelock_identity()` | ✅ Complete | Keccak256 hash                     |

**Identity Format:**

```
Keccak256("timelock_id:{id}:deadline_timestamp_microseconds:{deadline}")
```

**Use Case:**

- Reference implementation
- TypeScript cross-language verification
- Educational/simplified deployment

### 2. Scalar ElGamal PVSS (`crates/aptos-dkg/src/pvss/scalar_elgamal/`)

**Type:** Dual-output DKG producing scalar shares for IBE

| Component                         | Status      | Description                                   |
| --------------------------------- | ----------- | --------------------------------------------- |
| `Transcript::deal()`              | ✅ Complete | Deal scalar shares via chunked encryption     |
| `Transcript::verify()`            | ⚠️ Stub     | DLEQ proofs implemented, needs linear pairing |
| `Transcript::decrypt_own_share()` | ✅ Complete | BSGS discrete log decryption                  |
| `WeightedTranscript`              | ✅ Complete | Weighted protocol wrapper                     |
| **Linear Pairing Check**          | 🔲 PENDING  | Phase 5.1 - Critical                          |

**Design:**

- Splits 256-bit scalars into 16× 16-bit chunks
- Uses BSGS (Baby-Step Giant-Step) for efficient decryption
- Correlated randomness for proper aggregation

**Why Two Implementations?**

1. **Boneh-Franklin**: Simpler, proven crypto, but requires G2 public key
2. **Scalar ElGamal**: Matches existing DKG architecture, produces scalar output directly

**Current Direction:** Scalar ElGamal is the primary implementation for production. Boneh-Franklin serves as reference and cross-language verification target.

---

## Current Implementation Status

### By Component

| Component                    | Status      | Lines | Tests       |
| ---------------------------- | ----------- | ----- | ----------- |
| **IBE Cryptography**         | ✅ Complete | ~415  | 5+          |
| **Scalar ElGamal PVSS**      | ✅ Complete | ~2100 | 15          |
| **IBE DKG Protocol**         | ✅ Complete | ~1027 | Unit        |
| **DKG Manager**              | ✅ Complete | ~600  | -           |
| **Epoch Manager**            | ✅ Complete | ~1268 | -           |
| **Move: timelock.move**      | ✅ Complete | ~400  | Integration |
| **Move: threshold_dsa.move** | ✅ Complete | ~300  | Integration |
| **Move: ibe_config.move**    | ✅ Complete | ~716  | -           |
| **TypeScript SDK**           | ✅ Complete | ~1000 | 6 tests     |
| **Smoke Tests**              | ✅ Complete | -     | 30+ tests   |

### By Phase (from `implementation-plan-unified-dkg-ibe.md`)

| Phase   | Description                  | Status      |
| ------- | ---------------------------- | ----------- |
| 0       | Feasibility Test             | ✅ Complete |
| 1A-1D   | IBE Primitives + MPK Storage | ✅ Complete |
| 2       | Scalar ElGamal PVSS          | ✅ Complete |
| 2.0.5   | Unit Tests                   | ✅ Complete |
| 2.1     | Integration into RealDKG     | ✅ Complete |
| 2.2     | Aggregation Bug Fix          | ✅ Complete |
| 1E      | IBE Integration Tests        | ✅ Complete |
| 2.3     | Transcript Verification      | ✅ Complete |
| 2.4     | Serialization                | ✅ Complete |
| 2.5     | Error Handling               | ✅ Complete |
| 2.6     | DLEQ Proof Verification      | ✅ Complete |
| 3       | Timelock Registry            | ✅ Complete |
| 4       | DK Share Submission          | ✅ Complete |
| 5       | E2E Integration              | ✅ Complete |
| **5.1** | **Linear Pairing Check**     | 🔲 PENDING  |

---

## Risk Assessment & Priorities

### Critical Risks (Must Address)

| Risk                                           | Severity | Likelihood | Location                       | Mitigation                               |
| ---------------------------------------------- | -------- | ---------- | ------------------------------ | ---------------------------------------- |
| **No verification for aggregated transcripts** | 🔴 High  | Medium     | `scalar_elgamal/transcript.rs` | Implement Phase 5.1 linear pairing check |
| **DKG channel panic on termination**           | 🔴 High  | Low        | `dkg_manager/mod.rs`           | Fix at commit 77784ca8bf                 |
| **Concurrent DKG session conflicts**           | 🔴 High  | Low        | `epoch_manager.rs`             | Per-epoch HashMap routing                |

### High Risks (Should Prioritize)

| Risk                                       | Severity  | Likelihood | Location        | Mitigation                |
| ------------------------------------------ | --------- | ---------- | --------------- | ------------------------- |
| **No cryptographic share verification**    | 🟠 High   | Low        | Move contracts  | Add Chaum-Pedersen proofs |
| **Test flakiness (timing)**                | 🟠 Medium | High       | Smoke tests     | Timeout adjustments       |
| **Validator share persistence edge cases** | 🟠 Medium | Low        | `SafetyStorage` | Additional testing        |

### Immediate Priorities

1. **Phase 5.1: Linear Pairing Check** (HIGHEST PRIORITY)
   - Provides verification for aggregated transcripts
   - Enables encryption correctness without per-dealer DLEQ proofs
   - Reference: Upstream `chunky/transcript.rs:303-313`

2. **Share Verification Proofs** (HIGH)
   - Chaum-Pedersen DLOG proofs for validator shares
   - Prevents malicious validators from submitting random points

3. **Integration Test Coverage** (MEDIUM)
   - Full E2E encrypt→wait→decrypt flow
   - Validator restart during DKG

---

## Phase Roadmap

### Phase 5.1: Linear Pairing Check (NOW)

**Goal:** Add verification for aggregated transcripts

**Tasks:**

1. Derive linear equation for scalar ElGamal scheme
2. Implement `verify_linear_pairing_check()` in `transcript.rs`
3. Integrate into `Transcript::verify()` after aggregation
4. Add unit tests

**Reference:** Upstream implementation at `chunky/transcript.rs:303-313`

### Phase 5.2: Security Hardening

- Add Chaum-Pedersen proofs for share verification
- Implement rate limiting on share submission
- Add key recovery mechanism

### Phase 6: Production Readiness

- Performance testing with 100+ validators
- Security audit
- Mainnet deployment checklist

---

## Testing Strategy

### Test Coverage Matrix

| Layer                  | Test Type         | Coverage                                | Status      |
| ---------------------- | ----------------- | --------------------------------------- | ----------- |
| **Cryptography**       | Unit Tests        | IBE primitives, scalar ElGamal          | ✅ Complete |
| **DKG Protocol**       | Unit Tests        | Deal, verify, aggregate, decrypt        | ✅ Complete |
| **Move Contracts**     | Integration Tests | Registry, share collection, aggregation | ✅ Complete |
| **Validator Services** | Smoke Tests       | Event handling, share submission        | ✅ Complete |
| **E2E Flow**           | Smoke Tests       | Full encrypt→wait→decrypt               | ⚠️ Partial  |
| **Cross-Language**     | TypeScript        | Fp12 serialization compatibility        | ✅ Complete |

### Key Test Files

| File                                                      | Purpose                   |
| --------------------------------------------------------- | ------------------------- |
| `crates/aptos-dkg/src/ibe/tests.rs`                       | IBE unit tests            |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`  | Scalar ElGamal tests (15) |
| `testsuite/smoke-test/src/timelock/`                      | 30+ smoke tests           |
| `testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs` | IBE MPK tests             |
| `atomica/timelock-tests/test/`                            | TypeScript tests          |

---

## Key Files Reference

### Rust Cryptography

| File                                                            | Purpose                       |
| --------------------------------------------------------------- | ----------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                               | Boneh-Franklin IBE primitives |
| `crates/aptos-dkg/src/ibe/ciphertext.rs`                        | IBE ciphertext structure      |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs`               | Scalar ElGamal exports        |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`        | Core PVSS (2100+ lines)       |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` | Weighted wrapper              |
| `dkg/src/ibe_dkg.rs`                                            | IBE DKG protocol              |
| `dkg/src/dkg_manager/mod.rs`                                    | DKG lifecycle management      |
| `dkg/src/epoch_manager.rs`                                      | Validator event handling      |

### Move Contracts

| File                                                 | Purpose                           |
| ---------------------------------------------------- | --------------------------------- |
| `aptos-move/.../sources/timelock.move`               | Timelock registry and aggregation |
| `aptos-move/.../sources/threshold_dsa.move`          | Threshold DSA and MPK             |
| `aptos-move/.../sources/ibe_config.move`             | IBE configuration                 |
| `aptos-move/aptos-vm/src/validator_txns/timelock.rs` | Timelock transaction handler      |

### Tests

| File                                                              | Purpose                   |
| ----------------------------------------------------------------- | ------------------------- |
| `testsuite/smoke-test/src/timelock/mod.rs`                        | Main timelock test module |
| `testsuite/smoke-test/src/timelock/deadline_reveal.rs`            | Deadline and reveal tests |
| `testsuite/smoke-test/src/timelock/register_and_query.rs`         | Registration tests        |
| `testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs`         | IBE MPK tests             |
| `atomica/timelock-tests/test/cross-lang-ibe-verification.test.ts` | Cross-language tests      |

---

## Related Documentation

| Document                                                                                         | Purpose                                  |
| ------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| [implementation-plan-unified-dkg-ibe.md](implementation-plan-unified-dkg-ibe.md)                 | Detailed phase-by-phase plan (v2.13)     |
| [definitions.md](definitions.md)                                                                 | Core terminology and concepts            |
| [adr-001-dual-output-dkg.md](adr-001-dual-output-dkg.md)                                         | Architecture decision record             |
| [timelock-evaluation.md](timelock-evaluation.md)                                                 | Implementation evaluation (Jan 20, 2026) |
| [technical/chunked-elgamal-scalar-generation.md](technical/chunked-elgamal-scalar-generation.md) | Scalar generation details                |
| [developer-guides/testing.md](developer-guides/testing.md)                                       | Testing guide                            |
| [developer-guides/smoke-test-debugging.md](developer-guides/smoke-test-debugging.md)             | Debugging guide                          |

---

## Changelog

- **v3.0** (Jan 20, 2026): Consolidated multiple documents, added risk assessment, re-evaluated priorities based on code analysis
- **v2.13** (Jan 19, 2026): Last update to implementation-plan-unified-dkg-ibe.md
- **v2.0** (Jan 18, 2026): Major refactoring for scalar ElGamal
- **v1.0** (Jan 17, 2026): Initial documentation

---

## Success Criteria

### Production Ready When:

- [ ] Phase 5.1 (Linear Pairing Check) complete
- [ ] No `TODO` comments in security-critical paths
- [ ] Chaum-Pedersen proofs for share verification
- [ ] Security review completed
- [ ] E2E smoke test passes
- [ ] Cross-language serialization verified
- [ ] Performance benchmarks with 100+ validators

### Full Project Complete When:

- [ ] All phases 0-5 complete
- [ ] No regressions in randomness tests
- [ ] Documentation complete
- [ ] Deployment guide written
