# Implementation Plan: Unified DKG for Randomness + IBE

**Version:** 2.5
**Date:** January 19, 2026
**Branch:** feature/scalar-chunked-elgamal
**Status:** Core Implementation Complete, Security Hardening Required
**Reference:** [ADR-001: Dual Output DKG](adr-001-dual-output-dkg.md)

---

## Overview: What We're Building

This plan implements a **dual-output DKG** that produces two types of key material in a single round:

| PVSS Scheme | Output Type | Consumer | Purpose |
|-------------|-------------|----------|---------|
| **DAS PVSS** | `G1Projective` | WVUF | On-chain randomness |
| **Chunked Lifted ElGamal** | `Scalar` | IBE | Timelock encryption |

### Key Design Decisions

1. **Chunked Lifted ElGamal for IBE scalars** - We use Chunked Lifted ElGamal PVSS (not plain ElGamal, not DAS) to produce scalar shares. Each 256-bit scalar is split into 16 chunks of 16 bits, encrypted with lifted ElGamal, and decrypted using BSGS discrete log.

2. **DAS PVSS for randomness only** - The existing DAS PVSS continues unchanged for WVUF/randomness. We do NOT use DAS output for IBE.

3. **DKG produces ephemeral IBE keys** - The IBE master secret comes from the DKG, NOT from validator BLS keys directly. This provides forward secrecy and key separation.

4. **Same secret, two representations** - Both PVSS schemes share the same `InputSecret`. The scalar `a` is dealt twice: once as G1 shares (DAS) and once as scalar shares (Chunked ElGamal).

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
   (main transcript)      (scalar transcript)
        │                      │
        ▼                      ▼
   WVUF/Randomness        IBE Decryption Key
   e(share, H(input))     dk = H(identity)^scalar
```

---

## Changelog

- **v2.5** (Jan 19, 2026): **Code review and phase reordering.** Added comprehensive code review findings. Identified critical gaps: (1) verify() not implemented, (2) serialization stubs return empty, (3) silent BSGS failure. Reordered phases to prioritize security hardening (2.3-2.5) before feature work (Phases 3-5). Added Phase 2.3 (Verification), Phase 2.4 (Serialization), Phase 2.5 (Error Handling).
- **v2.4** (Jan 19, 2026): **Documentation alignment with ADR-001.** Fixed Decision 2 to show correct Chunked Lifted ElGamal struct (was showing incorrect non-chunked design). Added Overview section clarifying: (1) Chunked Lifted ElGamal for IBE scalars, (2) DAS PVSS for randomness only, (3) DKG produces ephemeral keys not using BLS directly.
- **v2.3** (Jan 19, 2026): Fixed scalar ElGamal aggregation bug. `chunks_to_scalar` incorrectly reconstructed scalars for aggregated transcripts. Added proper field arithmetic. All 15 unit tests and `randomness_correctness` smoke test pass.
- **v2.2** (Jan 18, 2026): Marked Phase 2.1 complete. Next is Phase 1E (mpk_encrypt_decrypt smoke test).
- **v2.1** (Jan 18, 2026): Added Phase 2.0.5 (Unit Tests) before RealDKG integration. Added CI job requirements for regression testing.
- **v2.0** (Jan 18, 2026): Major update reflecting completed Phase 2 Scalar ElGamal implementation. Documented decisions made during development. Reorganized phases to reflect actual implementation order.
- **v1.5** (Jan 17, 2026): Reordered phases. Phase 2 is now Dual-Output DKG (ADR-001).
- **v1.4** (Jan 17, 2026): Reordered phases to prioritize functionality over refactoring.
- **v1.3** (Jan 17, 2026): Marked Phase 0 and Phase 1A-1C complete.

---

## Current Status Summary

| Phase | Description                         | Status      | Priority |
| ----- | ----------------------------------- | ----------- | -------- |
| 0     | Feasibility Test                    | ✅ COMPLETE | - |
| 1A-1D | IBE Primitives + MPK Storage        | ✅ COMPLETE | - |
| 2     | Scalar ElGamal PVSS                 | ✅ COMPLETE | - |
| 2.0.5 | Unit Tests for Scalar ElGamal       | ✅ COMPLETE | - |
| 2.1   | Integration into RealDKG + DKGTrait | ✅ COMPLETE | - |
| 2.2   | Aggregation Bug Fix                 | ✅ COMPLETE | - |
| 1E    | IBE Integration Tests               | ✅ COMPLETE | - |
| **2.3** | **Transcript Verification**       | 🔴 PENDING  | **CRITICAL** |
| **2.4** | **Serialization Implementation**  | 🟡 PENDING  | **HIGH** |
| **2.5** | **Error Handling Hardening**      | 🟡 PENDING  | **HIGH** |
| 3     | Timelock Registry                   | 🔲 PENDING  | Medium |
| 4     | DK Share Submission                 | 🔲 PENDING  | Medium |
| 5     | E2E Integration                     | 🔲 PENDING  | Medium |

### Current Blockers for Production

| Issue | Location | Impact | Status |
|-------|----------|--------|--------|
| `verify()` returns Ok without checks | `transcript.rs:638-659` | Malicious dealer can submit invalid transcript | 🔴 BLOCKING |
| `verify_transcript()` skips scalar | `real_dkg/mod.rs:464-487` | No verification of scalar transcript | 🔴 BLOCKING |
| `get_ibe_master_public_key()` returns empty | `real_dkg/mod.rs:638-651` | On-chain MPK unusable | 🟡 BLOCKING |
| `get_scalar_secret_share()` returns None | `real_dkg/mod.rs:662-677` | Scalar shares not serializable | 🟡 BLOCKING |
| BSGS failure returns 0 silently | `transcript.rs:866-873` | Corrupted shares without error | 🟡 HIGH |

---

## Development Decisions Log

### Decision 1: Dual-Output DKG Architecture (ADR-001)

**Date:** January 17, 2026
**Context:** IBE requires scalar secrets, but existing DKG produces G1 elements for WVUF.

**Options Considered:**

1. Modify DKG to output scalar only → Rejected (breaks WVUF)
2. Modify IBE to accept G1 → Rejected (novel crypto)
3. Hash G1 to scalar → Rejected (breaks MPK relationship)
4. Switch to Chunky PVSS → Rejected (lacks weighted support)
5. **Dual-Output DKG** → Selected

**Decision:** Extend RealDKG to produce both G1 (DAS PVSS) and Scalar (ElGamal PVSS) shares in a single round using the same InputSecret.

**Rationale:** Meets all constraints (no WVUF changes, no novel crypto, single DKG round).

### Decision 2: Chunked Lifted ElGamal PVSS Design

**Date:** January 17-18, 2026
**Context:** Need a PVSS scheme that outputs scalar shares for IBE.

**Design Choices:**

- **Chunked Lifted ElGamal** - Split each 256-bit scalar share into 16 chunks of 16 bits
- Encrypt chunks (not full shares) to enable efficient discrete log via BSGS
- Use G2 for polynomial commitments (`V` vector) to enable verification
- Use G1 for ciphertexts matching encryption key group
- Correlated randomness: `Σ r_j · 2^{16j} = 0` ensures proper aggregation

**Chunked Encryption Formula:**

For validator `i` and chunk `j`:
```
C_{i,j} = G · u_{i,j} + PK_i · r_j
```
Where:
- `u_{i,j}` is the j-th 16-bit chunk of validator i's share
- `r_j` is shared randomness for chunk index j (same for all validators)
- `R_j = G · r_j` is the ephemeral key for chunk j

**Actual Implementation:**

```rust
// crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs
pub struct Transcript {
    /// Signatures of Knowledge proving dealer knows the secret
    soks: Vec<SoK<G2Projective>>,
    /// Commitment to first randomness coefficient r_0
    hat_w: G2Projective,
    /// Polynomial commitments V_i = G2^{f(i)} (n+1 elements)
    V: Vec<G2Projective>,
    /// Ephemeral keys R_j = G^{r_j} for each chunk j=0..15 (16 elements)
    ephemeral_keys: Vec<G1Projective>,
    /// Ciphertexts C_{i,j} - outer: validators (n), inner: chunks (16)
    ciphertexts: Vec<Vec<G1Projective>>,
    /// Pre-computed aggregate for threshold reconstruction
    encrypted_aggregate: Vec<G1Projective>,
}
```

**Key Difference from Plain ElGamal:** The chunking enables efficient decryption. Plain ElGamal encryption of a 256-bit value would require solving a discrete log in a space of 2^256 (infeasible). With 16-bit chunks, we solve 16 discrete logs in a space of 2^16 each (trivial with BSGS in ~256 operations per chunk).

### Decision 3: Weighted Protocol via Duplication

**Date:** January 18, 2026
**Context:** Need weighted threshold support for validator stake proportions.

**Approach:** Duplicate encryption keys proportional to validator weight, then delegate to unweighted `Transcript::deal()`.

```rust
// weighted_protocol.rs
fn to_weighted_encryption_keys(sc, eks) -> Vec<EncryptPubKey> {
    // Expand: validator with weight 3 gets 3 copies of their ek
    for (player_id, ek) in eks {
        for _ in 0..sc.get_player_weight(player) {
            duplicated_eks.push(ek.clone());
        }
    }
}
```

**Rationale:** Matches DAS weighted protocol approach. Simpler than implementing weighted Lagrange directly.

### Decision 4: Verify Stub Acceptable for Initial Integration

**Date:** January 18, 2026
**Context:** Full DLEQ verification is complex; need to make progress.

**Decision:** `verify()` returns error stub for now. Acceptable because:

1. DKG currently relies on honest majority assumption
2. Verification can be added incrementally
3. Smoke tests validate correctness via encrypt/decrypt roundtrip

**TODO:** Implement full verification before mainnet.

### Decision 5: Scalar Reconstruction Bug Fix

**Date:** January 19, 2026
**Context:** `randomness_correctness` smoke test was failing with discrete log solver errors.

**Root Cause:** The `chunks_to_scalar()` function incorrectly reconstructed scalars from chunks. It assumed chunk values fit in u16 range and used byte conversion. For aggregated transcripts (multiple dealers), the decrypted value is the sum of chunks from all dealers, which can exceed u16 (e.g., 3 dealers × 65535 = 196605).

**Fix Applied:**

1. Replaced byte conversion with proper field arithmetic:

   ```rust
   fn chunks_to_scalar(chunks: &[u16]) -> Scalar {
       let radix = Scalar::from(1u64 << CHUNK_BIT_SIZE);
       let mut result = Scalar::from(0u64);
       let mut multiplier = Scalar::from(1u64);
       for &chunk in chunks {
           result += Scalar::from(chunk as u64) * multiplier;
           multiplier *= radix;
       }
       result
   }
   ```

2. Expanded BSGS search range for aggregated transcripts:
   - Single dealer: `[0, 2^16)`
   - Multiple dealers: `[0, num_dealers × 2^16)`

3. Fixed type aliases in `TranscriptTrait` impl (`g1::` → `scalar::`)

4. Added missing `Mul` trait import for blstrs operations

**Test Results:**

- All 15 scalar ElGamal unit tests: ✅ PASS
- `randomness_correctness` smoke test: ✅ PASS (260s)

**Commit:** `66122d345c` - fix(dkg): Fix scalar ElGamal PVSS aggregation bug

---

## What's Been Implemented

### Phase 2: Scalar ElGamal PVSS

**Files Created:**

- `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs` - Module exports
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` - Core PVSS transcript
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` - Weighted wrapper

**Implemented Functions:**

| Function                                  | File                 | Status                  |
| ----------------------------------------- | -------------------- | ----------------------- |
| `Transcript::deal()`                      | transcript.rs        | ✅ Complete             |
| `Transcript::verify()`                    | transcript.rs        | 🔶 Stub (returns error) |
| `Transcript::aggregate_with()`            | transcript.rs        | ✅ Complete             |
| `Transcript::decrypt_own_share()`         | transcript.rs        | ✅ Complete             |
| `Transcript::get_dealt_public_key()`      | transcript.rs        | ✅ Complete             |
| `Transcript::get_public_key_share()`      | transcript.rs        | ✅ Complete             |
| `WeightedTranscript::deal()`              | weighted_protocol.rs | ✅ Complete             |
| `WeightedTranscript::verify()`            | weighted_protocol.rs | ✅ Delegates to inner   |
| `WeightedTranscript::decrypt_own_share()` | weighted_protocol.rs | ✅ Complete             |

**Test Status:**

- `randomness::e2e_correctness` - ✅ PASSING (validates DKG still works)
- `scalar_elgamal::tests` - ✅ ALL 15 PASSING (Jan 19, 2026)

**Commits:**

- `5644e16d84` - feat(dkg): add Scalar ElGamal PVSS and Timelock Registry stubs
- `5c9a7f095b` - feat(timelock): Phase 2 & 3 stubs for Dual-Output DKG
- `8801244943` - feat(timelock): Phase 2 Scalar ElGamal PVSS implementation
- `79bbe2002b` - feat(timelock): Implement WeightedTranscript::deal() for Dual-Output DKG
- `66122d345c` - fix(dkg): Fix scalar ElGamal PVSS aggregation bug

---

## What Remains

### Phase 2.0.5: Unit Tests for Scalar ElGamal ✅ COMPLETE

**Goal:** Establish solid test coverage before integrating into RealDKG.

**Rationale:** The current test coverage is minimal (only compile-check stubs). We need proof that `deal()` → `decrypt_own_share()` produces correct shares before integration.

**Tests Implemented (15 total):**

#### Transcript Tests (`transcript.rs`)

- ✅ `test_deal_creates_valid_structure` - Verify correct V, ephemeral_keys, ciphertexts sizes
- ✅ `test_deal_decrypt_roundtrip` - Deal secret, all players decrypt shares
- ✅ `test_aggregation_preserves_structure` - Multiple dealers aggregate correctly
- ✅ `test_serialization_roundtrip` - BCS serialize/deserialize
- ✅ `test_dealt_public_key_consistency` - MPK matches expected value
- ✅ `test_aggregated_decrypt_combines_secrets` - Aggregated shares decrypt correctly

#### WeightedTranscript Tests (`weighted_protocol.rs`)

- ✅ `test_weighted_encryption_key_expansion` - Verify weight duplication logic
- ✅ `test_weighted_deal_decrypt_roundtrip` - With validator weights
- ✅ `test_weighted_aggregation` - Multiple weighted transcripts
- ✅ `test_weighted_serialization_roundtrip` - BCS serialize/deserialize
- ✅ `test_weighted_dealt_public_key_consistency` - MPK matches expected
- ✅ `test_weighted_realistic_validator_weights` - Real-world stake proportions

**Validation:** All unit tests pass with `cargo test -p aptos-dkg` (44 tests total)

---

### Phase 2.1: Integration into RealDKG + DKGTrait ✅ COMPLETE

**Goal:** Add scalar transcript to RealDKG and extend DKGTrait with IBE-specific methods.

**Implemented:**

#### RealDKG Integration

- ✅ Extended `Transcripts` struct with `scalar: Option<ScalarTrx>` field
- ✅ Added `ScalarTrx` type alias for `scalar_elgamal::WeightedTranscript`
- ✅ Modified `generate_transcript()` to deal scalar transcript from same input secret
- ✅ Modified `aggregate_transcripts()` to aggregate scalar transcripts
- ✅ Modified `decrypt_secret_share_from_transcript()` to decrypt scalar shares
- ✅ Added `DealtSecretKeyShares.scalar` and `DealtPubKeyShares.scalar` fields

#### DKGTrait Refactoring

- ✅ Added `get_ibe_master_public_key(transcript) -> Vec<u8>` to DKGTrait (stub)
- ✅ Added `get_scalar_secret_share(dealt_share) -> Option<Vec<u8>>` to DKGTrait (stub)
- ✅ Implemented for RealDKG (stubs return empty/None, full impl in Phase 3)

**Note:** The `get_ibe_master_public_key` and `get_scalar_secret_share` implementations are stubs that return empty/None. Full serialization implementation is Phase 3 work.

### Phase 2.2: Aggregation Bug Fix ✅ COMPLETE

**Date:** January 19, 2026

**Bug:** Validator nodes crashed during DKG with discrete log solver failure when using aggregated transcripts (multiple dealers).

**Root Cause:** `chunks_to_scalar()` used byte conversion which assumed chunk values < 2^16. Aggregated transcripts produce sum of chunks from all dealers, exceeding this limit.

**Fix:**

1. **Fixed scalar reconstruction** (`transcript.rs:949-973`):
   - Replaced byte-based conversion with proper field arithmetic
   - Uses `result += chunk * multiplier` with `multiplier *= 2^16`

2. **Expanded BSGS search range** (`transcript.rs:848-855`):
   - Single dealer: `[0, 2^16)`
   - Multiple dealers: `[0, num_dealers × 2^16)`

3. **Fixed type aliases** (`weighted_protocol.rs:64-65`):
   - Changed `g1::DealtSecretKey` → `scalar::DealtSecretKey`

4. **Added `Mul` trait import** (`transcript.rs:114`):
   - Required for blstrs `G1Projective.mul()` operations

**Files Modified:**

- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` (+1094/-94 lines)
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` (+4/-0 lines)

**Validation:**

```
cargo test -p aptos-dkg --lib scalar_elgamal  # 15/15 pass
cargo test -p smoke-test randomness_correctness  # PASS (260s)
```

### Phase 1E: IBE Integration Tests ✅ COMPLETE

**Date:** January 19, 2026

**Goal:** Validate full IBE roundtrip with scalar ElGamal PVSS.

**Tests Added:** `crates/aptos-dkg/src/ibe/tests.rs`

| Test                                               | Description                                                                           | Status  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------- | ------- |
| `test_ibe_roundtrip_with_known_scalar`             | IBE encrypt/decrypt with known scalar secret                                          | ✅ PASS |
| `test_scalar_elgamal_pvss_ibe_roundtrip`           | Deal → decrypt shares → reconstruct → IBE encrypt/decrypt (4 validators, threshold 3) | ✅ PASS |
| `test_scalar_elgamal_pvss_ibe_multiple_identities` | Multiple IBE identities with reconstructed secret (5 validators)                      | ✅ PASS |

**Implementation Details:**

1. **Test Flow:**
   - Use `setup_dealing::<WeightedTranscript>()` to generate validator keys
   - Create `InputSecret` and deal via `WeightedTranscript::deal()`
   - Each validator decrypts share via `decrypt_own_share()` → returns `DealtSecretKeyShare` containing scalar
   - Reconstruct master scalar via `Reconstructable::reconstruct()` with `WeightedConfig`
   - Derive IBE decryption key and validate encrypt/decrypt roundtrip

2. **Type Correctness:**
   - `DealtSecretKeyShare = Vec<dealt_secret_key_share::scalar::DealtSecretKeyShare>` (weighted)
   - Single share per validator when weight=1
   - Blanket impl `Reconstructable<WeightedConfig>` in `generic_weighting.rs` handles weighted reconstruction

3. **Spec Compliance:** Verified against ADR-001:
   - ✅ Chunked Lifted ElGamal: `C_{i,j} = G · u_{i,j} + PK_i · r_j`
   - ✅ Scalar output: `DealtSecretKey { s: Scalar }`
   - ✅ Ephemeral keys: `ephemeral_keys: Vec<G1Projective>`
   - ✅ Ciphertexts: `ciphertexts: Vec<Vec<G1Projective>>`
   - ✅ BSGS decryption: 16-bit chunks, range `[0, 2^16)`

**Test Results:**

```
cargo test -p aptos-dkg --lib ibe::tests  # 17/17 pass (includes 2 new tests)
cargo test -p aptos-dkg --lib             # 47/47 pass (full DKG suite)
```

**Files Modified:**

| File                                  | Change                             |
| ------------------------------------- | ---------------------------------- |
| `crates/aptos-dkg/src/ibe/tests.rs`   | Added 2 IBE-PVSS integration tests |
| `crates/aptos-dkg/src/ibe/mod.rs`     | Added `ff::Field` import (minor)   |
| `testsuite/smoke-test/src/ibe/mod.rs` | Smoke test framework               |
| `testsuite/smoke-test/src/lib.rs`     | Added `ibe` module                 |

**Commit:** `20553ea074` - feat(ibe): Add scalar ElGamal PVSS IBE integration tests

---

## Code Review Findings (January 19, 2026)

A comprehensive code review was conducted comparing the implementation against ADR-001.

### ✅ What's Correct

| Component | Status | Notes |
|-----------|--------|-------|
| Chunked Lifted ElGamal struct | ✅ | Matches ADR-001: `ephemeral_keys`, `ciphertexts`, `encrypted_aggregate` |
| 16-bit chunking | ✅ | 256-bit scalar → 16 chunks of 16 bits |
| Correlated randomness | ✅ | `Σ r_j · 2^{16j} = 0` constraint implemented |
| BSGS discrete log | ✅ | Proper search range expansion for aggregated transcripts |
| `chunks_to_scalar()` | ✅ | Uses field arithmetic (fixed in v2.3) |
| Weighted protocol | ✅ | Correct type aliases, weight duplication |
| IBE primitives | ✅ | Boneh-Franklin construction, pairing verification |
| RealDKG dealing | ✅ | `generate_transcript()` deals scalar transcript |
| RealDKG aggregation | ✅ | `aggregate_transcripts()` aggregates scalar |
| RealDKG decryption | ✅ | `decrypt_secret_share_from_transcript()` works |

### ⚠️ Issues Found

#### Issue 1: Verification NOT Implemented (CRITICAL)

**Locations:**
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs:638-659`
- `types/src/dkg/real_dkg/mod.rs:464-487` (commented out)
- `types/src/dkg/real_dkg/mod.rs:381-390` (commented out)

**Problem:** `Transcript::verify()` only checks array dimensions, then returns `Ok(())`. No cryptographic verification is performed. A malicious dealer could submit an invalid scalar transcript.

**Current Code:**
```rust
fn verify<A: Serialize + Clone>(&self, sc, _pp, _spks, _eks, _auxs) -> Result<()> {
    if self.ciphertexts.len() != sc.n { bail!(...); }
    if self.V.len() != sc.n + 1 { bail!(...); }
    // NO CRYPTOGRAPHIC CHECKS!
    Ok(())
}
```

**Required Checks:**
1. Schnorr proof verification (proves dealer knows secret)
2. DLEQ proofs for encryption correctness
3. Polynomial commitment consistency
4. Signature verification on SoKs

#### Issue 2: DKGTrait Stubs Return Empty/None (HIGH)

**Locations:**
- `types/src/dkg/real_dkg/mod.rs:638-651` - `get_ibe_master_public_key()`
- `types/src/dkg/real_dkg/mod.rs:662-677` - `get_scalar_secret_share()`

**Problem:** These methods are stubs that return empty `Vec<u8>` or `None`, making on-chain MPK and scalar share serialization non-functional.

**Current Code:**
```rust
fn get_ibe_master_public_key(transcript: &Self::Transcript) -> Vec<u8> {
    let _dpk = transcript.main.get_dealt_public_key();
    Vec::new()  // STUB!
}

fn get_scalar_secret_share(dealt_share: &Self::DealtSecretShare) -> Option<Vec<u8>> {
    let _scalar_shares = dealt_share.scalar.as_ref()?;
    None  // STUB!
}
```

#### Issue 3: Silent BSGS Failure (HIGH)

**Location:** `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs:866-873`

**Problem:** If BSGS discrete log fails, the code silently returns 0 instead of an error, corrupting the share.

**Current Code:**
```rust
match result {
    Some(u_ij) => recovered_chunks.push(u_ij as u16),
    None => recovered_chunks.push(0u16),  // SILENT FAILURE!
}
```

**Should:** Return `Err` or `panic!` if BSGS fails (indicates invalid transcript or implementation bug).

### Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| `scalar_elgamal/transcript.rs` | 6 | deal, decrypt, aggregate, serialize |
| `scalar_elgamal/weighted_protocol.rs` | 6 | weighted deal/decrypt/aggregate |
| `ibe/tests.rs` | 17 | encrypt/decrypt, PVSS→IBE roundtrip |
| `smoke-test/src/ibe/` | 2 | E2E with real swarm |

---

## What Remains

### Phase 2.3: Transcript Verification 🔴 CRITICAL

**Goal:** Implement cryptographic verification for scalar ElGamal transcripts.

**Priority:** CRITICAL - Required before any production use.

**Tasks:**

1. **Implement `Transcript::verify()` in `transcript.rs`:**
   - [ ] Verify Schnorr proof (PoK of secret)
   - [ ] Verify polynomial commitment sizes and structure
   - [ ] Verify SoK signatures
   - [ ] Add DLEQ proof verification (optional - can defer)

2. **Enable scalar verification in RealDKG:**
   - [ ] Uncomment and implement `verify_transcript()` scalar checks (`real_dkg/mod.rs:464-487`)
   - [ ] Uncomment and implement `verify_transcript_extra()` scalar checks (`real_dkg/mod.rs:381-390`)
   - [ ] Verify dealers match main transcript
   - [ ] Verify MPK matches main transcript

3. **Add verification tests:**
   - [ ] `test_verify_valid_transcript` - Valid transcript passes
   - [ ] `test_verify_tampered_ciphertext` - Tampered ciphertext fails
   - [ ] `test_verify_wrong_commitment` - Wrong V fails
   - [ ] `test_verify_invalid_schnorr` - Invalid PoK fails

**Files to Modify:**
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`
- `types/src/dkg/real_dkg/mod.rs`

**Success Criteria:**
- [ ] `Transcript::verify()` performs cryptographic checks
- [ ] Invalid transcripts are rejected
- [ ] All existing tests still pass
- [ ] New verification tests pass

---

### Phase 2.4: Serialization Implementation 🟡 HIGH

**Goal:** Implement proper serialization for MPK and scalar shares.

**Priority:** HIGH - Required for on-chain use.

**Tasks:**

1. **Implement `get_ibe_master_public_key()`:**
   ```rust
   fn get_ibe_master_public_key(transcript: &Self::Transcript) -> Vec<u8> {
       let dpk = transcript.main.get_dealt_public_key();
       dpk.to_bytes().to_vec()  // G2Affine -> 96 bytes compressed
   }
   ```

2. **Implement `get_scalar_secret_share()`:**
   ```rust
   fn get_scalar_secret_share(dealt_share: &Self::DealtSecretShare) -> Option<Vec<u8>> {
       let scalar_shares = dealt_share.scalar.as_ref()?;
       Some(scalar_shares.iter()
           .flat_map(|s| s.to_bytes().to_vec())
           .collect())
   }
   ```

3. **Add serialization tests:**
   - [ ] `test_mpk_serialization_roundtrip`
   - [ ] `test_scalar_share_serialization_roundtrip`
   - [ ] `test_mpk_matches_expected_format`

**Files to Modify:**
- `types/src/dkg/real_dkg/mod.rs`
- `crates/aptos-dkg/src/pvss/dealt_pub_key.rs` (if needed)

**Success Criteria:**
- [ ] `get_ibe_master_public_key()` returns 96-byte G2 compressed
- [ ] `get_scalar_secret_share()` returns serialized scalars
- [ ] MPK can be used for on-chain IBE

---

### Phase 2.5: Error Handling Hardening 🟡 HIGH

**Goal:** Replace silent failures with proper error handling.

**Priority:** HIGH - Required for production robustness.

**Tasks:**

1. **Fix BSGS failure handling in `decrypt_own_share()`:**
   ```rust
   match result {
       Some(u_ij) => recovered_chunks.push(u_ij as u16),
       None => {
           // This should never happen with valid transcripts
           bail!("BSGS discrete log failed for player {} chunk {}", player.id, j);
       }
   }
   ```

2. **Return `Result` from `decrypt_own_share()`:**
   - Change signature to return `Result<(DealtSecretKeyShare, DealtPubKeyShare), Error>`
   - Propagate errors up the call stack

3. **Add error handling tests:**
   - [ ] `test_decrypt_invalid_ciphertext_fails`
   - [ ] `test_decrypt_wrong_key_fails`

**Files to Modify:**
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`
- `crates/aptos-dkg/src/pvss/traits/transcript.rs` (trait signature if needed)

**Success Criteria:**
- [ ] No silent failures in decryption path
- [ ] Errors are propagated with context
- [ ] Invalid inputs produce clear error messages

---

### Phase 3: Timelock Registry

**Goal:** On-chain registry for timelock deadlines.

**Priority:** Medium - Feature work, after security hardening.

**Tasks:**

1. Extend `ibe_config.move` with deadline registration
2. Add view functions for querying pending timelocks
3. Smoke test: `register_and_query`

### Phase 4: DK Share Submission

**Goal:** Validators submit decryption key shares after deadline.

**Priority:** Medium - Feature work.

**Tasks:**

1. Add `TimelockShare` ValidatorTransaction type
2. Implement deadline monitoring in epoch_manager
3. Implement share aggregation on-chain
4. Smoke tests: `deadline_reveal`, `dk_aggregation`

### Phase 5: E2E Integration

**Goal:** Full encrypt/decrypt cycle with real timelock.

**Priority:** Medium - Feature work.

**Tasks:**

1. Implement `timelock_e2e` smoke test
2. Verify complete user journey works

---

## Test Philosophy

### CI Requirements

**Required CI Jobs:** The following 3 crate test suites must pass before merging:

```bash
# Job 1: DKG crate tests (includes Scalar ElGamal unit tests)
cargo test -p aptos-dkg

# Job 2: Crypto crate tests
cargo test -p aptos-crypto

# Job 3: Types crate tests
cargo test -p aptos-types
```

**Rationale:** These crates contain the core cryptographic primitives. Regressions here could break DKG, randomness, or IBE functionality silently.

### Smoke Test Gate

Before merging any change:

```bash
# MUST PASS - validates full DKG flow still works
cargo test -p smoke-test --lib randomness::e2e_correctness -- --test-threads=1 --nocapture
```

### Chain Liveness Invariants

- Blocks continue to progress
- Epoch reconfiguration succeeds
- No validator crashes
- No `panic!` or `unwrap()` in consensus code paths

---

## File Change Summary

### Completed Files

| File                                                            | Description           | Status |
| --------------------------------------------------------------- | --------------------- | ------ |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs`               | Module exports        | ✅     |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`        | Core PVSS             | ✅     |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` | Weighted wrapper      | ✅     |
| `crates/aptos-dkg/src/ibe/mod.rs`                               | IBE primitives        | ✅     |
| `crates/aptos-dkg/src/ibe/ciphertext.rs`                        | Ciphertext struct     | ✅     |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move`  | On-chain MPK          | ✅     |
| `testsuite/smoke-test/src/timelock/mpk_on_chain.rs`             | MPK smoke test        | ✅     |
| `atomica/docs/adr-001-dual-output-dkg.md`                       | Architecture decision | ✅     |

### Completed in Phase 1E

| File                                  | Description                | Status |
| ------------------------------------- | -------------------------- | ------ |
| `crates/aptos-dkg/src/ibe/tests.rs`   | IBE-PVSS integration tests | ✅     |
| `crates/aptos-dkg/src/ibe/mod.rs`     | Import fix (ff::Field)     | ✅     |
| `testsuite/smoke-test/src/ibe/mod.rs` | Smoke test framework       | ✅     |
| `testsuite/smoke-test/src/lib.rs`     | Added ibe module           | ✅     |

### Completed in Phase 2.1

| File                            | Description               | Status |
| ------------------------------- | ------------------------- | ------ |
| `types/src/dkg/real_dkg/mod.rs` | Add scalar to Transcripts | ✅     |
| `types/src/dkg/mod.rs`          | DKGTrait IBE methods      | ✅     |

### Completed in Phase 2.2

| File                                                            | Description                   | Status |
| --------------------------------------------------------------- | ----------------------------- | ------ |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`        | Fix `chunks_to_scalar` + BSGS | ✅     |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` | Fix type aliases              | ✅     |

### Pending Files

| File                                                       | Description                           | Phase |
| ---------------------------------------------------------- | ------------------------------------- | ----- |
| `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` | Complete test                         | 1E    |
| `crates/aptos-dkg/src/ibe/mod.rs`                          | `derive_decryption_key_from_shares()` | 1E    |
| `types/src/validator_txn/mod.rs`                           | TimelockShare type                    | 4     |
| `testsuite/smoke-test/src/timelock/e2e.rs`                 | E2E test                              | 5     |

---

## Known Issues

### 1. verify() Not Implemented 🔴 CRITICAL

**Location:** `scalar_elgamal/transcript.rs:638-659`
**Impact:** Malicious dealer could submit invalid transcript
**Mitigation:** Honest majority assumption (temporary); **MUST implement in Phase 2.3 before production**
**Priority:** CRITICAL
**Assigned Phase:** 2.3

### 2. Serialization Stubs Return Empty 🟡 HIGH

**Location:** `real_dkg/mod.rs:638-677`
**Impact:** On-chain MPK unusable, scalar shares not serializable
**Mitigation:** None - blocks on-chain use
**Priority:** HIGH
**Assigned Phase:** 2.4

### 3. Silent BSGS Failure 🟡 HIGH

**Location:** `transcript.rs:866-873`
**Impact:** Corrupted shares without error if BSGS fails
**Mitigation:** Should never happen with valid transcripts, but must handle
**Priority:** HIGH
**Assigned Phase:** 2.5

### 4. generate() Not Implemented 🔵 LOW

**Location:** `transcript.rs:892-897`, `weighted_protocol.rs:172-177`
**Impact:** Cannot generate random transcripts for benchmarking
**Mitigation:** Use `deal()` with known secrets for testing
**Priority:** LOW (test-only)

### 5. TODO Comments in Production Code

**Locations:**
- `real_dkg/mod.rs:189` - "TODO(Phase 2)" in Transcripts struct
- `real_dkg/mod.rs:205` - "TODO(Phase 2)" in DealtPubKeyShares
- `real_dkg/mod.rs:222` - "TODO(Phase 2)" in DealtSecretKeyShares

**Impact:** Documentation debt
**Mitigation:** Clean up after Phase 2.3-2.5 complete
**Priority:** LOW

---

## Success Criteria

### Phase 2.x (Security Hardening) Complete When:

- [x] `Transcript::deal()` implemented
- [x] `Transcript::decrypt_own_share()` implemented
- [x] `WeightedTranscript::deal()` implemented
- [x] `randomness::e2e_correctness` passes
- [x] Unit tests for Transcript pass (Phase 2.0.5)
- [x] Unit tests for WeightedTranscript pass (Phase 2.0.5)
- [x] Integrated into RealDKG Transcripts struct (Phase 2.1)
- [x] DKGTrait IBE methods added (Phase 2.1)
- [x] Aggregation bug fixed (Phase 2.2)
- [ ] **`Transcript::verify()` performs cryptographic checks (Phase 2.3)** 🔴
- [ ] **RealDKG verifies scalar transcript (Phase 2.3)** 🔴
- [ ] **`get_ibe_master_public_key()` returns valid bytes (Phase 2.4)** 🟡
- [ ] **`get_scalar_secret_share()` returns valid bytes (Phase 2.4)** 🟡
- [ ] **BSGS failure returns error, not 0 (Phase 2.5)** 🟡

### Production Ready When:

- [ ] All Phase 2.3 tasks complete (verification)
- [ ] All Phase 2.4 tasks complete (serialization)
- [ ] All Phase 2.5 tasks complete (error handling)
- [ ] No `TODO` comments in security-critical paths
- [ ] Security review completed

### Full Project Complete When:

- [ ] Phases 2.3-2.5 complete (security hardening)
- [ ] `mpk_encrypt_decrypt` smoke test passes
- [ ] `timelock_e2e` smoke test passes
- [ ] All Phase 3-5 tasks complete
- [x] No regressions in randomness tests
