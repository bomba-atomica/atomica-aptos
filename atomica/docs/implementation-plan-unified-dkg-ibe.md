# Implementation Plan: Unified DKG for Randomness + IBE

**Version:** 1.0
**Date:** January 16, 2026
**Branch:** timelock-das-vpss
**Status:** Implementation Plan

---

## Problem Statement

The previous approach attempted to run a **parallel IbeDKG** alongside the existing **RealDKG** (Randomness). This caused:

1. **Concurrency Deadlocks**: Two simultaneous DKG sessions in `EpochManager`
2. **Regressions**: Modifications to shared components broke Randomness DKG
3. **Complexity**: Sequential execution patches, dual transcript handling

## New Approach: Unified DKG

**Key Insight**: The same PVSS shares produced by RealDKG can be used for both:
- **Randomness**: WVUF evaluation on shares
- **IBE**: Decryption key derivation from shares

Instead of running two DKGs, we **extend RealDKG** to also expose IBE-compatible outputs:

```
                    ┌─────────────────────────────────────┐
                    │           SINGLE DKG RUN            │
                    │  (Existing RealDKG infrastructure)  │
                    └─────────────────┬───────────────────┘
                                      │
                    ┌─────────────────┴───────────────────┐
                    │                                     │
                    ▼                                     ▼
          ┌─────────────────┐                   ┌─────────────────┐
          │   RANDOMNESS    │                   │      IBE        │
          │                 │                   │                 │
          │ WVUF::eval(sk)  │                   │ derive_dk(sk,id)│
          │ → random seed   │                   │ → decryption key│
          └─────────────────┘                   └─────────────────┘
```

## Benefits

1. **No concurrent DKG sessions** - eliminates deadlocks
2. **No modifications to EpochManager DKG flow** - reduces regression risk
3. **Same security model** - reuses existing threshold (2/3+1)
4. **Simpler architecture** - single transcript, single key storage

---

## Implementation Phases

### Phase 0: Baseline Verification (TDD Setup)

**Goal**: Ensure existing randomness DKG works before any changes.

**Smoke Test**: `test_baseline_randomness_dkg`
- Start 4-validator swarm with randomness enabled
- Wait for epoch 2 (DKG completes)
- Verify DKG transcript is on-chain
- Verify randomness is being produced
- **Must pass before proceeding**

**Files**:
- `testsuite/smoke-test/src/timelock/mod.rs` (new module)
- `testsuite/smoke-test/src/timelock/baseline_randomness.rs`

---

### Phase 1: IBE Crypto Primitives

**Goal**: Add IBE encryption/decryption to `aptos-dkg` crate without touching DKG flow.

**Components**:

1. **IBE Module** (`crates/aptos-dkg/src/ibe/mod.rs`)
   ```rust
   pub fn compute_identity(timelock_id: u64, deadline_us: u64) -> [u8; 32]
   pub fn hash_to_g1(identity: &[u8]) -> G1Projective
   pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine
   pub fn ibe_encrypt(mpk: &G2Affine, identity: &[u8], msg: &[u8]) -> Ciphertext
   pub fn ibe_decrypt(dk: &G1Affine, ciphertext: &Ciphertext) -> Vec<u8>
   ```

2. **Serialization Helpers**
   ```rust
   pub fn serialize_g1(point: &G1Affine) -> [u8; 48]
   pub fn deserialize_g1(bytes: &[u8]) -> Result<G1Affine>
   pub fn serialize_g2(point: &G2Affine) -> [u8; 96]
   pub fn deserialize_g2(bytes: &[u8]) -> Result<G2Affine>
   ```

**Smoke Test**: `test_ibe_crypto_roundtrip`
- Unit test IBE encrypt/decrypt with known test vectors
- Verify cross-compatibility with TypeScript implementation
- Test identity derivation matches spec

**Files**:
- `crates/aptos-dkg/src/ibe/mod.rs` (new)
- `crates/aptos-dkg/src/ibe/ciphertext.rs` (new)
- `crates/aptos-dkg/src/ibe/tests.rs` (new)
- `crates/aptos-dkg/src/lib.rs` (add `pub mod ibe`)

**Commit**: "feat(aptos-dkg): add IBE crypto primitives"

---

### Phase 2: Extract MPK from DKG Transcript

**Goal**: Derive Master Public Key (MPK) for IBE from existing DKG output.

**Key Insight**: The DKG transcript contains a dealt public key. For IBE:
- **MPK** = dealt public key (G2 point)
- This is already computed during DKG; we just need to expose it

**Components**:

1. **Add MPK extraction to RealDKG** (`types/src/dkg/real_dkg/mod.rs`)
   ```rust
   impl DKGTrait for RealDKG {
       // Existing methods...

       /// Extract MPK (G2) from transcript for IBE
       fn get_master_public_key(transcript: &Transcripts) -> G2Affine {
           transcript.main.get_dealt_public_key()
       }
   }
   ```

2. **Add trait method to DKGTrait** (`types/src/dkg/mod.rs`)
   ```rust
   pub trait DKGTrait {
       // Existing...

       /// Get the master public key for IBE from a transcript
       fn get_ibe_master_public_key(transcript: &Self::Transcript) -> Vec<u8>;
   }
   ```

**Smoke Test**: `test_mpk_extraction_from_dkg`
- Run DKG to completion
- Extract MPK from transcript
- Verify MPK is valid G2 point (96 bytes)
- Verify MPK matches dealt public key

**Files**:
- `types/src/dkg/mod.rs` (extend trait)
- `types/src/dkg/real_dkg/mod.rs` (implement extraction)
- `testsuite/smoke-test/src/timelock/mpk_extraction.rs`

**Commit**: "feat(dkg): add MPK extraction for IBE"

---

### Phase 3: On-Chain IBE State Storage

**Goal**: Store IBE public parameters on-chain after DKG completes.

**Components**:

1. **Move Module** (`aptos-move/framework/aptos-framework/sources/ibe_config.move`)
   ```move
   module aptos_framework::ibe_config {
       struct IBEPublicParams has key {
           /// Master Public Key (G2, 96 bytes)
           mpk: vector<u8>,
           /// Epoch when this MPK was generated
           epoch: u64,
       }

       /// Called by block prologue after DKG completes
       public(friend) fun on_dkg_complete(mpk: vector<u8>, epoch: u64)

       /// View function for clients
       public fun get_mpk(): vector<u8>

       /// Check if IBE is ready for encryption
       public fun is_ready(): bool
   }
   ```

2. **Integrate with block.move**
   - After DKG transcript is accepted, extract MPK and store

3. **Validator Transaction Handler**
   - Extend existing DKG result handler to also publish MPK

**Smoke Test**: `test_ibe_mpk_publication`
- Run DKG to completion
- Query `ibe_config::get_mpk()` via view function
- Verify MPK matches extracted value from transcript
- Verify `is_ready()` returns true

**Files**:
- `aptos-move/framework/aptos-framework/sources/ibe_config.move` (new)
- `aptos-move/framework/aptos-framework/sources/block.move` (integrate)
- `aptos-vm/src/validator_txns/dkg.rs` (extend handler)
- `testsuite/smoke-test/src/timelock/mpk_publication.rs`

**Commit**: "feat(framework): add ibe_config module for MPK storage"

---

### Phase 4: Deadline Registration & DK Derivation

**Goal**: Allow registering timelocks and deriving decryption keys from shares.

**Components**:

1. **Extend ibe_config.move**
   ```move
   struct TimelockRegistry has key {
       /// Map: deadline_timestamp_us -> TimelockInfo
       deadlines: Table<u64, TimelockInfo>,
       next_timelock_id: u64,
   }

   struct TimelockInfo has store {
       timelock_id: u64,
       deadline_timestamp_us: u64,
       /// None until revealed, then G1 decryption key (48 bytes)
       decryption_key: Option<vector<u8>>,
       /// Accumulated share contributions
       share_contributions: vector<ShareContribution>,
   }

   /// Register a new timelock (returns timelock_id)
   public fun register_timelock(deadline_us: u64): u64

   /// Called by validators to submit DK share
   public(friend) fun submit_dk_share(deadline_us: u64, share: vector<u8>, validator: address)

   /// Get decryption key (only available after deadline + threshold shares)
   public fun get_decryption_key(deadline_us: u64): Option<vector<u8>>
   ```

2. **Validator Share Submission** (`consensus/src/epoch_manager.rs`)
   - Watch for deadlines passing via block timestamps
   - Compute DK contribution: `dk_share = secret_share * H(identity)`
   - Submit via ValidatorTransaction

3. **On-Chain Aggregation**
   - Collect G1 share contributions
   - When threshold reached, aggregate using Lagrange coefficients
   - Store final DK

**Smoke Test**: `test_deadline_reveal`
- Register a timelock with deadline = now + 10 seconds
- Wait for deadline to pass
- Verify validators submit shares
- Verify DK is aggregated and available
- Verify DK can decrypt test ciphertext

**Files**:
- `aptos-move/framework/aptos-framework/sources/ibe_config.move` (extend)
- `consensus/src/epoch_manager.rs` (add deadline monitoring)
- `types/src/validator_txn/mod.rs` (add TimelockShare variant)
- `aptos-vm/src/validator_txns/mod.rs` (add handler)
- `testsuite/smoke-test/src/timelock/deadline_reveal.rs`

**Commit**: "feat(timelock): implement deadline registration and DK reveal"

---

### Phase 5: E2E Integration Test

**Goal**: Full encryption/decryption cycle test.

**Smoke Test**: `test_timelock_e2e`
1. Start swarm, wait for DKG
2. Query MPK from chain
3. Register timelock with deadline = now + 15 seconds
4. Encrypt message off-chain using MPK + identity
5. Wait for deadline
6. Query DK from chain
7. Decrypt message off-chain
8. Verify plaintext matches original

**Files**:
- `testsuite/smoke-test/src/timelock/e2e.rs`
- `testsuite/smoke-test/src/timelock/ibe_client.rs` (test helper)

**Commit**: "test(timelock): add E2E smoke test"

---

## TDD Workflow

For each phase:

1. **Write failing test first**
   ```bash
   # Create test file, run to verify it fails
   RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::phase_N -- --nocapture
   ```

2. **Implement minimal code to pass**
   - Make smallest change possible
   - Run test after each change

3. **Commit when green**
   ```bash
   git add -A && git commit -m "feat(component): description"
   ```

4. **Verify no regressions**
   ```bash
   # Run baseline randomness test
   RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_basic_consumption -- --nocapture
   ```

5. **Push for CI**
   ```bash
   git push origin timelock-das-vpss
   ```

---

## Smoke Test Summary

| Phase | Test Name | Validates |
|-------|-----------|-----------|
| 0 | `baseline_randomness` | Existing DKG works |
| 1 | `ibe_crypto_roundtrip` | IBE primitives correct |
| 2 | `mpk_extraction_from_dkg` | MPK derivable from transcript |
| 3 | `ibe_mpk_publication` | MPK stored on-chain |
| 4 | `deadline_reveal` | Share submission + aggregation |
| 5 | `timelock_e2e` | Full encrypt/decrypt cycle |

**Run all timelock tests**:
```bash
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock -- --nocapture --test-threads=1
```

---

## Risk Mitigation

1. **No EpochManager DKG flow changes in Phases 0-2**
   - IBE primitives and MPK extraction are additive
   - Existing randomness unaffected

2. **Minimal consensus changes in Phases 3-4**
   - Reuse existing ValidatorTransaction infrastructure
   - New transaction type, not modified existing

3. **Baseline test as regression gate**
   - Always run `baseline_randomness` before pushing
   - CI runs full randomness test suite

4. **Incremental commits**
   - Each phase is a separate commit
   - Easy to bisect if issues arise

---

## File Change Summary

### New Files
- `crates/aptos-dkg/src/ibe/mod.rs`
- `crates/aptos-dkg/src/ibe/ciphertext.rs`
- `crates/aptos-dkg/src/ibe/tests.rs`
- `aptos-move/framework/aptos-framework/sources/ibe_config.move`
- `testsuite/smoke-test/src/timelock/mod.rs`
- `testsuite/smoke-test/src/timelock/baseline_randomness.rs`
- `testsuite/smoke-test/src/timelock/mpk_extraction.rs`
- `testsuite/smoke-test/src/timelock/mpk_publication.rs`
- `testsuite/smoke-test/src/timelock/deadline_reveal.rs`
- `testsuite/smoke-test/src/timelock/e2e.rs`

### Modified Files
- `crates/aptos-dkg/src/lib.rs` (add `pub mod ibe`)
- `types/src/dkg/mod.rs` (extend DKGTrait)
- `types/src/dkg/real_dkg/mod.rs` (implement MPK extraction)
- `types/src/validator_txn/mod.rs` (add TimelockShare)
- `aptos-move/framework/aptos-framework/sources/block.move` (integrate IBE)
- `aptos-vm/src/validator_txns/mod.rs` (add handler)
- `consensus/src/epoch_manager.rs` (deadline monitoring)
- `testsuite/smoke-test/src/lib.rs` (add timelock module)

---

## Success Criteria

1. All 6 smoke tests pass
2. Existing randomness tests still pass
3. No concurrent DKG sessions
4. MPK queryable via view function
5. DK revealed after deadline + threshold
6. TypeScript client can encrypt/decrypt
