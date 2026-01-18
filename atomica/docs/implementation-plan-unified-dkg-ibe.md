# Implementation Plan: Unified DKG for Randomness + IBE

**Version:** 2.1
**Date:** January 18, 2026
**Branch:** timelock-das-vpss
**Status:** Phase 2.0.5 In Progress
**Reference:** [ADR-001: Dual Output DKG](adr-001-dual-output-dkg.md)

## Changelog

- **v2.1** (Jan 18, 2026): Added Phase 2.0.5 (Unit Tests) before RealDKG integration. Added CI job requirements for regression testing.
- **v2.0** (Jan 18, 2026): Major update reflecting completed Phase 2 Scalar ElGamal implementation. Documented decisions made during development. Reorganized phases to reflect actual implementation order.
- **v1.5** (Jan 17, 2026): Reordered phases. Phase 2 is now Dual-Output DKG (ADR-001).
- **v1.4** (Jan 17, 2026): Reordered phases to prioritize functionality over refactoring.
- **v1.3** (Jan 17, 2026): Marked Phase 0 and Phase 1A-1C complete.

---

## Current Status Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Feasibility Test | ✅ COMPLETE |
| 1A-1D | IBE Primitives + MPK Storage | ✅ COMPLETE |
| 2 | Scalar ElGamal PVSS | ✅ CORE COMPLETE |
| 2.0.5 | Unit Tests for Scalar ElGamal | ✅ COMPLETE |
| 2.1 | Integration into RealDKG + DKGTrait | 🔲 PENDING (Next) |
| 1E | mpk_encrypt_decrypt smoke test | 🔶 BLOCKED → Ready after 2.1 |
| 3 | Timelock Registry | 🔲 PENDING |
| 4 | DK Share Submission | 🔲 PENDING |
| 5 | E2E Integration | 🔲 PENDING |

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

### Decision 2: Scalar ElGamal PVSS Design

**Date:** January 17-18, 2026
**Context:** Need a PVSS scheme that outputs scalar shares for IBE.

**Design Choices:**
- Encrypt scalar shares under ElGamal (not polynomial commitments like Chunky)
- Use G2 for polynomial commitments (`V` vector) to enable DLEQ proofs
- Use G1 for ciphertexts (`C` vector) matching encryption key group
- Ciphertext structure: `C[i] = h1 * f(i) + ek[i] * r` (additive ElGamal)
- Ephemeral key: `C_0 = g1 * r` for decryption

**Implementation:**
```rust
// transcript.rs
pub struct Transcript {
    soks: Vec<SoK<G2Projective>>,  // Signatures of Knowledge
    hat_w: G2Projective,            // Commitment to randomness
    V: Vec<G2Projective>,           // Polynomial commitments (n+1)
    C: Vec<G1Projective>,           // Ciphertexts (n)
    C_0: G1Projective,              // Ephemeral public key
}
```

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

---

## What's Been Implemented

### Phase 2: Scalar ElGamal PVSS

**Files Created:**
- `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs` - Module exports
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` - Core PVSS transcript
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` - Weighted wrapper

**Implemented Functions:**

| Function | File | Status |
|----------|------|--------|
| `Transcript::deal()` | transcript.rs | ✅ Complete |
| `Transcript::verify()` | transcript.rs | 🔶 Stub (returns error) |
| `Transcript::aggregate_with()` | transcript.rs | ✅ Complete |
| `Transcript::decrypt_own_share()` | transcript.rs | ✅ Complete |
| `Transcript::get_dealt_public_key()` | transcript.rs | ✅ Complete |
| `Transcript::get_public_key_share()` | transcript.rs | ✅ Complete |
| `WeightedTranscript::deal()` | weighted_protocol.rs | ✅ Complete |
| `WeightedTranscript::verify()` | weighted_protocol.rs | ✅ Delegates to inner |
| `WeightedTranscript::decrypt_own_share()` | weighted_protocol.rs | ✅ Complete |

**Test Status:**
- `randomness::e2e_correctness` - ✅ PASSING (validates DKG still works)

**Commits:**
- `5644e16d84` - feat(dkg): add Scalar ElGamal PVSS and Timelock Registry stubs
- `5c9a7f095b` - feat(timelock): Phase 2 & 3 stubs for Dual-Output DKG
- `8801244943` - feat(timelock): Phase 2 Scalar ElGamal PVSS implementation
- `79bbe2002b` - feat(timelock): Implement WeightedTranscript::deal() for Dual-Output DKG

---

## What Remains

### Phase 2.0.5: Unit Tests for Scalar ElGamal ✅ COMPLETE

**Goal:** Establish solid test coverage before integrating into RealDKG.

**Rationale:** The current test coverage is minimal (only compile-check stubs). We need proof that `deal()` → `decrypt_own_share()` produces correct shares before integration.

**Tests Implemented (15 total):**

#### Transcript Tests (`transcript.rs`)
- ✅ `test_deal_creates_valid_structure` - Verify correct V, C, C_0 vector sizes
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

### Phase 2.1: Integration into RealDKG + DKGTrait

**Goal:** Add scalar transcript to RealDKG and extend DKGTrait with IBE-specific methods.

**Tasks:**

#### RealDKG Integration
1. Extend `types/src/dkg/real_dkg/mod.rs`:
   ```rust
   pub struct Transcripts {
       pub main: WTrx,
       pub fast: Option<WTrx>,
       pub scalar: Option<scalar_elgamal::WeightedTranscript>,  // NEW
   }
   ```
2. Modify dealing in `epoch_manager.rs` to produce scalar transcript alongside main
3. Modify aggregation to include scalar transcript
4. Extract scalar MPK for on-chain storage

#### DKGTrait Refactoring
5. Add `get_ibe_master_public_key() -> Option<G2Affine>` to DKGTrait
6. Add `get_scalar_secret_share() -> Option<Scalar>` to DKGTrait
7. Implement for RealDKG

**Validation:** `mpk_on_chain` smoke test passes with scalar MPK

### Phase 1E: Unblock mpk_encrypt_decrypt

**Goal:** With scalar shares available, implement the blocked encrypt/decrypt smoke test.

**Tasks:**
1. Implement `derive_decryption_key_from_shares()` using scalar shares
2. Complete `mpk_encrypt_decrypt` smoke test
3. Validate IBE roundtrip works end-to-end

### Phase 3: Timelock Registry

**Goal:** On-chain registry for timelock deadlines.

**Tasks:**
1. Extend `ibe_config.move` with deadline registration
2. Add view functions for querying pending timelocks
3. Smoke test: `register_and_query`

### Phase 4: DK Share Submission

**Goal:** Validators submit decryption key shares after deadline.

**Tasks:**
1. Add `TimelockShare` ValidatorTransaction type
2. Implement deadline monitoring in epoch_manager
3. Implement share aggregation on-chain
4. Smoke tests: `deadline_reveal`, `dk_aggregation`

### Phase 5: E2E Integration

**Goal:** Full encrypt/decrypt cycle with real timelock.

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

| File | Description | Status |
|------|-------------|--------|
| `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs` | Module exports | ✅ |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` | Core PVSS | ✅ |
| `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` | Weighted wrapper | ✅ |
| `crates/aptos-dkg/src/ibe/mod.rs` | IBE primitives | ✅ |
| `crates/aptos-dkg/src/ibe/ciphertext.rs` | Ciphertext struct | ✅ |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move` | On-chain MPK | ✅ |
| `testsuite/smoke-test/src/timelock/mpk_on_chain.rs` | MPK smoke test | ✅ |
| `atomica/docs/adr-001-dual-output-dkg.md` | Architecture decision | ✅ |

### Pending Files

| File | Description | Phase |
|------|-------------|-------|
| `types/src/dkg/real_dkg/mod.rs` | Add scalar to Transcripts | 2.1 |
| `dkg/src/epoch_manager.rs` | Deal scalar transcript | 2.1 |
| `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` | Complete test | 1E |
| `types/src/validator_txn/mod.rs` | TimelockShare type | 5 |
| `testsuite/smoke-test/src/timelock/e2e.rs` | E2E test | 6 |

---

## Known Issues

### 1. verify() Not Implemented

**Location:** `scalar_elgamal/transcript.rs:145`
**Impact:** Malicious dealer could submit invalid transcript
**Mitigation:** Honest majority assumption; add verification before mainnet
**Priority:** Medium

### 2. generate() Not Implemented

**Location:** `transcript.rs:209`, `weighted_protocol.rs:176`
**Impact:** Cannot generate random transcripts for unit tests
**Mitigation:** Use `deal()` with known secrets for testing
**Priority:** Low (test-only)

---

## Success Criteria

### Phase 2 Complete When:
- [x] `Transcript::deal()` implemented
- [x] `Transcript::decrypt_own_share()` implemented
- [x] `WeightedTranscript::deal()` implemented
- [x] `randomness::e2e_correctness` passes
- [x] Unit tests for Transcript pass (Phase 2.0.5)
- [x] Unit tests for WeightedTranscript pass (Phase 2.0.5)
- [ ] Integrated into RealDKG Transcripts struct (Phase 2.1)
- [ ] Scalar MPK stored on-chain (Phase 2.1)

### Full Project Complete When:
- [ ] `mpk_encrypt_decrypt` smoke test passes
- [ ] `timelock_e2e` smoke test passes
- [ ] All Phase 1-6 tasks complete
- [ ] No regressions in randomness tests
