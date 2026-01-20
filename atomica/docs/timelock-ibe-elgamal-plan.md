# Timelock Encryption Implementation Plan

## IBE + ElGamal PVSS Integration

## 1. Overview

This document outlines the implementation of **Timelock Encryption** using Identity-Based Encryption (IBE) with ElGamal PVSS on the Aptos blockchain.

**Core Concept**: Users encrypt messages for a future time. Validators collectively hold shares of the master secret key. After the deadline passes, validators submit their shares which are aggregated on-chain to reveal the decryption key.

## 2. Architecture

### Components Status

| Component                | Location                                      | Status                           |
| ------------------------ | --------------------------------------------- | -------------------------------- | ----------- |
| IBE Cryptography         | `crates/aptos-dkg/src/ibe/mod.rs`             | IBE DKG Protocol                 | ✅ Complete |
| `dkg/src/ibe_dkg.rs`     | ✅ Complete                                   |
| DKG Manager              | `dkg/src/dkg_manager/mod.rs`                  | ✅ Complete                      |
| Epoch Manager            | `dkg/src/epoch_manager.rs`                    | ✅ Complete (with TODO comments) |
| Move: timelock.move      | `aptos-move/framework/.../timelock.move`      | ✅ Core Logic                    |
| Move: threshold_dsa.move | `aptos-move/framework/.../threshold_dsa.move` | ✅ Share Aggregation             |
| TypeScript SDK           | `atomica/timelock-tests/src/`                 | ✅ Crypto + Tests                |
| Rust Smoke Tests         | `testsuite/smoke-test/src/timelock/`          | ✅ 30+ tests                     |

### Cryptographic Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Client         │     │  Validators      │     │  On-Chain       │
│  (Encrypt)      │     │  (Key Gen)       │     │  (Aggregate)    │
├─────────────────┤     ├──────────────────┤     ├─────────────────┤
│ 1. Fetch MPK    │     │ 1. Run DKG       │     │ 1. Collect      │
│ 2. Compute ID   │     │ 2. Get SK share  │     │    shares       │
│ 3. IBE Encrypt  │────▶│ 3. Derive DK_i   │────▶│ 2. Aggregate    │
│                 │     │ 4. Submit share  │     │ 3. Publish DK   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## 3. Identity-Based Encryption Layer

### Current Implementation (`crates/aptos-dkg/src/ibe/mod.rs`)

| Function                                  | Purpose                          |
| ----------------------------------------- | -------------------------------- |
| `ibe_encrypt(mpk, identity, message)`     | Encrypt using MPK and identity   |
| `ibe_decrypt(dk, ciphertext)`             | Decrypt using decryption key     |
| `derive_decryption_key(msk, identity)`    | Derive DK from MSK and identity  |
| `compute_timelock_identity(id, deadline)` | Compute canonical identity bytes |
| `serialize_g1/g2`, `deserialize_g1/g2`    | Point serialization              |

### Identity Format

```
Identity = Keccak256("timelock_id:{timelock_id}:deadline_timestamp_microseconds:{deadline}")
```

- **timelock_id**: Unique u64 identifier assigned by the chain
- **deadline**: Unix timestamp in microseconds when decryption becomes available
- Output: 32-byte hash used as input to `hash_to_curve`

### IBE DKG (`dkg/src/ibe_dkg.rs`)

Implements Weighted DAS PVSS extended for IBE:

| Function                           | Purpose                                          |
| ---------------------------------- | ------------------------------------------------ |
| `IbeTranscript::deal()`            | Generate transcript with encrypted scalar shares |
| `IbeTranscript::verify()`          | Verify transcript validity                       |
| `IbeTranscript::aggregate_with()`  | Combine transcripts                              |
| `decrypt_own_share()`              | Decrypt validator's secret key shares            |
| `reconstruct_secret_from_shares()` | Reconstruct master secret                        |

## 4. On-Chain Module Design

### Module: `aptos_framework::timelock`

#### State Structure

```move
struct TimelockState has key {
    next_timelock_id: u64,
    pending_deadlines: vector<u64>,
    deadline_to_ids: Table<u64, vector<u64>>,
    id_to_deadline: Table<u64, u64>,
    shares: Table<u64, vector<DecryptionKeyShare>>,
    decryption_keys: Table<u64, vector<u8>>,
}
```

#### Key Functions

| Function                         | Access    | Description                   |
| -------------------------------- | --------- | ----------------------------- |
| `initialize()`                   | framework | Initialize timelock state     |
| `register(deadline)`             | public    | Register new timelock, get ID |
| `on_new_block()`                 | internal  | Process passed deadlines      |
| `publish_decryption_key_share()` | validator | Submit validator's key share  |
| `get_decryption_key(id)`         | view      | Query revealed decryption key |

#### Events

| Event                     | Trigger          | Fields                        |
| ------------------------- | ---------------- | ----------------------------- |
| `TimelockRegisteredEvent` | `register()`     | timelock_id, deadline         |
| `RequestRevealEvent`      | `on_new_block()` | deadline, timelock_ids        |
| `SecretRevealedEvent`     | Threshold met    | timelock_id, deadline, secret |

### Module: `aptos_framework::threshold_dsa`

#### Functions for Timelock

```move
verify_timelock_share(share_bytes: vector<u8>): bool
aggregate_timelock_shares(
    share_bytes_list: &vector<vector<u8>>,
    validator_indices: &vector<u64>,
    total_validators: u64
): vector<u8>
```

## 5. Validator Node Implementation

### Epoch Manager (`dkg/src/epoch_manager.rs`)

The epoch manager handles all timelock-related events:

#### Event Handlers

```rust
fn on_dkg_start_notification(&mut self, notification: EventNotification) {
    // Handles: DKGStartEvent, StartKeyGenEvent, MasterPublicKeyPublishedEvent,
    //         RequestRevealEvent, TimelockRegisteredEvent, SecretRevealedEvent
}
```

#### Share Storage

```rust
fn store_timelock_share(&mut self, epoch: u64, share: &[u8]) -> Result<()>
fn retrieve_timelock_share(&self, epoch: u64) -> Result<Vec<u8>>
```

- Stores shares in persistent `SafetyStorage`
- Survives node restarts

#### Share Submission (`reveal_one_timelock`)

**This is the core timelock reveal logic** (lines 879-967 in epoch_manager.rs):

```rust
fn reveal_one_timelock(&self, timelock_id: u64, deadline: u64) {
    // 1. Retrieve secret share for MPK epoch (constant 1)
    let share_bytes = self.retrieve_timelock_share(1)?;

    // 2. Deserialize IbeShare (Vec<IbeShare>)
    let shares: Vec<IbeShare> = bcs::from_bytes(&share_bytes)?;
    let dealer_sk_share = shares[0].as_scalar();

    // 3. Compute Identity
    let identity = compute_timelock_identity(timelock_id, deadline);

    // 4. Derive Decryption Key Share: DK_i = SK_i * H(ID)
    let dk_share_g1 = derive_decryption_key(dealer_sk_share, &identity)?;

    // 5. Serialize and submit via ValidatorTransaction
    let share = DecryptionKeyShare {
        timelock_id,
        author: self.my_addr,
        validator_idx: my_index as u64,
        share: dk_bytes,
    };
    let txn = ValidatorTransaction::TimelockShare(share);
    self.vtxn_pool.put(Topic::TIMELOCK, Arc::new(txn), None);
}
```

### Key Storage Integration

Shares are stored in `PersistentSafetyStorage`:

```rust
key_storage.set_timelock_share(epoch, share.to_vec())
key_storage.get_timelock_share(epoch)
```

## 6. Client API

### Encryption

```typescript
// 1. Fetch MPK from chain
const mpk = await queries.getMasterPublicKey(interval);

// 2. Compute identity
const identity = IBECrypto.computeTimelockIdentity(timelockId, deadline);

// 3. Encrypt
const ciphertext = IBECrypto.ibeEncrypt(mpk, identity, message);
```

### Decryption

```typescript
// 1. Wait for deadline to pass
await waiters.waitForDeadline(deadline);

// 2. Fetch decryption key
const dk = await queries.getDecryptionKey(timelockId);

// 3. Decrypt
const plaintext = IBECrypto.ibeDecrypt(dk, identity, ciphertext);
```

## 7. Complete Flow

### Phase 1: Setup (DKG)

1. **Initialization**: `timelock::initialize()` called at genesis
2. **Start DKG**: `StartKeyGenEvent` emitted → `start_timelock_dkg()` called
3. **DKG Execution**: `DKGManager<IbeDKG>` runs distributed key generation
4. **Publish MPK**: Transcript aggregated, MPK published via `publish_master_public_key()`
5. **Extract Shares**: `process_timelock_key_published()`:
   - Deserializes transcript
   - Calls `decrypt_own_share()` to get IbeShare
   - Stores in persistent safety storage

### Phase 2: Registration

1. **Client**: Calls `timelock::register(deadline)` → gets `timelock_id`
2. **Chain**: Stores `id_to_deadline`, adds to `pending_deadlines`
3. **Event**: Emits `TimelockRegisteredEvent`

### Phase 3: Encryption (Client)

1. Fetch MPK from chain (via `threshold_dsa::get_master_public_key`)
2. Compute identity: `Keccak256("timelock_id:{id}:deadline:{deadline}")`
3. Encrypt message using `ibe_encrypt(mpk, identity, message)`
4. Store/send ciphertext

### Phase 4: Deadline Detection

1. `on_new_block()` called each block
2. Checks `pending_deadlines` for passed deadlines
3. For each passed deadline, emits `RequestRevealEvent(deadline, [timelock_ids])`

### Phase 5: Share Submission (Validators)

1. **Event Received**: `process_request_reveal()` called
2. **For each timelock_id**: `reveal_one_timelock()` called
3. **Retrieve Share**: Load from persistent storage
4. **Derive Share**: `DK_i = SK_i * H(ID)`
5. **Submit**: Create `ValidatorTransaction::TimelockShare`
6. **Pool**: Transaction added to validator transaction pool

### Phase 6: On-Chain Aggregation

1. **Collect**: `publish_decryption_key_share()` called by validators
2. **Store**: Shares accumulated in `TimelockState.shares[timelock_id]`
3. **Threshold Check**: When `shares.len() >= threshold`:
4. **Aggregate**: Call `aggregate_timelock_shares()` with Lagrange coefficients
5. **Publish**: Store aggregated DK, emit `SecretRevealedEvent`

### Phase 7: Decryption (Client)

1. **Wait**: Poll `get_decryption_key(timelock_id)` or listen for event
2. **Fetch**: Retrieve revealed DK bytes
3. **Decrypt**: Call `ibe_decrypt(dk, ciphertext)`
4. **Verify**: Check plaintext matches expected

## 8. Testing Strategy

### Current Test Coverage

| Category             | Tests                              | Status         |
| -------------------- | ---------------------------------- | -------------- |
| Registration         | ID assignment, deadline storage    | ✅             |
| Deadline Processing  | Sorted deadlines, batch processing | ✅             |
| Event Emission       | All event types                    | ✅             |
| MPK Publication      | DKG → MPK flow                     | ✅             |
| Share Aggregation    | Lagrange, threshold                | ✅             |
| **E2E IBE**          | Full encrypt→wait→decrypt          | ⚠️ See notes   |
| **Share Submission** | Validator auto-submit              | ✅ Implemented |

### Test Files

- `testsuite/smoke-test/src/timelock/test_ibe.rs` - Main E2E test
- `testsuite/smoke-test/src/timelock/test_share_submission.rs` - Share tests
- `testsuite/smoke-test/src/timelock/test_threshold_*.rs` - Threshold tests
- `atomica/timelock-tests/test/ibe-e2e.test.ts` - TypeScript E2E

### Known Test Issues

The E2E tests may fail due to:

1. Timing issues (deadline + buffer time)
2. Transaction pool capacity
3. Network delays in test swarm

## 9. Security Considerations

1. **DKG Security**: Threshold scheme ensures no single validator can decrypt early
2. **Liveness**: Threshold of 2f+1 required for decryption
3. **Front-running**: Deadlines are deterministic, no MEV opportunity
4. **Replay Attacks**: Each timelock has unique ID, cannot be reused
5. **Persistent Storage**: Shares survive node restarts

## 10. Known Issues / TODOs

From code comments in `epoch_manager.rs`:

- **TODO Line 700-708**: After DKG completes, need to detect when transcript is finalized on-chain
- **TODO Line 971**: Add persistent storage (already implemented via `key_storage`)
- **TODO Line 873-876**: Could pre-compute identity strings for registered timelocks
- **Line 267**: Currently just logs `SecretRevealedEvent` - could stop trying to reveal

### Potential Improvements

1. **Chaum-Pedersen proofs**: Add zero-knowledge proofs for share verification
2. **Rate limiting**: Prevent DoS attacks on share submission
3. **Key rotation**: Handle epoch changes gracefully
4. **Recovery**: Support threshold key recovery

## 11. References

- [Boneh-Franklin IBE](https://crypto.stanford.edu/~dabo/papers/ibe.pdf)
- [BLS01: BLS Signatures](https://crypto.stanford.edu/~dabo/papers/bfls.pdf)
- Weighted DAS PVSS: See `aptos_dkg::pvss::das`
