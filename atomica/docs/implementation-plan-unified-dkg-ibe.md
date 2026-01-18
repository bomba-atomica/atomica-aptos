# Implementation Plan: Unified DKG for Randomness + IBE

**Version:** 1.4
**Date:** January 17, 2026
**Branch:** timelock-das-vpss
**Status:** Implementation In Progress

## Changelog

- **v1.4** (Jan 17, 2026): Reordered phases to prioritize functionality over refactoring. Phase 2 is now Timelock Registry (was Phase 3), Phase 3 is DK Share Submission (was Phase 4), Phase 4 is DKGTrait Refactoring (was Phase 2, now marked optional).
- **v1.3** (Jan 17, 2026): Marked Phase 0 and Phase 1A-1C complete. IBE crypto primitives implemented and tested. Updated architectural notes to reflect actual implementation choices (SHA3-256 for key derivation, BCS for Gt serialization).
- **v1.2** (Jan 17, 2026): Restructured phases to prioritize on-chain MPK storage. Added comprehensive test pyramid (unit, integration, smoke) for each phase. Added IBE encryption/decryption verification in smoke tests.
- **v1.1** (Jan 16, 2026): Initial plan with Phase 0 complete.

---

## Test Philosophy

> **CRITICAL**: Every phase must maintain backward compatibility. Existing randomness tests MUST continue to pass.

### Test Pyramid

Each phase includes three levels of testing:

1. **Unit Tests** - Fast, isolated tests for individual functions
   - Run in < 1 second per test
   - No network or blockchain dependencies
   - Location: `crates/*/src/**/tests.rs` or inline `#[cfg(test)]`

2. **Integration Tests** - Test module interactions
   - May use mock blockchain state
   - Location: `aptos-move/e2e-move-tests/` for Move, `crates/*/tests/` for Rust

3. **Smoke Tests** - End-to-end validation with real validator swarm
   - Full blockchain environment
   - Location: `testsuite/smoke-test/src/timelock/`
   - Run with: `RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::* -- --nocapture`

### Regression Gate

> **CRITICAL INVARIANT**: DKG modifications must NEVER halt the chain. There is no situation where a panic or process exit is acceptable in consensus code. All errors must be handled gracefully with logging and recovery.

Before merging any phase, verify:

#### 1. Core Randomness Tests

```bash
# MUST PASS - Core randomness (baseline)
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture

# MUST PASS - Basic consumption
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_basic_consumption -- --nocapture
```

#### 2. Chain Liveness Tests

```bash
# MUST PASS - Blocks continue to be produced
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib consensus::consensus_only -- --nocapture

# MUST PASS - Epoch reconfiguration succeeds
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::dkg_with_validator_join_leave -- --nocapture
```

#### 3. Chain Liveness Invariants

Every smoke test must verify these invariants:

| Invariant               | How to Verify                                       | Failure Mode             |
| ----------------------- | --------------------------------------------------- | ------------------------ |
| Blocks progress         | `client.get_ledger_information().version` increases | Chain halted             |
| Epoch advances          | `epoch` field increases after reconfiguration       | Stuck in epoch           |
| Post-epoch blocks       | Version continues increasing after epoch change     | Epoch transition failure |
| No validator crashes    | All 4 validators remain responsive                  | Process exit/panic       |
| Graceful error handling | No `panic!` or `unwrap()` on fallible paths         | Crash on edge case       |

**Liveness Check Helper** (add to all smoke tests):

```rust
async fn verify_chain_liveness(client: &Client, test_name: &str) {
    let info1 = client.get_ledger_information().await.unwrap().into_inner();
    let initial_version = info1.version;
    let initial_epoch = info1.epoch;

    // Wait for blocks to progress
    tokio::time::sleep(Duration::from_secs(5)).await;

    let info2 = client.get_ledger_information().await.unwrap().into_inner();
    assert!(
        info2.version > initial_version,
        "[{}] Chain halted! Version stuck at {}",
        test_name,
        initial_version
    );

    info!(
        "[{}] Chain liveness OK: version {} -> {}, epoch {}",
        test_name, initial_version, info2.version, info2.epoch
    );
}

async fn verify_epoch_transition(client: &Client, test_name: &str) {
    let info1 = client.get_ledger_information().await.unwrap().into_inner();
    let initial_epoch = info1.epoch;

    // Trigger reconfiguration (implementation-specific)
    trigger_reconfiguration(client).await;

    // Wait for epoch to advance
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let info = client.get_ledger_information().await.unwrap().into_inner();
        if info.epoch > initial_epoch {
            info!(
                "[{}] Epoch transition OK: {} -> {}",
                test_name, initial_epoch, info.epoch
            );
            break;
        }
        if Instant::now() > deadline {
            panic!(
                "[{}] Epoch transition FAILED! Stuck at epoch {}",
                test_name, initial_epoch
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Verify blocks continue after epoch change
    verify_chain_liveness(client, &format!("{}_post_epoch", test_name)).await;
}
```

#### 4. Consensus Code Safety Rules

**NEVER allowed in consensus/DKG code paths:**

```rust
// ❌ FORBIDDEN - Will crash the validator
panic!("something went wrong");
unwrap();  // on fallible operations
expect("should never fail");
unreachable!();
std::process::exit(1);

// ✅ REQUIRED - Graceful error handling
match result {
    Ok(value) => value,
    Err(e) => {
        error!("[DKG] Operation failed: {:?}. Continuing without IBE.", e);
        return; // Skip IBE, but keep consensus running
    }
}

// ✅ REQUIRED - Defensive defaults
let mpk = transcript.main.get_dealt_public_key()
    .map(|pk| serialize_g2(&pk))
    .unwrap_or_else(|e| {
        warn!("[IBE] Failed to extract MPK: {:?}. IBE disabled for this epoch.", e);
        Vec::new()  // Empty MPK = IBE not ready
    });
```

#### 5. Pre-Merge Checklist

Before merging any IBE/timelock change:

- [ ] `randomness::e2e_correctness` passes
- [ ] `randomness::e2e_basic_consumption` passes
- [ ] `consensus::consensus_only` passes (blocks progress)
- [ ] Epoch reconfiguration test passes
- [ ] Manual review: no `panic!`, `unwrap()`, or `expect()` in new consensus code
- [ ] Manual review: all new error paths log and continue, never crash
- [ ] Smoke test runs for full 2+ minutes without validator crashes
- [ ] Post-epoch block production verified

---

## Prerequisites

> **IMPORTANT**: Before embarking on this implementation, verify the baseline state of randomness tests.

### Known Failing Tests (as of branch fork)

The following randomness smoke tests are **failing on this branch**:

```
randomness::disable_feature_0::disable_feature_0
randomness::disable_feature_1::disable_feature_1
randomness::enable_feature_0::enable_feature_0
randomness::enable_feature_1::enable_feature_1
randomness::enable_feature_2::enable_feature_2
```

These are feature flag toggle tests, not core DKG functionality tests.

### Required Passing Tests

The following tests **MUST pass** before proceeding with IBE implementation:

```bash
# Core randomness correctness (DKG + randomness generation)
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture

# Basic consumption (DKG + on-chain randomness usage)
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_basic_consumption -- --nocapture
```

If these core tests fail, fix them first before proceeding with IBE work.

### Baseline Verification (January 16, 2026)

✅ **`e2e_correctness` PASSED** - Core DKG and WVUF verification working correctly.

The feature flag tests (`enable_feature_*`, `disable_feature_*`) fail due to epoch tracking issues unrelated to core DKG functionality. These can be addressed separately.

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

### Phase 0: Feasibility Test (Can RealDKG Do Extra Work?) ✅ COMPLETE

**Goal**: Verify that we can extend RealDKG to perform additional operations without breaking existing randomness functionality.

**Approach**:

- Extract the Master Public Key (MPK) from the DKG transcript after share decryption
- This proves we can access the dealt public key needed for IBE
- Log the result to confirm extraction works

**Implementation** (January 16, 2026):

1. Added MPK extraction after `decrypt_secret_share_from_transcript()` in `epoch_manager.rs`
2. Used `transcript.main.get_dealt_public_key()` to access the dealt public key
3. Logged the MPK type for verification

**Verification**:

```bash
# PASSED after modification:
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
# Result: ok. 1 passed; 0 failed; finished in 117.45s
```

**Files Modified**:

- `consensus/src/epoch_manager.rs` (added MPK extraction after share extraction)

**Commit**: `8abe840f94` - "feat(consensus): Phase 0 - extract MPK from DKG transcript"

---

### Phase 1: On-Chain MPK Storage with IBE Primitives

**Goal**: Store the Master Public Key (MPK) on-chain in a Move struct, with a view function to retrieve it. Add IBE crypto primitives for encryption/decryption.

**Rationale**: The MPK is the foundation for all IBE operations. By storing it on-chain first, clients can immediately start encrypting messages. This also enables the key smoke test: encrypt with on-chain MPK → decrypt with corresponding private key.

#### Phase 1 Status

| Sub-Phase | Description                              | Status                       |
| --------- | ---------------------------------------- | ---------------------------- |
| 1A        | Move module `ibe_config.move`            | ✅ COMPLETE                  |
| 1B        | Rust MPK extraction in `dkg.rs`          | ✅ COMPLETE                  |
| 1C        | IBE Crypto Module (`aptos-dkg/src/ibe/`) | ✅ COMPLETE                  |
| 1D        | Smoke test `mpk_on_chain`                | ✅ COMPLETE                  |
| 1E        | Smoke test `mpk_encrypt_decrypt`         | 🔶 BLOCKED (Crypto Mismatch) |

#### Critical Issue: IBE-DKG Mismatch

Our analysis has confirmed a fundamental type mismatch:

- **DKG (DAS)** produces `G1Projective` shares (reconstructs to `g1^s`).
- **IBE (BF)** requires `Scalar` secret (to compute `H(id)^s`).
- **Upstream** avoids this by using `Chunky` PVSS (Scalar shares) + `FPTX` IBE.

**Decision:**

1.  **Short-term (for tests):** Implement "Shadow Mode" where we derive a scalar from the G1 element: `s' = Hash(g1^s)`. We use this `s'` to derive a "Shadow MPK" for testing encryption. This allows testing the _plumbing_ (Move -> Rust -> Key Derivation).
2.  **Long-term (for production):** We must switch Timelock to use **Chunky PVSS** (Scalar DKG), separate from the Randomness DKG. This is a Phase 3 task.

#### Next Steps

1.  Implement `g1_to_scalar` in `aptos-dkg`.
2.  Update `mpk_encrypt_decrypt` to use this "Shadow Mode".
3.  Mark Phase 1 complete with this caveat.

**Commits:**

- `63c0544add` - feat(ibe): Phase 1A - add ibe_config.move module
- `cca59e5bf0` - feat(ibe): Phase 1B+1C - wire MPK extraction and on-chain storage
- `8b2410b965` - feat(ibe): add IBE crypto primitives for timelock encryption

**Components**:

1. **Move Module** (`aptos-move/framework/aptos-framework/sources/ibe_config.move`)

   ```move
   module aptos_framework::ibe_config {
       use std::option::{Self, Option};
       use aptos_framework::system_addresses;

       /// Stores IBE public parameters, updated after each DKG
       struct IBEPublicParams has key {
           /// Master Public Key (G2, 96 bytes compressed)
           mpk: vector<u8>,
           /// Epoch when this MPK was generated
           epoch: u64,
       }

       /// Initialize IBE config (called during genesis)
       public fun initialize(aptos_framework: &signer) {
           system_addresses::assert_aptos_framework(aptos_framework);
           move_to(aptos_framework, IBEPublicParams {
               mpk: vector::empty(),
               epoch: 0,
           });
       }

       /// Update MPK after DKG completes (called by validator transaction handler)
       public(friend) fun set_mpk(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
           let params = borrow_global_mut<IBEPublicParams>(@aptos_framework);
           params.mpk = mpk;
           params.epoch = epoch;
       }

       /// View function: Get current MPK
       #[view]
       public fun get_mpk(): vector<u8> acquires IBEPublicParams {
           borrow_global<IBEPublicParams>(@aptos_framework).mpk
       }

       /// View function: Get epoch when MPK was set
       #[view]
       public fun get_epoch(): u64 acquires IBEPublicParams {
           borrow_global<IBEPublicParams>(@aptos_framework).epoch
       }

       /// View function: Check if IBE is ready for encryption
       #[view]
       public fun is_ready(): bool acquires IBEPublicParams {
           vector::length(&borrow_global<IBEPublicParams>(@aptos_framework).mpk) == 96
       }
   }
   ```

2. **IBE Crypto Module** (`crates/aptos-dkg/src/ibe/mod.rs`) ✅ IMPLEMENTED

   ```rust
   // Identity derivation
   pub fn compute_identity(timelock_id: u64, deadline_us: u64) -> [u8; 32]
   pub fn hash_to_g1(identity: &[u8]) -> G1Projective

   // Key derivation
   pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine
   pub fn verify_decryption_key(dk: &G1Affine, identity: &[u8], mpk: &G2Affine) -> bool

   // Encryption/Decryption
   pub fn ibe_encrypt<R: rand::Rng>(mpk: &G2Affine, identity: &[u8], msg: &[u8], rng: &mut R) -> Ciphertext
   pub fn ibe_decrypt(dk: &G1Affine, ciphertext: &Ciphertext) -> Vec<u8>
   ```

   **Implementation Notes:**
   - Uses SHA3-256 for identity hashing (DST: `APTOS_IBE_IDENTITY_DST`)
   - Uses SHA3-256 in counter mode for symmetric key derivation from Gt
   - Gt serialization via BCS (deterministic)
   - Ciphertext struct: `{ u: G2Affine, v: Vec<u8> }`
   - 16 unit tests covering roundtrips, serialization, and edge cases

3. **MPK Publication in Validator Transaction Handler**
   - Extend `aptos-vm/src/validator_txns/dkg.rs` to call `ibe_config::set_mpk()` after DKG transcript is accepted
   - Extract MPK from transcript using `transcript.main.get_dealt_public_key()`

**Tests**:

| Type        | Test                                                  | Validates                                                   | Status     |
| ----------- | ----------------------------------------------------- | ----------------------------------------------------------- | ---------- |
| Unit        | `ibe::tests::test_compute_identity_deterministic`     | Identity derivation is deterministic                        | ✅         |
| Unit        | `ibe::tests::test_hash_to_g1_deterministic`           | Hash-to-curve produces valid G1 point                       | ✅         |
| Unit        | `ibe::tests::test_encrypt_decrypt_roundtrip`          | IBE encrypt/decrypt with known keys                         | ✅         |
| Unit        | `ibe::tests::test_encrypt_decrypt_large_message`      | Large message encryption/decryption                         | ✅         |
| Unit        | `ibe::tests::test_wrong_decryption_key_fails`         | Wrong identity fails to decrypt                             | ✅         |
| Unit        | `ibe::tests::test_verify_decryption_key_valid`        | DK verification against MPK                                 | ✅         |
| Unit        | `ibe::tests::test_ciphertext_serialization_roundtrip` | Ciphertext BCS serialization                                | ✅         |
| Unit        | `ibe::ciphertext::tests::*` (2 tests)                 | Ciphertext struct basics                                    | ✅         |
| Integration | `ibe_config::test_initialize`                         | Move module initializes correctly                           | ✅         |
| Integration | `ibe_config::test_set_and_get_mpk`                    | MPK storage and retrieval                                   | ✅         |
| Integration | `ibe_config::test_is_ready_before_and_after`          | Ready check before/after MPK set                            | ✅         |
| Integration | `ibe_config::test_mpk_update_across_epochs`           | MPK updates correctly on epoch change                       | ✅         |
| Integration | `ibe_config::test_set_mpk_invalid_length_*`           | Invalid MPK rejected                                        | ✅         |
| **Smoke 1** | `timelock::mpk_on_chain`                              | **DKG stores MPK on-chain, retrievable and deserializable** | ✅         |
| **Smoke 2** | `timelock::mpk_encrypt_decrypt`                       | **On-chain MPK can encrypt; private key can decrypt**       | 🔲 BLOCKED |

**Test Counts:**

- Unit tests (IBE Rust): 16 tests ✅ PASSING
- Integration tests (Move): 9 tests ✅ PASSING
- Smoke tests: 2 tests 🔲 1 PASSED, 1 BLOCKED

---

#### Smoke Test 1: `mpk_on_chain` (MPK Storage & Retrieval)

**Purpose**: Verify the DKG process correctly stores the MPK on-chain and it can be retrieved and deserialized.

```rust
#[tokio::test]
async fn mpk_on_chain() {
    // === SETUP ===
    let swarm = new_local_swarm_with_randomness(4).await;
    let client = swarm.validators().next().unwrap().rest_client();

    // 1. Wait for DKG to complete
    let dkg_session = wait_for_dkg_finish(&client, None, 120).await;
    info!("DKG completed for epoch {}", dkg_session.metadata.dealer_epoch);

    // === VERIFY MPK IS ON-CHAIN ===

    // 2. Query MPK from chain via view function
    let mpk_bytes = view_ibe_config_get_mpk(&client).await;

    // 3. Verify MPK has correct length (G2 compressed = 96 bytes)
    assert_eq!(
        mpk_bytes.len(),
        96,
        "MPK should be 96 bytes (G2 compressed), got {} bytes",
        mpk_bytes.len()
    );

    // 4. Verify MPK can be deserialized to a valid G2 point
    let mpk = deserialize_g2(&mpk_bytes)
        .expect("MPK should deserialize to valid G2 point");

    // 5. Verify is_ready() returns true
    let is_ready = view_ibe_config_is_ready(&client).await;
    assert!(is_ready, "ibe_config::is_ready() should return true after DKG");

    // 6. Verify epoch matches
    let on_chain_epoch = view_ibe_config_get_epoch(&client).await;
    assert_eq!(
        on_chain_epoch,
        dkg_session.metadata.dealer_epoch + 1,
        "On-chain epoch should match DKG target epoch"
    );

    // === VERIFY MPK MATCHES TRANSCRIPT ===

    // 7. Extract MPK directly from the DKG transcript
    let transcript: Transcripts = bcs::from_bytes(&dkg_session.transcript)
        .expect("transcript should deserialize");
    let transcript_mpk = transcript.main.get_dealt_public_key();
    let transcript_mpk_bytes = serialize_g2(&transcript_mpk);

    // 8. Verify on-chain MPK matches transcript MPK
    assert_eq!(
        mpk_bytes, transcript_mpk_bytes,
        "On-chain MPK must match MPK extracted from DKG transcript"
    );

    info!("SUCCESS: MPK correctly stored on-chain and matches transcript");

    // === VERIFY CHAIN LIVENESS ===
    verify_chain_liveness(&client, "mpk_on_chain").await;
}
```

**What This Test Validates**:

- DKG completes successfully
- MPK is published to `ibe_config` Move module
- MPK is retrievable via `get_mpk()` view function
- MPK is exactly 96 bytes (valid G2 compressed format)
- MPK can be deserialized to a valid G2 curve point
- `is_ready()` returns true after MPK is set
- Epoch stored on-chain matches DKG epoch
- On-chain MPK byte-for-byte matches MPK extracted from transcript
- Chain continues to make progress (liveness check)

---

#### Smoke Test 2: `mpk_encrypt_decrypt` (IBE Encryption/Decryption)

**Purpose**: Verify that a message encrypted with the on-chain MPK can be decrypted using the private key derived from validator shares.

**Prerequisite**: `mpk_on_chain` test passes (MPK is correctly stored).

```rust
#[tokio::test]
async fn mpk_encrypt_decrypt() {
    // === SETUP ===
    let swarm = new_local_swarm_with_randomness(4).await;
    let client = swarm.validators().next().unwrap().rest_client();

    // 1. Wait for DKG to complete
    wait_for_dkg_finish(&client, None, 120).await;

    // 2. Verify IBE is ready
    assert!(
        view_ibe_config_is_ready(&client).await,
        "IBE must be ready before encryption test"
    );

    // === READ MPK FROM CHAIN ===

    // 3. Query MPK from chain via view function
    let mpk_bytes = view_ibe_config_get_mpk(&client).await;
    let mpk = deserialize_g2(&mpk_bytes).expect("valid G2 point");

    // === ENCRYPT MESSAGE ===

    // 4. Create a test identity (simulating a timelock identity)
    let test_timelock_id: u64 = 12345;
    let test_deadline_us: u64 = 1_000_000_000_000; // arbitrary future timestamp
    let identity = compute_identity(test_timelock_id, test_deadline_us);

    // 5. Encrypt a test message using the on-chain MPK
    let plaintext = b"Hello, Timelock! This is a secret message.";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext);

    info!(
        "Encrypted {} bytes -> {} bytes ciphertext",
        plaintext.len(),
        ciphertext.serialized_len()
    );

    // === DERIVE DECRYPTION KEY FROM SHARES ===

    // 6. Get the DKG transcript and validator decrypt keys
    let dkg_session = get_last_completed_dkg(&client).await;
    let decrypt_key_map = decrypt_key_map(&swarm);

    // 7. Derive the decryption key by combining validator shares
    //    This simulates what would happen after validators reveal shares
    let dk = derive_decryption_key_from_shares(
        &dkg_session,
        &decrypt_key_map,
        &identity,
    );

    // === DECRYPT AND VERIFY ===

    // 8. Decrypt the ciphertext using the derived decryption key
    let decrypted = ibe_decrypt(&dk, &ciphertext);

    // 9. Verify the decrypted message matches the original
    assert_eq!(
        decrypted.as_slice(),
        plaintext,
        "Decrypted message must match original plaintext"
    );

    info!("SUCCESS: Message encrypted with on-chain MPK, decrypted with derived key");

    // === NEGATIVE TEST: WRONG IDENTITY FAILS ===

    // 10. Verify that wrong identity cannot decrypt
    let wrong_identity = compute_identity(99999, 0);
    let wrong_dk = derive_decryption_key_from_shares(
        &dkg_session,
        &decrypt_key_map,
        &wrong_identity,
    );
    let wrong_decrypted = ibe_decrypt(&wrong_dk, &ciphertext);

    assert_ne!(
        wrong_decrypted.as_slice(),
        plaintext,
        "Wrong identity should NOT decrypt correctly"
    );

    info!("SUCCESS: Wrong identity correctly fails to decrypt");

    // === VERIFY CHAIN LIVENESS ===
    verify_chain_liveness(&client, "mpk_encrypt_decrypt").await;
}
```

**What This Test Validates**:

- MPK can be read from chain and used for encryption
- IBE encryption produces a valid ciphertext
- Decryption key can be derived from validator secret shares
- Decrypted message matches original plaintext exactly
- Wrong identity cannot decrypt the message (security property)
- Chain continues to make progress (liveness check)

---

**Smoke Test Dependencies**:

```
┌─────────────────────┐
│  mpk_on_chain       │  ← Run first: validates storage
│  (Storage Test)     │
└──────────┬──────────┘
           │ depends on
           ▼
┌─────────────────────┐
│ mpk_encrypt_decrypt │  ← Run second: validates crypto
│  (Crypto Test)      │
└─────────────────────┘
```

**Run Commands**:

```bash
# Run storage test only
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::mpk_on_chain -- --nocapture

# Run encryption/decryption test only
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::mpk_encrypt_decrypt -- --nocapture

# Run both in sequence
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib "timelock::mpk_" -- --nocapture --test-threads=1
```

**Files**:

- `aptos-move/framework/aptos-framework/sources/ibe_config.move` (new)
- `crates/aptos-dkg/src/ibe/mod.rs` (new)
- `crates/aptos-dkg/src/ibe/ciphertext.rs` (new)
- `crates/aptos-dkg/src/ibe/tests.rs` (new)
- `crates/aptos-dkg/src/lib.rs` (add `pub mod ibe`)
- `aptos-vm/src/validator_txns/dkg.rs` (extend to publish MPK)
- `testsuite/smoke-test/src/timelock/mod.rs` (new)
- `testsuite/smoke-test/src/timelock/mpk_on_chain.rs` (new)
- `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` (new)
- `testsuite/smoke-test/src/lib.rs` (add `mod timelock`)

**Commit**: "feat(ibe): add on-chain MPK storage with IBE primitives and smoke tests"

---

### Phase 2: Timelock Registry & Deadline Tracking

**Goal**: Allow users to register timelocks with deadlines. Track deadlines on-chain for coordinated reveal.

**Components**:

1. **Extend ibe_config.move**

   ```move
   /// Registry for active timelocks
   struct TimelockRegistry has key {
       deadlines: Table<u64, TimelockInfo>,
       next_timelock_id: u64,
   }

   struct TimelockInfo has store {
       timelock_id: u64,
       deadline_timestamp_us: u64,
       identity: vector<u8>,  // 32-byte identity hash
       decryption_key: Option<vector<u8>>,  // G1, 48 bytes when revealed
       reveal_threshold: u64,
       share_count: u64,
   }

   /// Register a new timelock (returns timelock_id)
   public entry fun register_timelock(
       account: &signer,
       deadline_us: u64
   ): u64

   /// View: Get timelock info
   #[view]
   public fun get_timelock(timelock_id: u64): TimelockInfo

   /// View: Get decryption key (None before reveal)
   #[view]
   public fun get_decryption_key(timelock_id: u64): Option<vector<u8>>
   ```

**Tests**:

| Type        | Test                                    | Validates                                  |
| ----------- | --------------------------------------- | ------------------------------------------ |
| Unit        | `ibe_config::test_register_timelock`    | Timelock registration succeeds             |
| Unit        | `ibe_config::test_identity_derivation`  | Identity computed correctly from deadline  |
| Integration | `ibe_config::test_registry_persistence` | Timelocks survive across blocks            |
| Smoke       | `timelock::register_and_query`          | Register timelock, query via view function |

**Files**:

- `aptos-move/framework/aptos-framework/sources/ibe_config.move` (extend)
- `testsuite/smoke-test/src/timelock/register_timelock.rs` (new)

**Commit**: "feat(timelock): add timelock registry with deadline tracking"

---

### Phase 3: Decryption Key Share Submission & Aggregation

**Goal**: Validators submit DK shares after deadline passes; aggregate to reveal decryption key.

**Components**:

1. **Validator Share Submission** (`consensus/src/epoch_manager.rs`)
   - Monitor block timestamps for passed deadlines
   - Compute DK contribution: `dk_share = secret_share * H(identity)`
   - Submit via ValidatorTransaction

2. **New ValidatorTransaction Type** (`types/src/validator_txn/mod.rs`)

   ```rust
   pub enum ValidatorTransaction {
       // Existing variants...
       TimelockShare {
           timelock_id: u64,
           share: Vec<u8>,  // G1, 48 bytes
           validator_index: u64,
       },
   }
   ```

3. **On-Chain Aggregation** (extend `ibe_config.move`)

   ```move
   /// Submit a DK share (called by validator transaction handler)
   public(friend) fun submit_dk_share(
       timelock_id: u64,
       share: vector<u8>,
       validator_index: u64
   )

   /// Internal: Aggregate shares when threshold reached
   fun try_aggregate_dk(timelock_id: u64)
   ```

**Tests**:

| Type        | Test                                     | Validates                               |
| ----------- | ---------------------------------------- | --------------------------------------- |
| Unit        | `ibe::test_dk_share_derivation`          | DK share computation correct            |
| Unit        | `ibe::test_lagrange_aggregation`         | Share aggregation produces valid DK     |
| Integration | `ibe_config::test_share_submission`      | Shares accepted and counted             |
| Integration | `ibe_config::test_threshold_aggregation` | DK revealed at threshold                |
| Smoke       | `timelock::deadline_reveal`              | Validators submit shares after deadline |
| Smoke       | `timelock::dk_aggregation`               | DK aggregated and queryable             |

**Key Smoke Test: `deadline_reveal`**:

```rust
#[tokio::test]
async fn deadline_reveal() {
    let swarm = new_local_swarm_with_randomness(4).await;
    wait_for_dkg_finish(&client, None, 120).await;

    // Register timelock with deadline = now + 10 seconds
    let deadline = current_time_us() + 10_000_000;
    let timelock_id = register_timelock(&client, deadline).await;

    // Wait for deadline to pass
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Verify DK is now available
    let dk = view_get_decryption_key(&client, timelock_id).await;
    assert!(dk.is_some(), "DK should be revealed after deadline");
    assert_eq!(dk.unwrap().len(), 48, "DK should be 48 bytes (G1)");
}
```

**Files**:

- `consensus/src/epoch_manager.rs` (add deadline monitoring)
- `types/src/validator_txn/mod.rs` (add TimelockShare)
- `aptos-vm/src/validator_txns/mod.rs` (add handler)
- `aptos-move/framework/aptos-framework/sources/ibe_config.move` (extend)
- `testsuite/smoke-test/src/timelock/deadline_reveal.rs` (new)

**Commit**: "feat(timelock): implement DK share submission and aggregation"

---

### Phase 4: Formalize MPK Extraction in DKGTrait (Optional Refactoring)

**Goal**: Add formal trait methods for MPK extraction to enable type-safe integration across the codebase.

**Rationale**: This is a code quality improvement, not new functionality. The MPK extraction already works via `transcript.main.get_dealt_public_key()`. This phase formalizes it as a trait method for cleaner architecture.

**Priority**: LOW - Can be done anytime or skipped entirely.

**Components**:

1. **Extend DKGTrait** (`types/src/dkg/mod.rs`)

   ```rust
   pub trait DKGTrait {
       // Existing methods...

       /// Get the master public key for IBE from a transcript (serialized G2)
       fn get_ibe_master_public_key(transcript: &Self::Transcript) -> Vec<u8>;
   }
   ```

2. **Implement for RealDKG** (`types/src/dkg/real_dkg/mod.rs`)
   ```rust
   impl DKGTrait for RealDKG {
       fn get_ibe_master_public_key(transcript: &Transcripts) -> Vec<u8> {
           let dpk = transcript.main.get_dealt_public_key();
           // Serialize G2 point to 96 bytes
           serialize_dealt_public_key(&dpk)
       }
   }
   ```

**Tests**:

| Type        | Test                                      | Validates                               |
| ----------- | ----------------------------------------- | --------------------------------------- |
| Unit        | `real_dkg::tests::test_get_ibe_mpk`       | MPK extraction returns valid 96-byte G2 |
| Unit        | `real_dkg::tests::test_mpk_deterministic` | Same transcript → same MPK              |
| Integration | `dkg_trait::test_mpk_matches_dealt_pk`    | Extracted MPK matches dealt public key  |

**Files**:

- `types/src/dkg/mod.rs` (extend trait)
- `types/src/dkg/real_dkg/mod.rs` (implement extraction)
- `types/src/dkg/real_dkg/tests.rs` (unit tests)

**Commit**: "refactor(dkg): formalize MPK extraction in DKGTrait"

---

### Phase 5: End-to-End Integration Test

**Goal**: Full encryption/decryption cycle test validating the entire flow.

**The Ultimate Smoke Test: `timelock_e2e`**:

This test validates the complete user journey:

```rust
#[tokio::test]
async fn timelock_e2e() {
    // === SETUP ===
    let swarm = new_local_swarm_with_randomness(4).await;
    let client = swarm.validators().next().unwrap().rest_client();

    // 1. Wait for DKG to complete
    wait_for_dkg_finish(&client, None, 120).await;

    // === ENCRYPTION (User side) ===

    // 2. Query MPK from chain via view function
    let mpk_bytes = view_ibe_config_get_mpk(&client).await;
    assert!(view_ibe_config_is_ready(&client).await, "IBE should be ready");
    let mpk = deserialize_g2(&mpk_bytes).expect("valid MPK");

    // 3. Register a timelock with deadline = now + 20 seconds
    let deadline_us = current_timestamp_us(&client).await + 20_000_000;
    let timelock_id = register_timelock(&client, deadline_us).await;

    // 4. Compute identity for this timelock
    let identity = compute_identity(timelock_id, deadline_us);

    // 5. Encrypt message off-chain using MPK + identity
    let plaintext = b"Secret message for the future!";
    let ciphertext = ibe_encrypt(&mpk, &identity, plaintext);

    // Message is now encrypted - cannot be decrypted until deadline passes

    // === WAIT FOR DEADLINE ===

    // 6. Wait for deadline to pass + validators to submit shares
    tokio::time::sleep(Duration::from_secs(25)).await;

    // === DECRYPTION (User side) ===

    // 7. Query decryption key from chain
    let dk_option = view_get_decryption_key(&client, timelock_id).await;
    assert!(dk_option.is_some(), "DK should be available after deadline");
    let dk = deserialize_g1(&dk_option.unwrap()).expect("valid DK");

    // 8. Decrypt message off-chain
    let decrypted = ibe_decrypt(&dk, &ciphertext);

    // 9. Verify plaintext matches original
    assert_eq!(decrypted, plaintext, "Decryption must recover original message");

    // === VERIFY NO REGRESSIONS ===

    // 10. Verify randomness still works
    let randomness = view_per_block_randomness(&client).await;
    assert!(randomness.seed.is_some(), "Randomness should still work");

    // 11. Verify chain liveness (blocks continue to progress)
    verify_chain_liveness(&client, "timelock_e2e").await;

    // 12. Verify epoch transition works (if time permits)
    // This ensures our changes don't break reconfiguration
    verify_epoch_transition(&client, "timelock_e2e").await;
}
```

**Test Coverage Summary**:

| Scenario                    | Validated By                                  |
| --------------------------- | --------------------------------------------- |
| MPK published after DKG     | Step 2: `get_mpk()` returns 96 bytes          |
| View function works         | Step 2: Direct view call succeeds             |
| IBE encryption works        | Step 5: `ibe_encrypt()` produces ciphertext   |
| Deadline registration works | Step 3: `register_timelock()` returns ID      |
| Validators submit shares    | Step 7: DK available after deadline           |
| IBE decryption works        | Step 8: `ibe_decrypt()` returns plaintext     |
| Full roundtrip              | Step 9: Decrypted == Original                 |
| No randomness regression    | Step 10: Randomness seed present              |
| **Chain liveness**          | Step 11: Blocks continue to progress          |
| **Epoch transition works**  | Step 12: Chain enters new epoch and continues |

**Files**:

- `testsuite/smoke-test/src/timelock/e2e.rs` (new)
- `testsuite/smoke-test/src/timelock/ibe_client.rs` (test helper functions)
- `testsuite/smoke-test/src/timelock/mod.rs` (export all tests)

**Commit**: "test(timelock): add comprehensive E2E smoke test"

---

## TDD Workflow

For each phase, follow strict TDD:

### Step 1: Write Tests First

```bash
# Unit tests (run fast, iterate quickly)
cargo test -p aptos-dkg --lib ibe:: -- --nocapture

# Integration tests (Move)
aptos move test --package-dir aptos-move/framework/aptos-framework

# Smoke tests (full validator swarm)
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::phase_N -- --nocapture
```

### Step 2: Run Tests (Expect Failure)

```bash
# Verify tests fail for the right reason (not compilation errors)
cargo test -p smoke-test --lib timelock::mpk_on_chain -- --nocapture 2>&1 | grep "assertion failed"
```

### Step 3: Implement Minimal Code

- Make the smallest change possible to pass the test
- Run tests after each change

### Step 4: Verify Green

```bash
# All new tests pass
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock -- --nocapture --test-threads=1
```

### Step 5: Verify No Regressions

```bash
# CRITICAL: Baseline randomness tests MUST still pass
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_basic_consumption -- --nocapture
```

### Step 6: Commit

```bash
git add -A && git commit -m "feat(component): description"
git push origin timelock-das-vpss
```

---

## Test Summary Matrix

### By Phase

| Phase | Description                  | Unit Tests          | Integration Tests   | Smoke Tests                           | Status        |
| ----- | ---------------------------- | ------------------- | ------------------- | ------------------------------------- | ------------- |
| 0     | Feasibility                  | -                   | -                   | `randomness::e2e_correctness`         | ✅            |
| 1     | MPK Storage + IBE Primitives | `ibe::*` (16)       | `ibe_config::*` (9) | `mpk_on_chain`, `mpk_encrypt_decrypt` | 🔶 1/2 PASSED |
| 2     | Timelock Registry            | `ibe_config::*` (2) | `ibe_config::*` (1) | `register_and_query`                  | 🔲            |
| 3     | DK Share Submission          | `ibe::*` (2)        | `ibe_config::*` (2) | `deadline_reveal`, `dk_aggregation`   | 🔲            |
| 4     | DKGTrait Refactor (Optional) | `real_dkg::*` (2)   | `dkg_trait::*` (1)  | -                                     | 🔲            |
| 5     | E2E Integration              | -                   | -                   | `timelock_e2e`                        | 🔲            |

### By Test Type

| Type               | Location                             | Run Command                                  | Expected Time |
| ------------------ | ------------------------------------ | -------------------------------------------- | ------------- |
| Unit               | `crates/*/src/**/tests.rs`           | `cargo test -p aptos-dkg --lib`              | < 10s         |
| Integration (Move) | `aptos-move/framework/**/*.move`     | `aptos move test`                            | < 30s         |
| Integration (Rust) | `crates/*/tests/`                    | `cargo test -p <crate>`                      | < 30s         |
| Smoke              | `testsuite/smoke-test/src/timelock/` | `cargo test -p smoke-test --lib timelock::*` | 2-5 min       |

### Run All Timelock Tests

```bash
# Quick validation (unit + integration)
cargo test -p aptos-dkg --lib ibe:: -- --nocapture && \
aptos move test --package-dir aptos-move/framework/aptos-framework

# Full validation (including smoke tests)
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock -- --nocapture --test-threads=1

# Full validation with randomness regression check
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib "randomness::e2e|timelock::" -- --nocapture --test-threads=1
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

**Move Contracts:**

- `aptos-move/framework/aptos-framework/sources/ibe_config.move`

**Rust IBE Module:**

- `crates/aptos-dkg/src/ibe/mod.rs`
- `crates/aptos-dkg/src/ibe/ciphertext.rs`
- `crates/aptos-dkg/src/ibe/tests.rs`

**Smoke Tests:**

- `testsuite/smoke-test/src/timelock/mod.rs`
- `testsuite/smoke-test/src/timelock/mpk_on_chain.rs`
- `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs`
- `testsuite/smoke-test/src/timelock/mpk_extraction.rs`
- `testsuite/smoke-test/src/timelock/register_timelock.rs`
- `testsuite/smoke-test/src/timelock/deadline_reveal.rs`
- `testsuite/smoke-test/src/timelock/e2e.rs`
- `testsuite/smoke-test/src/timelock/ibe_client.rs`

### Modified Files

**DKG Types:**

- `crates/aptos-dkg/src/lib.rs` (add `pub mod ibe`)
- `types/src/dkg/mod.rs` (extend DKGTrait)
- `types/src/dkg/real_dkg/mod.rs` (implement MPK extraction)

**Validator Transactions:**

- `types/src/validator_txn/mod.rs` (add TimelockShare)
- `aptos-vm/src/validator_txns/dkg.rs` (publish MPK)
- `aptos-vm/src/validator_txns/mod.rs` (add timelock handler)

**Consensus:**

- `consensus/src/epoch_manager.rs` (deadline monitoring)

**Test Infrastructure:**

- `testsuite/smoke-test/src/lib.rs` (add timelock module)

---

## Success Criteria

### Functional Requirements

| #   | Requirement                                          | Validated By                                |
| --- | ---------------------------------------------------- | ------------------------------------------- |
| 1   | MPK stored on-chain in Move struct                   | `timelock::mpk_on_chain` smoke test         |
| 2   | MPK retrievable via view function                    | `ibe_config::get_mpk()` returns 96 bytes    |
| 3   | MPK can be deserialized to valid G2 point            | `mpk_on_chain`: `deserialize_g2()` succeeds |
| 4   | On-chain MPK matches transcript MPK                  | `mpk_on_chain`: byte-for-byte comparison    |
| 5   | Message encrypted with on-chain MPK can be decrypted | `timelock::mpk_encrypt_decrypt` smoke test  |
| 6   | Wrong identity fails to decrypt                      | `mpk_encrypt_decrypt`: negative test case   |
| 7   | Timelocks can be registered with deadlines           | `timelock::register_and_query` smoke test   |
| 8   | DK revealed after deadline passes                    | `timelock::deadline_reveal` smoke test      |
| 9   | Full E2E encrypt/decrypt cycle works                 | `timelock::e2e` smoke test                  |
| 10  | TypeScript client can encrypt/decrypt                | Cross-compatibility unit tests              |

### Non-Regression Requirements

| #   | Requirement                                | Validated By                              |
| --- | ------------------------------------------ | ----------------------------------------- |
| 11  | `randomness::e2e_correctness` passes       | Smoke test (run before every merge)       |
| 12  | `randomness::e2e_basic_consumption` passes | Smoke test (run before every merge)       |
| 13  | No concurrent DKG sessions                 | Single DKG architecture (by design)       |
| 14  | No modifications to existing DKG flow      | Code review (Phase 0-2 are additive only) |

### Chain Liveness Requirements (CRITICAL)

| #   | Requirement                                    | Validated By                                 |
| --- | ---------------------------------------------- | -------------------------------------------- |
| 15  | Blocks continue to progress                    | `verify_chain_liveness()` in all smoke tests |
| 16  | Epoch reconfiguration succeeds                 | `verify_epoch_transition()` in smoke tests   |
| 17  | Chain produces blocks after epoch transition   | Post-epoch liveness check                    |
| 18  | No validator crashes during/after DKG          | All 4 validators responsive throughout test  |
| 19  | No `panic!`/`unwrap()` in consensus code paths | Manual code review before merge              |
| 20  | All DKG errors handled gracefully with logging | Error paths log and continue, never crash    |

### Test Coverage Requirements

| Level       | Minimum Tests                           | Status      |
| ----------- | --------------------------------------- | ----------- |
| Unit        | 16 tests in `ibe::*`                    | ✅ Complete |
| Integration | 9 tests in `ibe_config::*` (Move)       | ✅ Complete |
| Smoke       | 8 smoke tests (including 2 for Phase 1) | 🔲 Pending  |

### Definition of Done

A phase is complete when:

1. All unit tests pass (`cargo test -p aptos-dkg --lib`)
2. All integration tests pass (`aptos move test`)
3. All smoke tests pass (`cargo test -p smoke-test --lib timelock::*`)
4. Baseline randomness tests pass (regression gate)
5. Code is committed and pushed
6. CI passes on `timelock-das-vpss` branch
