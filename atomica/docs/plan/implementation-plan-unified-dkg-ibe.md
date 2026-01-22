# Implementation Plan: Unified DKG for Randomness + IBE

**Version:** 2.15
**Date:** January 22, 2026
**Branch:** `timelock-elgamal-pvss`
**Status:** Phase 5.2 (IBE DK Reconstruction) COMPLETE
**Reference:** [ADR-001: Dual Output DKG](adr-001-dual-output-dkg.md)
**Related Docs:**

- [IBE Implementation Summary](../ibe-summary.md)
- [Technical: Chunked ElGamal](technical/chunked-elgamal-scalar-generation.md)
- [Definitions](definitions.md)
- [Timelock IBE ElGamal Plan](timelock-ibe-elgamal-plan.md)

---

## Overview: What We're Building

This plan implements a **dual-output DKG** that produces two types of key material in a single round:

| PVSS Scheme                | Output Type    | Consumer | Purpose             |
| -------------------------- | -------------- | -------- | ------------------- |
| **DAS PVSS**               | `G1Projective` | WVUF     | On-chain randomness |
| **Chunked Lifted ElGamal** | `Scalar`       | IBE      | Timelock encryption |

### Key Design Decisions

1. **Chunked Lifted ElGamal for IBE scalars** - We use Chunked Lifted ElGamal PVSS to produce scalar shares. Each 256-bit scalar is split into 16 chunks of 16 bits, encrypted with lifted ElGamal, and decrypted using BSGS discrete log.

2. **DAS PVSS for randomness only** - The existing DAS PVSS continues unchanged for WVUF/randomness. We do NOT use DAS output for IBE.

3. **DKG produces ephemeral IBE keys** - The IBE master secret comes from the DKG, NOT from validator BLS keys directly.

4. **Same secret, two representations** - Both PVSS schemes share the same `InputSecret`. The scalar `a` is dealt twice: once as G1 shares (DAS) and once as scalar shares (Chunked ElGamal).

---

## Cryptographic Objects: Precise Definitions

### Key Material Taxonomy

#### Layer 1: Master Key Pair (DKG Output)

| Name                  | Symbol | Type        | Size     | Created By        | Visibility              | On-Chain? |
| --------------------- | ------ | ----------- | -------- | ----------------- | ----------------------- | --------- |
| **Master Secret Key** | MSK    | Scalar (Fr) | 32 bytes | DKG (distributed) | SECRET - never revealed | ❌ NO     |
| **Master Public Key** | MPK    | G2 point    | 96 bytes | DKG               | PUBLIC                  | ✅ YES    |

**Relationship:** `MPK = MSK × g₂`

**Lifecycle:**

- MSK is created via distributed key generation (DKG)
- MSK never exists as a single value on any machine
- MSK is distributed as PVSS shares to validators
- MPK is published on-chain after DKG completes

---

#### Layer 2: PVSS Secret Shares (Validator Private State)

| Name                      | Symbol | Type          | Size              | Held By                  | Visibility              | On-Chain? |
| ------------------------- | ------ | ------------- | ----------------- | ------------------------ | ----------------------- | --------- |
| **PVSS Secret Share**     | s_i    | Vec\<Scalar\> | 32 bytes × weight | Validator i              | SECRET - off-chain only | ❌ NO     |
| **PVSS Public Key Share** | pk_i   | Vec\<G2\>     | 96 bytes × weight | Public (from transcript) | PUBLIC                  | ❌ NO     |

**Relationship:**

```
pk_i[j] = s_i[j] × g₂                    (for each virtual player j)
sum_i(Lagrange_i × s_i[0]) = MSK         (reconstruction)
sum_i(Lagrange_i × pk_i[0]) = MPK        (verification)
```

**Lifecycle:**

1. DKG creates PVSS transcript with encrypted shares
2. Each validator decrypts their `s_i` shares using their private key
3. PVSS transcript contains public key shares `pk_i` for verification

**Security properties:**

- `s_i` values MUST remain secret
- Knowledge of threshold `t` shares allows MSK reconstruction
- `pk_i` provides verifiability without revealing `s_i`

---

#### Layer 3: IBE Decryption Key Shares (On-Chain Submissions)

| Name         | Symbol     | Type     | Size     | Submitted By | Visibility | On-Chain? |
| ------------ | ---------- | -------- | -------- | ------------ | ---------- | --------- |
| **DK Share** | dk_share_i | G1 point | 48 bytes | Validator i  | PUBLIC     | ✅ YES    |

**Computation (off-chain by validator):**

```
H = hash_to_G1(identity)
dk_share_i = sum_j(s_i[j] × H) = (sum_j s_i[j]) × H
```

---

#### Layer 4: Reconstructed Decryption Key (Final Output)

| Name               | Symbol | Type     | Size     | Created By      | Visibility             | On-Chain? |
| ------------------ | ------ | -------- | -------- | --------------- | ---------------------- | --------- |
| **Decryption Key** | DK     | G1 point | 48 bytes | Native function | PUBLIC after threshold | ✅ YES    |

**Reconstruction:**

```
DK = sum_i(Lagrange_i(validators) × dk_share_i)
   = MSK × H
```

**Verification:**

```
e(DK, g₂) =? e(H, MPK)
```

---

### Architecture Diagram

```
InputSecret (scalar a)
        │
        ├─────────────────────┬────────────────────────────┐
        │                     │                            │
        ▼                     ▼                            ▼
   ┌─────────┐      ┌─────────────────────┐      ┌─────────────────┐
   │DAS PVSS │      │Chunked Lifted       │      │   MPK = g2^a    │
   │         │      │ElGamal PVSS         │      │  (on-chain)     │
   └────┬────┘      └──────────┬──────────┘      └─────────────────┘
        │                      │
        ▼                      ▼
   G1 shares              Scalar shares
   (WVUF/randomness)      (IBE DK reconstruction)
        │                      │
        ▼                      ▼
   WVUF/Randomness        IBE Decryption Key
                           dk = H(identity)^scalar
```

---

## Changelog

- **v2.15** (Jan 22, 2026): **Documentation consolidation**
  - Merged key material definitions into master plan
  - Created consolidated `ibe-summary.md` reference
  - Deleted outdated analysis docs
- **v2.14** (Jan 22, 2026): **Phase 5.2 (IBE DK Reconstruction) COMPLETE**
  - Implemented unified `reconstruct_ibe_dk()` in Rust SDK
  - Implemented native function `reconstruct_ibe_dk_internal()` in Move VM
  - Updated Move API `ibe.move` with new signature
  - All 39 IBE/DKG tests pass

---

## Current Status Summary

| Phase     | Description                  | Status      | Priority |
| --------- | ---------------------------- | ----------- | -------- |
| 0         | Feasibility Test             | ✅ COMPLETE | -        |
| 1A-1D     | IBE Primitives + MPK Storage | ✅ COMPLETE | -        |
| 2         | Scalar ElGamal PVSS          | ✅ COMPLETE | -        |
| 2.0.5-2.6 | Testing, verification, DLEQ  | ✅ COMPLETE | -        |
| 3         | Timelock Registry            | ✅ COMPLETE | -        |
| 4         | DK Share Submission          | ✅ COMPLETE | -        |
| 5         | E2E Integration              | ✅ COMPLETE | -        |
| 5.2       | IBE DK Reconstruction        | ✅ COMPLETE | -        |

### Current Blockers for Production

| Issue                            | Impact            | Status     |
| -------------------------------- | ----------------- | ---------- |
| `mpk_encrypt_decrypt` smoke test | Not yet run       | 🔲 Pending |
| `timelock_e2e` smoke test        | Not yet run       | 🔲 Pending |
| Security review                  | Not yet completed | 🔲 Pending |

---

## What's Been Implemented

### Phase 5.2: IBE DK Reconstruction ✅ COMPLETE

This phase implements the unified IBE decryption key reconstruction path:

```
Input: (validator_indices, scalar_shares, weights, total_weight, identity)
       │
       ▼
1. Wrap scalar_shares in DealtSecretKeyShare format
       │
       ▼
2. Delegate to WeightedConfig::new() + DealtSecretKey::reconstruct()
       │
       ▼
3. Derive DK = H(identity)^secret
       │
       ▼
Output: G1Affine (the reconstructed decryption key)
```

### API Signature

**Rust SDK** (`crates/aptos-dkg/src/ibe/mod.rs`):

```rust
pub fn reconstruct_ibe_dk(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],  // Per-validator, per-virtual-player
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> G1Affine
```

**Move VM** (`ibe.move`):

```move
public fun reconstruct_ibe_dk<G1>(
    validator_indices: vector<u64>,
    scalar_shares: vector<vector<u8>>,  // Nested: validator -> virtual_player
    weights: vector<u64>,
    threshold: u64,
    total_weight: u64,
    identity: vector<u8>,
): crypto_algebra::Element<G1>
```

### Files Modified

| File                                                              | Purpose                               |
| ----------------------------------------------------------------- | ------------------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | `reconstruct_ibe_dk()` implementation |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function                       |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move API                              |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move`    | Timelock registry                     |
| `crates/aptos-dkg/src/ibe/golden_vectors.rs`                      | Golden vector generation              |

### Tests

| File                                                              | Tests | Status  |
| ----------------------------------------------------------------- | ----- | ------- |
| `crates/aptos-dkg/src/ibe/tests.rs`                               | 28    | ✅ PASS |
| `aptos-move/framework/aptos-framework/tests/ibe_native_test.move` | 11    | ✅ PASS |

---

## Design Decisions

### Decision: Scalar Share Reconstruction (Jan 22, 2026)

We implemented scalar share reconstruction with framework delegation, NOT G1-based reconstruction.

**Rationale:**

1. **Security** - Native functions should not implement crypto
2. **Consistency** - Move VM uses same crypto as Rust SDK
3. **Simplicity** - Framework handles virtual player expansion

---

## Files Reference

### Core Implementation

| File                                                              | Purpose                 |
| ----------------------------------------------------------------- | ----------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | Rust SDK IBE primitives |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function         |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move API                |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move`    | Timelock registry       |

### Tests

| File                                                              | Purpose                  |
| ----------------------------------------------------------------- | ------------------------ |
| `crates/aptos-dkg/src/ibe/tests.rs`                               | Rust SDK tests           |
| `aptos-move/framework/aptos-framework/tests/ibe_native_test.move` | Move tests               |
| `crates/aptos-dkg/src/ibe/golden_vectors.rs`                      | Golden vector generation |

### Fixtures

| File                                                                           | Format |
| ------------------------------------------------------------------------------ | ------ |
| `atomica/golden_vectors/ibe_golden_vectors.json`                               | JSON   |
| `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move` | Move   |
