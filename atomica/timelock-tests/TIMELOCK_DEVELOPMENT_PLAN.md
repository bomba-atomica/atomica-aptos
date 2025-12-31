# Timelock Implementation - Development Plan

**Date:** December 31, 2024  
**Based on:** Code Review findings  
**Target:** Production-ready MVP

---

## Table of Contents

1. [Overview](#overview)
2. [Phase 1: Critical Bug Fixes](#phase-1-critical-bug-fixes)
3. [Phase 2: Security Hardening](#phase-2-security-hardening)
4. [Phase 3: Code Quality](#phase-3-code-quality)
5. [Phase 4: Test Coverage](#phase-4-test-coverage)
6. [Phase 5: TypeScript SDK](#phase-5-typescript-sdk)
7. [Phase 6: Future Enhancements](#phase-6-future-enhancements)
8. [Timeline Summary](#timeline-summary)
9. [Risk Assessment](#risk-assessment)

---

## Overview

### Current State

The timelock implementation is approximately **85% complete** for MVP. The core DKG integration, on-chain state management, and IBE cryptography are functional. However, critical bugs in the share aggregation logic must be fixed before production deployment.

### Goals

1. **Immediate (Week 1):** Fix critical bugs that could cause data loss
2. **Short-term (Week 2):** Security hardening and code quality improvements
3. **Medium-term (Weeks 3-4):** Complete test coverage and TypeScript SDK
4. **Future:** Advanced features (recovery, validator changes)

### Success Criteria

- [ ] All critical and high-priority bugs fixed
- [ ] Share aggregation produces correct decryption keys
- [ ] Basic share validation implemented
- [ ] End-to-end encryption/decryption works in integration tests
- [ ] TypeScript SDK can encrypt/decrypt messages

---

## Phase 1: Critical Bug Fixes

**Duration:** 2-3 days  
**Priority:** P0 - Must complete before any deployment

### Task 1.1: Fix Invalid Share Counting

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 2 hours  
**Risk:** High (core functionality)

#### Current Code (Lines 227-258)

The threshold check happens before validating shares, causing invalid shares to be counted.

#### Implementation Steps

1. Refactor `publish_secret_share` to validate shares first:

```move
public entry fun publish_secret_share(
    validator: &signer,
    interval: u64,
    share: vector<u8>
) acquires TimelockState {
    // ... existing validation ...

    let state = borrow_global_mut<TimelockState>(@aptos_framework);

    // Early return if already revealed
    if (table::contains(&state.revealed_secrets, interval)) {
        return
    };

    // Validate share format BEFORE storing
    let share_opt = deserialize<G1, FormatG1Compr>(&share);
    assert!(std::option::is_some(&share_opt), EINVALID_SHARE);

    // ... rest of storage logic ...

    // Count VALID shares for threshold
    let valid_count = count_valid_shares(shares_list);

    if (valid_count >= threshold) {
        aggregate_valid_shares(state, interval, shares_list, threshold);
    }
}

fun count_valid_shares(shares: &vector<ValidatorShare>): u64 {
    let count = 0;
    let i = 0;
    let len = vector::length(shares);
    while (i < len) {
        let s_bytes = &vector::borrow(shares, i).share;
        let element_opt = deserialize<G1, FormatG1Compr>(s_bytes);
        if (std::option::is_some(&element_opt)) {
            count = count + 1;
        };
        i = i + 1;
    };
    count
}

fun aggregate_valid_shares(
    state: &mut TimelockState,
    interval: u64,
    shares: &vector<ValidatorShare>,
    threshold: u64
) {
    let sum = zero<G1>();
    let valid_count = 0;
    let i = 0;
    let len = vector::length(shares);

    while (i < len && valid_count < threshold) {
        let s_bytes = &vector::borrow(shares, i).share;
        let element_opt = deserialize<G1, FormatG1Compr>(s_bytes);
        if (std::option::is_some(&element_opt)) {
            let element = std::option::extract(&mut element_opt);
            sum = add(&sum, &element);
            valid_count = valid_count + 1;
        };
        i = i + 1;
    };

    // Only store if we actually got enough valid shares
    if (valid_count >= threshold) {
        let aggregated_bytes = serialize<G1, FormatG1Compr>(&sum);
        table::add(&mut state.revealed_secrets, interval, aggregated_bytes);

        event::emit_event(&mut state.secret_revealed_events, SecretRevealedEvent {
            interval,
            secret: aggregated_bytes,
        });
    }
}
```

2. Add error constant:

```move
const EINVALID_SHARE: u64 = 3;  // Already exists, ensure it's used
```

3. Add unit test for invalid share rejection

#### Testing

```bash
cd aptos-move/framework/aptos-framework
cargo test test_invalid_share_rejected
```

---

### Task 1.2: Implement Historical Threshold Storage

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 4 hours  
**Risk:** High (affects correctness)

#### Implementation Steps

1. Add new struct for interval configuration:

```move
struct IntervalConfig has store, drop, copy {
    threshold: u64,
    total_validators: u64,
    created_at: u64,  // timestamp
}
```

2. Add table to TimelockState:

```move
struct TimelockState has key {
    // ... existing fields ...
    interval_configs: Table<u64, IntervalConfig>,
}
```

3. Update `initialize`:

```move
public(friend) fun initialize(framework: &signer) {
    // ... existing code ...
    move_to(framework, TimelockState {
        // ... existing fields ...
        interval_configs: table::new(),
    });
}
```

4. Store config when emitting StartKeyGenEvent:

```move
// In on_new_block, after calculating threshold:
let config = IntervalConfig {
    threshold,
    total_validators,
    created_at: now,
};
table::add(&mut state.interval_configs, state.current_interval, config);
```

5. Use stored config in `publish_secret_share`:

```move
// Replace dynamic threshold calculation with:
let config = table::borrow(&state.interval_configs, interval);
let threshold = config.threshold;
```

6. Add view function:

```move
#[view]
public fun get_interval_config(interval: u64): Option<IntervalConfig> acquires TimelockState {
    if (!exists<TimelockState>(@aptos_framework)) {
        return option::none()
    };
    let state = borrow_global<TimelockState>(@aptos_framework);
    if (table::contains(&state.interval_configs, interval)) {
        option::some(*table::borrow(&state.interval_configs, interval))
    } else {
        option::none()
    }
}
```

#### Testing

- Test that threshold from DKG time is used, not current time
- Test with simulated validator set changes

---

### Task 1.3: Fix Topic Mismatch

**File:** `dkg/src/dkg_manager/mod.rs`  
**Effort:** 30 minutes  
**Risk:** Low

#### Implementation

```rust
// Line 415, replace:
let vtxn_guard = self.vtxn_pool.put(
    Topic::DKG,
    Arc::new(txn),
    Some(self.pull_notification_tx.clone()),
);

// With:
let topic = if self.is_timelock {
    Topic::TIMELOCK
} else {
    Topic::DKG
};
let vtxn_guard = self.vtxn_pool.put(
    topic,
    Arc::new(txn),
    Some(self.pull_notification_tx.clone()),
);
```

#### Testing

```bash
cargo test -p aptos-dkg
```

---

## Phase 2: Security Hardening

**Duration:** 3-4 days  
**Priority:** P1 - Required for production

### Task 2.1: Add Share Pre-validation on Submission

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 2 hours

Already partially addressed in Task 1.1. Ensure:

- Share is validated before storage (not just aggregation)
- Invalid shares are rejected with clear error

### Task 2.2: Add Interval Validation

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 1 hour

```move
public entry fun publish_secret_share(...) {
    // Add: Verify interval is valid for reveal
    let state = borrow_global<TimelockState>(@aptos_framework);

    // Can only reveal shares for past intervals
    assert!(interval < state.current_interval, EINVALID_INTERVAL);

    // Can only reveal if we have a config (DKG happened)
    assert!(table::contains(&state.interval_configs, interval), EINVALID_INTERVAL);

    // ... rest of function
}
```

### Task 2.3: Cleanup Timelock DKG Sessions

**File:** `dkg/src/epoch_manager.rs`  
**Effort:** 1 hour

```rust
impl<P: OnChainConfigProvider> EpochManager<P> {
    // Add cleanup method
    fn cleanup_timelock_session(&mut self, interval: u64) {
        if let Some(close_tx) = self.timelock_dkg_close_txs.remove(&interval) {
            // Signal close
            let (ack_tx, _) = oneshot::channel();
            let _ = close_tx.send(ack_tx);
        }
        self.timelock_rpc_msg_txs.remove(&interval);
        info!("[Timelock] Cleaned up DKG session for interval {}", interval);
    }

    // Call after successful key publication
    fn process_timelock_key_published(&mut self, event: KeyPublishedEvent) {
        // ... existing code ...

        // Cleanup DKG session
        self.cleanup_timelock_session(event.interval);
    }
}
```

### Task 2.4: Add First-Share Assertion

**File:** `dkg/src/epoch_manager.rs`  
**Effort:** 30 minutes

```rust
// In process_timelock_reveal, line 661:
if shares.main.is_empty() {
    error!("[Timelock] No main shares available for interval {}", event.interval);
    return;
}

// Add explicit assertion with documentation
assert!(
    shares.main.len() == 1,
    "[Timelock] Expected exactly 1 main share for timelock DKG, got {}. \
     This indicates a configuration mismatch.",
    shares.main.len()
);

let dk_g1 = shares.main[0].as_group_element().clone();
```

---

## Phase 3: Code Quality

**Duration:** 1-2 days  
**Priority:** P2 - Should complete before release

### Task 3.1: Remove Unused Variables

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 30 minutes

Remove `validator_addresses` from both locations (lines 122-129 and 216-223) or use them for additional validation.

### Task 3.2: Improve Threshold Fallback

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 30 minutes

```move
// Replace line 134:
if (total_validators == 0) { threshold = 1; }; // Fallback for testing/genesis

// With:
assert!(total_validators > 0, EEMPTY_VALIDATOR_SET);

// Or for genesis compatibility:
if (total_validators == 0) {
    // Skip DKG during genesis when no validators are active yet
    return
};
```

### Task 3.3: Document Gt Hashing

**File:** `crates/aptos-dkg/src/ibe/mod.rs`  
**Effort:** 1 hour

```rust
/// Hashes a Gt element to bytes for use as a symmetric key.
///
/// # Implementation Note
///
/// BLS12-381 Gt elements don't have a standardized serialization format.
/// We use the Debug representation which is:
/// - Deterministic for the same Gt value
/// - Sufficient entropy for key derivation
///
/// For cross-language compatibility, any implementation must use
/// the same Debug format or this exact hashing approach.
///
/// TODO: Consider using a standardized Gt serialization if one becomes
/// available in the blstrs library.
fn hash_gt_to_bytes(gt: &Gt) -> Result<Vec<u8>> {
    let mut hasher = Keccak256::new();
    // Note: Using Debug format for Gt serialization - see docstring
    hasher.update(format!("{:?}", gt));
    Ok(hasher.finalize().to_vec())
}
```

### Task 3.4: Complete Move Spec File

**File:** `aptos-move/framework/aptos-framework/sources/specs/timelock.spec.move`  
**Effort:** 4 hours

```move
spec aptos_framework::timelock {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec publish_public_key {
        // Aborts if not a validator
        aborts_if !stake::spec_is_current_epoch_validator(
            std::signer::address_of(validator)
        );

        // Aborts if timelock not initialized
        aborts_if !exists<TimelockState>(@aptos_framework);

        // Only first publisher succeeds
        ensures old(!table::spec_contains(
            global<TimelockState>(@aptos_framework).public_keys,
            interval
        )) ==>
            table::spec_contains(
                global<TimelockState>(@aptos_framework).public_keys,
                interval
            );
    }

    spec publish_secret_share {
        // Aborts if not a validator
        aborts_if !stake::spec_is_current_epoch_validator(
            std::signer::address_of(validator)
        );

        // Aborts if timelock not initialized
        aborts_if !exists<TimelockState>(@aptos_framework);

        // Aborts if share is invalid (not a valid G1 point)
        // Note: This requires modeling the deserialize function
    }

    // Invariant: revealed secrets are never removed
    invariant forall interval: u64:
        old(table::spec_contains(
            global<TimelockState>(@aptos_framework).revealed_secrets,
            interval
        )) ==>
        table::spec_contains(
            global<TimelockState>(@aptos_framework).revealed_secrets,
            interval
        );
}
```

---

## Phase 4: Test Coverage

**Duration:** 3-4 days  
**Priority:** P1 - Required for confidence in production

### Task 4.1: Add Invalid Share Test (Move)

**File:** `aptos-move/framework/aptos-framework/sources/timelock.move`  
**Effort:** 2 hours

```move
#[test(framework = @aptos_framework)]
#[expected_failure(abort_code = EINVALID_SHARE)]
public fun test_invalid_share_rejected(framework: &signer) acquires TimelockState {
    // Setup
    timestamp::set_time_has_started_for_testing(framework);
    account::create_account_for_test(@aptos_framework);
    initialize(framework);

    // Create a validator account
    let validator = account::create_signer_for_test(@0x123);
    // Mock validator registration (requires stake module setup)

    // Attempt to submit invalid share (random bytes, not a valid G1 point)
    let invalid_share = vector[0u8, 1u8, 2u8, 3u8];
    publish_secret_share(&validator, 0, invalid_share);
}
```

### Task 4.2: Add Threshold Edge Case Tests (Rust)

**File:** `aptos-move/e2e-move-tests/src/tests/timelock_tests.rs`  
**Effort:** 3 hours

```rust
#[test]
fn test_threshold_exact() {
    // Setup with 4 validators
    // Submit exactly threshold (3) valid shares
    // Verify aggregation succeeds
}

#[test]
fn test_threshold_minus_one() {
    // Setup with 4 validators
    // Submit threshold-1 (2) valid shares
    // Verify aggregation does NOT happen
}

#[test]
fn test_threshold_with_invalid_shares() {
    // Setup with 4 validators
    // Submit 2 valid + 2 invalid shares
    // Verify aggregation does NOT happen (only 2 valid < threshold 3)
}
```

### Task 4.3: Add Smoke Test for Share Validation

**File:** `testsuite/smoke-test/src/timelock/basic_flow.rs`  
**Effort:** 2 hours

Add test case that:

1. Starts network with 4 validators
2. Waits for DKG
3. Manually submits an invalid share from one validator
4. Verifies the invalid share is rejected
5. Verifies that with 3 valid shares, aggregation still works

### Task 4.4: Complete TypeScript Tests

**File:** `atomica/timelock-tests/test/ibe-e2e.test.ts`  
**Effort:** 4 hours (depends on Task 5.1)

After TypeScript IBE is implemented, update tests to use real crypto operations instead of placeholders.

---

## Phase 5: TypeScript SDK

**Duration:** 4-5 days  
**Priority:** P2 - Required for frontend integration

### Task 5.1: Implement TypeScript IBE Module

**File:** `atomica/timelock-tests/src/ibe.ts` (new file)  
**Effort:** 8 hours

```typescript
import { bls12_381 } from "@noble/curves/bls12-381";
import { keccak_256 } from "@noble/hashes/sha3";

export interface Ciphertext {
  u: Uint8Array; // 96 bytes - G2 point
  v: Uint8Array; // encrypted message
}

/**
 * Compute timelock identity for encryption.
 */
export function computeTimelockIdentity(interval: bigint, chainId: number): Uint8Array {
  const hasher = keccak_256.create();

  // Add interval as little-endian bytes
  const intervalBytes = new Uint8Array(8);
  const view = new DataView(intervalBytes.buffer);
  view.setBigUint64(0, interval, true);
  hasher.update(intervalBytes);

  // Add chain ID
  hasher.update(new Uint8Array([chainId]));

  // Add domain separator
  hasher.update(new TextEncoder().encode("atomica_timelock"));

  return hasher.digest();
}

/**
 * Encrypt a message using IBE.
 */
export function ibeEncrypt(
  mpk: Uint8Array, // 96 bytes - G2 point
  identity: Uint8Array,
  message: Uint8Array,
): Ciphertext {
  // 1. Generate random scalar
  const r = bls12_381.utils.randomPrivateKey();

  // 2. Compute U = r * G2_generator
  const u = bls12_381.G2.ProjectivePoint.BASE.multiply(
    bls12_381.utils.mod(BigInt("0x" + Buffer.from(r).toString("hex"))),
  ).toRawBytes(true);

  // 3. Hash identity to G1 point
  const qId = bls12_381.G1.hashToCurve(identity, {
    DST: "BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_POP_",
  });

  // 4. Compute pairing and derive key
  const mpkPoint = bls12_381.G2.ProjectivePoint.fromHex(mpk);
  const gid = bls12_381.pairing(qId, mpkPoint);
  // Multiply by r (simplified - actual impl needs Gt exponentiation)

  // 5. Hash to symmetric key
  const keyHash = keccak_256(gid.toString());

  // 6. XOR encrypt
  const v = xorBytes(message, keyHash);

  return { u, v };
}

/**
 * Decrypt a ciphertext using the decryption key.
 */
export function ibeDecrypt(
  dk: Uint8Array, // 48 bytes - G1 point
  ciphertext: Ciphertext,
): Uint8Array {
  // 1. Compute pairing e(dk, u)
  const dkPoint = bls12_381.G1.ProjectivePoint.fromHex(dk);
  const uPoint = bls12_381.G2.ProjectivePoint.fromHex(ciphertext.u);
  const gid = bls12_381.pairing(dkPoint, uPoint);

  // 2. Hash to symmetric key
  const keyHash = keccak_256(gid.toString());

  // 3. XOR decrypt
  return xorBytes(ciphertext.v, keyHash);
}

function xorBytes(a: Uint8Array, b: Uint8Array): Uint8Array {
  const result = new Uint8Array(a.length);
  for (let i = 0; i < a.length; i++) {
    result[i] = a[i] ^ b[i % b.length];
  }
  return result;
}

/**
 * Deserialize a G1 point from compressed bytes.
 */
export function deserializeG1(bytes: Uint8Array): bls12_381.G1.ProjectivePoint {
  if (bytes.length !== 48) {
    throw new Error(`Invalid G1 compressed length: expected 48, got ${bytes.length}`);
  }
  return bls12_381.G1.ProjectivePoint.fromHex(bytes);
}

/**
 * Deserialize a G2 point from compressed bytes.
 */
export function deserializeG2(bytes: Uint8Array): bls12_381.G2.ProjectivePoint {
  if (bytes.length !== 96) {
    throw new Error(`Invalid G2 compressed length: expected 96, got ${bytes.length}`);
  }
  return bls12_381.G2.ProjectivePoint.fromHex(bytes);
}
```

### Task 5.2: Add Transcript Parsing

**File:** `atomica/timelock-tests/src/transcript.ts` (new file)  
**Effort:** 4 hours

```typescript
/**
 * Extract the Master Public Key (G2 point) from a DKG transcript.
 */
export function extractMpkFromTranscript(transcriptBytes: Uint8Array): Uint8Array {
  // The transcript is BCS-serialized Transcripts struct
  // We need to parse it to extract the dealt public key

  // This requires understanding the exact BCS format of:
  // - Transcripts { main: Transcript, aux: Option<Transcript> }
  // - Transcript contains dealt_public_key

  // For now, a simplified approach assuming known offsets
  // TODO: Implement full BCS parsing or use a generated deserializer

  throw new Error("Not yet implemented - requires BCS parsing");
}
```

### Task 5.3: Update TypeScript Tests

**File:** `atomica/timelock-tests/test/ibe-e2e.test.ts`  
**Effort:** 2 hours

Replace all placeholders with actual implementations.

---

## Phase 6: Future Enhancements

**Priority:** P3 - Post-MVP

### 6.1: DKG Failure Recovery

**Effort:** 16 hours

If DKG fails to complete for an interval:

1. Detect failure (timeout or insufficient dealers)
2. Emit failure event
3. Retry DKG for same interval
4. Allow fallback to previous interval's key with warning

### 6.2: Validator Set Change Handling

**Effort:** 24 hours

When validators join/leave during DKG:

1. Track validator set at DKG start
2. Only accept shares from original participants
3. Handle the case where leaving validators can't reveal

### 6.3: Key Rotation Within Interval

**Effort:** 8 hours

Currently first validator to publish wins. Consider:

- Voting on transcript acceptance
- Aggregating multiple transcripts
- Challenge mechanism for invalid transcripts

### 6.4: Metrics and Monitoring

**Effort:** 8 hours

Add Prometheus metrics for:

- DKG completion time per interval
- Share submission latency
- Aggregation success rate
- Invalid share rejections

---

## Timeline Summary

```
Week 1: Critical Fixes
├── Day 1-2: Task 1.1 (Invalid share counting)
├── Day 2-3: Task 1.2 (Historical threshold)
└── Day 3: Task 1.3 (Topic mismatch)

Week 2: Security & Quality
├── Day 1: Tasks 2.1-2.4 (Security hardening)
└── Day 2-3: Tasks 3.1-3.4 (Code quality)

Week 3: Testing
├── Day 1-2: Tasks 4.1-4.3 (Move & Rust tests)
└── Day 3-4: Task 4.4 (TypeScript tests - parallel with SDK)

Week 4: TypeScript SDK
├── Day 1-3: Task 5.1 (IBE implementation)
├── Day 4: Task 5.2 (Transcript parsing)
└── Day 5: Task 5.3 (Update tests)

Post-MVP: Future Enhancements (as needed)
└── Tasks 6.1-6.4
```

---

## Risk Assessment

| Risk                                               | Probability | Impact | Mitigation                                |
| -------------------------------------------------- | ----------- | ------ | ----------------------------------------- |
| BLS12-381 compatibility issues between Rust and TS | Medium      | High   | Test extensively, use same curve library  |
| Validator set changes break reveals                | Medium      | High   | Implement historical threshold (Task 1.2) |
| Performance issues with large validator sets       | Low         | Medium | Profile and optimize aggregation          |
| BCS parsing errors in TypeScript                   | Medium      | Medium | Generate deserializers from Move types    |
| Gt serialization format changes                    | Low         | High   | Document and lock format                  |

---

## Acceptance Criteria Checklist

### Phase 1 Complete

- [ ] Invalid shares are rejected at submission time
- [ ] Invalid shares don't count toward threshold
- [ ] Threshold is stored per interval at creation time
- [ ] Reveal uses historical threshold, not current
- [ ] Timelock DKG uses Topic::TIMELOCK

### Phase 2 Complete

- [ ] Interval validation prevents reveals for future/non-existent intervals
- [ ] DKG sessions are cleaned up after completion
- [ ] Share selection is documented and asserted

### Phase 3 Complete

- [ ] No unused variables in Move code
- [ ] Threshold fallback is properly handled
- [ ] Gt hashing is documented
- [ ] Spec file has basic coverage

### Phase 4 Complete

- [ ] Tests exist for invalid share rejection
- [ ] Tests exist for threshold edge cases
- [ ] Smoke tests validate full flow
- [ ] TypeScript tests use real crypto

### Phase 5 Complete

- [ ] TypeScript can compute timelock identity
- [ ] TypeScript can encrypt messages with MPK
- [ ] TypeScript can decrypt messages with revealed DK
- [ ] End-to-end test passes with real chain

---

## Commands Reference

### Building

```bash
# Build Move framework
cd aptos-move/framework/aptos-framework
cargo build

# Run Move tests
cargo test -p aptos-framework

# Build full node
cargo build -p aptos-node

# Build TypeScript
cd atomica/timelock-tests
bun install
bun run build
```

### Testing

```bash
# Move unit tests
cargo test -p aptos-framework timelock

# E2E Move tests
cargo test -p aptos-e2e-move-tests timelock

# Smoke tests
cargo test -p smoke-test timelock

# TypeScript tests
cd atomica/timelock-tests
bun test
```

### Running Local Testnet

```bash
cd atomica/docker-test-harness
docker compose up -d
```

---

_End of Development Plan_
