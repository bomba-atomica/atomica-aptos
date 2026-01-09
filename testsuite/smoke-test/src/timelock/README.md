# Timelock IBE Smoke Test Suite

## Overview

This test suite validates the end-to-end operation of the **Identity-Based Encryption (IBE) Timelock Protocol**. The protocol enables time-locked encryption where messages can only be decrypted after a specified deadline, enforced by threshold cryptography across the validator set.

---

## Scope

### What This Test Suite SHOULD Check

These are runtime behaviors that can be verified by observing on-chain state and validator behavior in a local swarm:

1. **DKG Protocol Execution**
   - DKG starts automatically on genesis/initialization
   - Validators produce and aggregate transcripts
   - DKG completes within expected time bounds
   - Transcript is stored on-chain in `DKGState`

2. **MPK Publication**
   - Master Public Key is derived from DKG transcript
   - MPK is published to `threshold_dsa::MasterPubKeys` table
   - `MasterPublicKeyPublishedEvent` is emitted
   - MPK is queryable via view function

3. **Timelock Registration**
   - Sequential ID assignment (starting from 2, since 1 = MPK_ID)
   - Deadline stored correctly in `id_to_deadline` mapping
   - Deadline inserted into sorted `pending_deadlines` vector
   - `TimelockRegisteredEvent` is emitted with correct fields

4. **Event Emission & Processing**
   - `StartKeyGenEvent` emitted on initialization
   - `RequestRevealEvent` emitted when `block_time >= deadline`
   - Events contain correct payloads (timelock_ids, deadlines, etc.)
   - Validators observe and react to events

5. **Deadline Detection**
   - `on_new_block()` correctly identifies passed deadlines
   - Multiple timelocks with same deadline are batched
   - Deadlines are processed in chronological order

6. **DK Share Submission**
   - Validators submit shares after `RequestRevealEvent`
   - Shares are valid G1 points (48 bytes compressed)
   - Shares are stored in `TimelockState.shares[timelock_id]`
   - Duplicate shares from same validator are rejected

7. **Threshold Enforcement**
   - Aggregation occurs only when `shares.len() >= threshold`
   - With f Byzantine validators stopped, aggregation still succeeds if threshold met
   - With too many validators stopped, aggregation does NOT occur (secret not revealed)

8. **Lagrange Aggregation**
   - Aggregated DK is mathematically correct (enables decryption)
   - Result stored in `TimelockState.decryption_keys[timelock_id]`
   - `SecretRevealedEvent` emitted with correct DK bytes

9. **End-to-End IBE Flow**
   - Encrypt with MPK → Register timelock → Wait for deadline → Decrypt with revealed DK
   - Decrypted plaintext matches original

10. **Share Storage & Retrieval (Validators)**
    - Validators store their scalar shares after MPK publication
    - Shares persist across validator restarts
    - Shares are correctly retrieved for DK derivation

### What This Test Suite DOES NOT Need to Check

These are covered by unit tests, formal verification, or other test suites:

1. **Cryptographic Correctness of Primitives**
   - BLS signature verification (covered by `aptos-crypto` tests)
   - Hash-to-curve implementation (covered by `aptos-dkg` tests)
   - Pairing operations (covered by `blst` library tests)
   - Lagrange coefficient computation (covered by `threshold_dsa` Move unit tests)

2. **Move Language Semantics**
   - Table operations work correctly
   - Vector sorting is correct
   - Event emission mechanics work
   - Resource lifecycle (move semantics)

3. **Consensus & Block Production**
   - Validators reach consensus
   - Block timestamps advance correctly
   - Epoch transitions work
   - Validator transactions are included in blocks

4. **Network Layer**
   - Reliable broadcast works
   - Validators can communicate
   - Transaction propagation works

5. **Storage Layer**
   - RocksDB operations work
   - State sync works correctly

### What Is IMPOSSIBLE for This Test Suite to Check

Due to limitations of the smoke test environment:

1. **Byzantine Behavior Simulation**
   - Malicious validators submitting invalid shares (would require code modification)
   - Validators colluding to reveal early (requires compromised keys)
   - Sybil attacks on DKG (requires network-level simulation)

2. **Long-Term Timing Attacks**
   - Deadline manipulation via block timestamp skew (requires adversarial block producers)
   - Clock drift attacks across validators

3. **Key Leakage Detection**
   - Cannot verify that scalar shares are not leaked to disk/logs
   - Cannot verify memory is properly zeroed after use

4. **Real Network Conditions**
   - Network partitions
   - High latency between validators
   - Message reordering

5. **Production-Scale Behavior**
   - Performance with 100+ validators
   - Behavior under high transaction load
   - Storage growth over many epochs

---

## Protocol Flow Summary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TIMELOCK IBE PROTOCOL                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 1: INITIALIZATION                                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Genesis → timelock::initialize() → emit StartKeyGenEvent            │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 2: DKG FOR MPK                                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Validators observe StartKeyGenEvent                                  │  │
│  │    → Generate random polynomial shares                               │  │
│  │    → Broadcast transcripts via reliable broadcast                    │  │
│  │    → Aggregate transcripts → derive MPK (G2, 96 bytes)              │  │
│  │    → Submit ValidatorTransaction::TimelockDKGResult                  │  │
│  │    → VM calls threshold_dsa::publish_master_public_key()            │  │
│  │    → emit MasterPublicKeyPublishedEvent                              │  │
│  │    → Validators store their scalar shares for later reveal           │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 3: CLIENT ENCRYPTION                                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Client reads MPK from threshold_dsa::get_master_public_key(1)        │  │
│  │ Client computes identity = hash(timelock_id, deadline)               │  │
│  │ Client encrypts: ciphertext = IBE.Encrypt(MPK, identity, plaintext)  │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 4: TIMELOCK REGISTRATION                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Client calls timelock::register(deadline)                            │  │
│  │    → Assigns unique timelock_id (starting from 2)                    │  │
│  │    → Stores deadline in mappings                                     │  │
│  │    → emit TimelockRegisteredEvent                                    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 5: DEADLINE DETECTION                                                │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ on_new_block() checks: block_timestamp >= pending_deadlines[0]?      │  │
│  │    → If yes: emit RequestRevealEvent { timelock_ids }                │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 6: DK SHARE GENERATION                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Validators observe RequestRevealEvent                                │  │
│  │    → Retrieve stored scalar share                                    │  │
│  │    → Compute identity = hash(timelock_id, deadline)                  │  │
│  │    → Derive DK share: dk_i = scalar_i × H(identity) (G1, 48 bytes)  │  │
│  │    → Submit ValidatorTransaction::TimelockShare                      │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 7: THRESHOLD AGGREGATION                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ VM calls timelock::publish_decryption_key_share()                    │  │
│  │    → Stores share in TimelockState.shares[timelock_id]               │  │
│  │    → Checks: shares.len() >= threshold?                              │  │
│  │    → If yes: Lagrange interpolation → DK = Σ(λ_i × dk_i)            │  │
│  │    → Stores DK in TimelockState.decryption_keys[timelock_id]         │  │
│  │    → emit SecretRevealedEvent                                        │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                     │                                       │
│                                     ▼                                       │
│  PHASE 8: CLIENT DECRYPTION                                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │ Client polls timelock::get_decryption_key(timelock_id)               │  │
│  │ Client decrypts: plaintext = IBE.Decrypt(DK, ciphertext)             │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Atomic Verification Breakdown

Each verification represents a single testable assertion about the system:

### Initialization Verifications

| ID  | Verification                                          | On-Chain State to Check                 |
| --- | ----------------------------------------------------- | --------------------------------------- |
| I1  | `TimelockState` resource exists at `@aptos_framework` | Resource exists                         |
| I2  | `next_timelock_id` initialized to 2                   | `TimelockState.next_timelock_id == 2`   |
| I3  | `mpk_dkg_started` is set to true after init           | `TimelockState.mpk_dkg_started == true` |
| I4  | `threshold_dsa::State` resource exists                | Resource exists                         |
| I5  | `StartKeyGenEvent` is emitted with `epoch=1`          | Event query returns event               |

### DKG Verifications

| ID  | Verification                              | On-Chain State to Check               |
| --- | ----------------------------------------- | ------------------------------------- |
| D1  | DKG transcript produced by end of epoch 1 | `DKGState.last_completed.is_some()`   |
| D2  | Transcript is non-empty                   | `last_completed.transcript.len() > 0` |
| D3  | Transcript targets correct epoch          | `last_completed.target_epoch() == 2`  |
| D4  | Transcript contains valid MPK bytes       | Deserialize and validate G2 point     |

### MPK Publication Verifications

| ID  | Verification                                 | On-Chain State to Check               |
| --- | -------------------------------------------- | ------------------------------------- |
| M1  | MPK stored in `threshold_dsa` for interval 1 | `get_master_public_key(1).is_some()`  |
| M2  | MPK is valid G2 point (96 bytes)             | Deserialize as G2, check not identity |
| M3  | `MasterPublicKeyPublishedEvent` emitted      | Event query returns event             |
| M4  | Event contains correct MPK bytes             | Event payload matches stored MPK      |

### Registration Verifications

| ID  | Verification                                        | On-Chain State to Check           |
| --- | --------------------------------------------------- | --------------------------------- |
| R1  | First registration returns ID 2                     | Return value from `register()`    |
| R2  | Sequential registrations increment ID               | IDs are 2, 3, 4, ...              |
| R3  | `id_to_deadline` mapping updated                    | `get_deadline(id) == deadline`    |
| R4  | Deadline added to `pending_deadlines`               | View function or state inspection |
| R5  | `TimelockRegisteredEvent` emitted                   | Event query                       |
| R6  | Event contains correct `timelock_id` and `deadline` | Event payload validation          |
| R7  | Registration with past deadline fails               | Transaction reverts               |
| R8  | Registration before MPK ready fails (if enforced)   | Transaction reverts               |

### Deadline Detection Verifications

| ID  | Verification                                        | On-Chain State to Check        |
| --- | --------------------------------------------------- | ------------------------------ |
| DD1 | `RequestRevealEvent` emitted when `now >= deadline` | Event query after deadline     |
| DD2 | Event contains correct `timelock_ids`               | Event payload validation       |
| DD3 | Deadline removed from `pending_deadlines`           | View function shows removal    |
| DD4 | Multiple timelocks with same deadline batched       | Single event with multiple IDs |
| DD5 | Earlier deadlines processed before later ones       | Event ordering                 |

### Share Submission Verifications

| ID  | Verification                                  | On-Chain State to Check    |
| --- | --------------------------------------------- | -------------------------- |
| S1  | Shares appear in `TimelockState.shares[id]`   | Query share count          |
| S2  | Each share is 48 bytes (compressed G1)        | Share data validation      |
| S3  | Share count increases over time               | Polling shows growth       |
| S4  | Duplicate shares from same validator rejected | Share count doesn't double |
| S5  | Shares from non-validators rejected           | Transaction reverts        |

### Threshold Enforcement Verifications

| ID  | Verification                                         | On-Chain State to Check            |
| --- | ---------------------------------------------------- | ---------------------------------- |
| T1  | Secret NOT revealed with fewer than threshold shares | `get_decryption_key(id).is_none()` |
| T2  | Secret revealed exactly when threshold reached       | Timing of revelation               |
| T3  | Threshold is `(n * 2 / 3) + 1`                       | Compare to config                  |

### Aggregation Verifications

| ID  | Verification                                  | On-Chain State to Check            |
| --- | --------------------------------------------- | ---------------------------------- |
| A1  | Aggregated DK stored in `decryption_keys[id]` | `get_decryption_key(id).is_some()` |
| A2  | Aggregated DK is 48 bytes (compressed G1)     | Length check                       |
| A3  | `SecretRevealedEvent` emitted                 | Event query                        |
| A4  | Event contains correct DK bytes               | Event payload validation           |

### E2E Cryptographic Verifications

| ID  | Verification                                  | Test Method                          |
| --- | --------------------------------------------- | ------------------------------------ |
| E1  | Ciphertext produced with MPK is valid         | Encryption succeeds                  |
| E2  | Decryption with correct DK recovers plaintext | `decrypt(dk, ct) == plaintext`       |
| E3  | Decryption with wrong DK fails                | `decrypt(wrong_dk, ct) != plaintext` |
| E4  | Identity mismatch prevents decryption         | Different timelock_id fails          |

---

## Test File Recommendations

### Naming Convention

- `test_[phase]_[component].rs` - Atomic tests for specific phase/component
- `test_[phase]_[phase].rs` - Integration tests spanning phases
- `test_e2e_[scenario].rs` - Full end-to-end scenarios

---

### Phase 1: Initialization Tests

#### `test_init_state.rs`

Tests that verify the initial state is correctly set up.

| Test Name                         | Description                                        | Verifications |
| --------------------------------- | -------------------------------------------------- | ------------- |
| `test_timelock_state_exists`      | Verify `TimelockState` resource created at genesis | I1            |
| `test_initial_timelock_id`        | Verify `next_timelock_id` starts at 2              | I2            |
| `test_mpk_dkg_started_flag`       | Verify `mpk_dkg_started` is true after init        | I3            |
| `test_threshold_dsa_state_exists` | Verify `threshold_dsa::State` resource exists      | I4            |

#### `test_init_events.rs`

Tests for initialization event emission.

| Test Name                         | Description                                             | Verifications |
| --------------------------------- | ------------------------------------------------------- | ------------- |
| `test_start_keygen_event_emitted` | Query events API for `StartKeyGenEvent`                 | I5            |
| `test_start_keygen_event_payload` | Verify event has `epoch=1` and correct threshold config | I5            |

---

### Phase 2: DKG Tests

#### `test_dkg_completion.rs`

Tests that verify DKG completes successfully.

| Test Name                          | Description                                         | Verifications |
| ---------------------------------- | --------------------------------------------------- | ------------- |
| `test_dkg_completes_by_epoch_2`    | Wait for epoch 2, verify `last_completed.is_some()` | D1            |
| `test_dkg_transcript_non_empty`    | Verify transcript has non-zero length               | D2            |
| `test_dkg_transcript_target_epoch` | Verify `target_epoch() == 2`                        | D3            |

#### `test_dkg_transcript_validity.rs`

Tests that verify the DKG transcript contains valid cryptographic data.

| Test Name                            | Description                                           | Verifications |
| ------------------------------------ | ----------------------------------------------------- | ------------- |
| `test_transcript_contains_valid_mpk` | Deserialize transcript, extract and validate MPK      | D4            |
| `test_transcript_mpk_is_g2_point`    | Verify MPK is valid G2 point (96 bytes, not identity) | D4            |

---

### Phase 3: MPK Publication Tests

#### `test_mpk_storage.rs`

Tests that verify MPK is correctly stored in `threshold_dsa`.

| Test Name                             | Description                                                     | Verifications |
| ------------------------------------- | --------------------------------------------------------------- | ------------- |
| `test_mpk_available_in_threshold_dsa` | Call `get_master_public_key(1)`, verify `Some`                  | M1            |
| `test_mpk_is_96_bytes`                | Verify MPK length is 96 bytes                                   | M2            |
| `test_mpk_is_valid_g2`                | Deserialize as G2 point, verify not identity                    | M2            |
| `test_mpk_matches_transcript`         | Compare MPK in `threshold_dsa` with MPK derived from transcript | M2, D4        |

#### `test_mpk_events.rs`

Tests for MPK publication event emission.

| Test Name                          | Description                                          | Verifications |
| ---------------------------------- | ---------------------------------------------------- | ------------- |
| `test_mpk_published_event_emitted` | Query events API for `MasterPublicKeyPublishedEvent` | M3            |
| `test_mpk_published_event_payload` | Verify event contains `id=1` and correct MPK bytes   | M4            |

---

### Phase 4: Registration Tests

#### `test_registration_ids.rs`

Tests for timelock ID assignment.

| Test Name                                 | Description                                      | Verifications |
| ----------------------------------------- | ------------------------------------------------ | ------------- |
| `test_first_registration_returns_id_2`    | Register once, verify returned ID is 2           | R1            |
| `test_sequential_id_assignment`           | Register 5 times, verify IDs are 2,3,4,5,6       | R2            |
| `test_concurrent_registration_unique_ids` | Multiple concurrent registrations get unique IDs | R2            |

#### `test_registration_state.rs`

Tests for registration state updates.

| Test Name                               | Description                                           | Verifications |
| --------------------------------------- | ----------------------------------------------------- | ------------- |
| `test_deadline_stored_correctly`        | Verify `get_deadline(id)` returns registered deadline | R3            |
| `test_deadline_in_pending_list`         | Verify deadline appears in `pending_deadlines`        | R4            |
| `test_multiple_timelocks_same_deadline` | Two registrations with same deadline both tracked     | R3, R4        |

#### `test_registration_events.rs`

Tests for registration event emission.

| Test Name                         | Description                                            | Verifications |
| --------------------------------- | ------------------------------------------------------ | ------------- |
| `test_registration_event_emitted` | Query for `TimelockRegisteredEvent` after registration | R5            |
| `test_registration_event_payload` | Verify event has correct `timelock_id` and `deadline`  | R6            |

#### `test_registration_validation.rs`

Tests for registration input validation.

| Test Name                     | Description                              | Verifications |
| ----------------------------- | ---------------------------------------- | ------------- |
| `test_past_deadline_rejected` | Registration with `deadline < now` fails | R7            |
| `test_zero_deadline_rejected` | Registration with `deadline = 0` fails   | R7            |

---

### Phase 5: Deadline Detection Tests

#### `test_deadline_trigger.rs`

Tests for deadline detection and event emission.

| Test Name                                  | Description                                                 | Verifications |
| ------------------------------------------ | ----------------------------------------------------------- | ------------- |
| `test_request_reveal_emitted_on_deadline`  | After deadline passes, `RequestRevealEvent` appears         | DD1           |
| `test_request_reveal_contains_timelock_id` | Event payload contains correct `timelock_ids`               | DD2           |
| `test_deadline_removed_from_pending`       | After processing, deadline no longer in `pending_deadlines` | DD3           |

#### `test_deadline_batching.rs`

Tests for batch processing of deadlines.

| Test Name                    | Description                                                | Verifications |
| ---------------------------- | ---------------------------------------------------------- | ------------- |
| `test_same_deadline_batched` | Two timelocks with same deadline → one event with both IDs | DD4           |
| `test_deadline_ordering`     | Earlier deadlines processed before later ones              | DD5           |

---

### Phase 6: Share Submission Tests

#### `test_share_submission.rs`

Tests for DK share submission by validators.

| Test Name                           | Description                                | Verifications |
| ----------------------------------- | ------------------------------------------ | ------------- |
| `test_shares_appear_after_deadline` | After deadline, shares accumulate in state | S1            |
| `test_share_is_48_bytes`            | Each share is exactly 48 bytes             | S2            |
| `test_share_count_increases`        | Polling shows share count growing          | S3            |

#### `test_share_validation.rs`

Tests for share validation rules.

| Test Name                       | Description                                            | Verifications |
| ------------------------------- | ------------------------------------------------------ | ------------- |
| `test_duplicate_share_rejected` | Same validator submitting twice doesn't increase count | S4            |
| `test_share_is_valid_g1`        | Share deserializes as valid G1 point                   | S2            |

---

### Phase 7: Threshold & Aggregation Tests

#### `test_threshold_below.rs`

Tests behavior when threshold is NOT met.

| Test Name                        | Description                                           | Verifications |
| -------------------------------- | ----------------------------------------------------- | ------------- |
| `test_no_reveal_with_one_share`  | With 3 validators, 1 share → no DK                    | T1            |
| `test_no_reveal_below_threshold` | With 3 validators, stop 2 → no DK revealed            | T1            |
| `test_partial_shares_stored`     | Below threshold, shares are stored but no aggregation | S1, T1        |

#### `test_threshold_met.rs`

Tests behavior when threshold IS met.

| Test Name                         | Description                               | Verifications |
| --------------------------------- | ----------------------------------------- | ------------- |
| `test_reveal_at_threshold`        | With 3 validators, 2 shares → DK revealed | T2            |
| `test_reveal_with_all_validators` | All validators submit → DK revealed       | T2            |
| `test_threshold_calculation`      | Verify threshold is `(n * 2 / 3) + 1`     | T3            |

#### `test_aggregation.rs`

Tests for Lagrange aggregation correctness.

| Test Name                            | Description                                          | Verifications |
| ------------------------------------ | ---------------------------------------------------- | ------------- |
| `test_aggregated_dk_stored`          | After threshold, `get_decryption_key(id)` returns DK | A1            |
| `test_aggregated_dk_is_48_bytes`     | DK is exactly 48 bytes                               | A2            |
| `test_secret_revealed_event_emitted` | `SecretRevealedEvent` appears after aggregation      | A3            |
| `test_secret_revealed_event_payload` | Event contains correct `timelock_id` and DK bytes    | A4            |

---

### Phase 8: E2E Cryptographic Tests

#### `test_ibe_encrypt.rs`

Tests for client-side encryption.

| Test Name                        | Description                                      | Verifications |
| -------------------------------- | ------------------------------------------------ | ------------- |
| `test_encrypt_with_mpk_succeeds` | Encryption with on-chain MPK produces ciphertext | E1            |
| `test_ciphertext_format`         | Ciphertext has expected structure (U, V)         | E1            |

#### `test_ibe_decrypt.rs`

Tests for client-side decryption.

| Test Name                           | Description                                             | Verifications |
| ----------------------------------- | ------------------------------------------------------- | ------------- |
| `test_decrypt_with_revealed_dk`     | Decryption with on-chain DK recovers plaintext          | E2            |
| `test_decrypt_wrong_dk_fails`       | Decryption with DK from different timelock fails        | E3            |
| `test_decrypt_wrong_identity_fails` | Ciphertext for ID 2 can't be decrypted with DK for ID 3 | E4            |

---

### Integration Tests (Multi-Phase)

#### `test_phase_1_to_2.rs` (Init → DKG)

| Test Name                | Description                                | Phases |
| ------------------------ | ------------------------------------------ | ------ |
| `test_init_triggers_dkg` | `StartKeyGenEvent` leads to DKG completion | 1 → 2  |

#### `test_phase_2_to_3.rs` (DKG → MPK Publication)

| Test Name                         | Description                                          | Phases |
| --------------------------------- | ---------------------------------------------------- | ------ |
| `test_dkg_produces_mpk`           | DKG completion leads to MPK in `threshold_dsa`       | 2 → 3  |
| `test_mpk_matches_dkg_transcript` | MPK in `threshold_dsa` matches MPK in DKG transcript | 2 → 3  |

#### `test_phase_4_to_5.rs` (Registration → Deadline Detection)

| Test Name                           | Description                                    | Phases |
| ----------------------------------- | ---------------------------------------------- | ------ |
| `test_registration_to_reveal_event` | Register → wait → `RequestRevealEvent` emitted | 4 → 5  |

#### `test_phase_5_to_7.rs` (Deadline → Aggregation)

| Test Name                           | Description                                            | Phases    |
| ----------------------------------- | ------------------------------------------------------ | --------- |
| `test_reveal_event_to_dk_available` | `RequestRevealEvent` → shares submitted → DK available | 5 → 6 → 7 |

#### `test_phase_3_to_8.rs` (MPK → Decryption)

| Test Name                               | Description                                  | Phases                |
| --------------------------------------- | -------------------------------------------- | --------------------- |
| `test_encrypt_with_mpk_decrypt_with_dk` | Encrypt → register → wait → decrypt succeeds | 3 → 4 → 5 → 6 → 7 → 8 |

---

### Full E2E Tests

#### `test_e2e_happy_path.rs`

| Test Name                            | Description                                                                    | Phases |
| ------------------------------------ | ------------------------------------------------------------------------------ | ------ |
| `test_complete_timelock_flow`        | Full flow: init → DKG → MPK → encrypt → register → deadline → reveal → decrypt | 1-8    |
| `test_multiple_timelocks_sequential` | Three timelocks with staggered deadlines all decrypt correctly                 | 1-8    |
| `test_multiple_timelocks_concurrent` | Three timelocks registered together, all decrypt correctly                     | 1-8    |

#### `test_e2e_edge_cases.rs`

| Test Name                           | Description                                         | Phases |
| ----------------------------------- | --------------------------------------------------- | ------ |
| `test_immediate_deadline`           | Register with deadline 1 second away → still works  | 4-8    |
| `test_many_timelocks_same_deadline` | 10 timelocks with identical deadline → all revealed | 4-8    |

---

### Failure Mode Tests

#### `test_failure_dkg.rs`

| Test Name                | Description                           | Expected |
| ------------------------ | ------------------------------------- | -------- |
| `test_no_mpk_before_dkg` | Query MPK before DKG completes → None | M1 fails |

#### `test_failure_timing.rs`

| Test Name                        | Description                          | Expected |
| -------------------------------- | ------------------------------------ | -------- |
| `test_no_dk_before_deadline`     | Query DK before deadline → None      | A1 fails |
| `test_no_shares_before_deadline` | Query shares before deadline → empty | S1 fails |

#### `test_failure_threshold.rs`

| Test Name                                | Description                             | Expected |
| ---------------------------------------- | --------------------------------------- | -------- |
| `test_validator_failure_blocks_reveal`   | Stop 2 of 3 validators → no DK revealed | T1       |
| `test_validator_recovery_enables_reveal` | Stop 2, restart 1 → DK revealed         | T2       |

---

## Test Helpers

### Required Helper Functions (in `test_helpers.rs`)

```rust
/// Create a swarm configured for timelock testing
async fn create_timelock_swarm(config: TimelockTestConfig) -> (LocalSwarm, Client, ChainId);

/// Wait for DKG to complete (transcript available)
async fn wait_for_dkg_completion(swarm: &LocalSwarm, client: &Client, timeout_secs: u64);

/// Wait for MPK to be published to threshold_dsa
async fn wait_for_mpk_publication(client: &Client, timeout_secs: u64) -> Vec<u8>;

/// Register a timelock and return the assigned ID
async fn register_timelock(swarm: &LocalSwarm, client: &Client, deadline: u64) -> Result<u64>;

/// Wait for a specific timelock's DK to be revealed
async fn wait_for_dk_reveal(client: &Client, timelock_id: u64, timeout_secs: u64) -> Vec<u8>;

/// Query share count for a timelock
async fn get_share_count(client: &Client, timelock_id: u64) -> u64;

/// Query events of a specific type
async fn get_events<T: MoveStructType>(client: &Client, start: u64, limit: u64) -> Vec<T>;

/// Stop a validator node
async fn stop_validator(swarm: &mut LocalSwarm, index: usize);

/// Restart a validator node
async fn restart_validator(swarm: &mut LocalSwarm, index: usize);

/// Get current chain timestamp in microseconds
async fn get_chain_time(client: &Client) -> u64;

/// Force block production (trigger on_new_block)
async fn force_block_production(swarm: &LocalSwarm, client: &Client);
```

---

## Verification Functions (in `mod.rs`)

```rust
/// Verify DKG transcript exists and return it
async fn verify_dkg_transcript(client: &Client) -> Result<DKGTranscript>;

/// Verify MPK is published to threshold_dsa
async fn verify_mpk_on_chain(client: &Client, interval: u64) -> Result<Vec<u8>>;

/// Verify a timelock's deadline is stored
async fn verify_deadline_stored(client: &Client, timelock_id: u64) -> Result<u64>;

/// Verify DK is revealed for a timelock
async fn verify_dk_revealed(client: &Client, timelock_id: u64) -> Result<Vec<u8>>;

/// Verify share count for a timelock
async fn verify_share_count(client: &Client, timelock_id: u64, expected: u64) -> Result<()>;

/// Query specific event type and verify it exists
async fn verify_event_emitted<T: MoveStructType>(client: &Client) -> Result<T>;
```

---

## File Structure

```
testsuite/smoke-test/src/timelock/
├── README.md                          # This document
├── mod.rs                             # Module root + verification functions
├── test_helpers.rs                    # Shared test utilities
│
├── # Phase 1: Initialization
├── test_init_state.rs                 # TimelockState, threshold_dsa::State existence
├── test_init_events.rs                # StartKeyGenEvent emission
│
├── # Phase 2: DKG
├── test_dkg_completion.rs             # DKG completes, transcript exists
├── test_dkg_transcript_validity.rs    # Transcript contains valid MPK
│
├── # Phase 3: MPK Publication
├── test_mpk_storage.rs                # MPK in threshold_dsa
├── test_mpk_events.rs                 # MasterPublicKeyPublishedEvent
│
├── # Phase 4: Registration
├── test_registration_ids.rs           # ID assignment
├── test_registration_state.rs         # State updates
├── test_registration_events.rs        # TimelockRegisteredEvent
├── test_registration_validation.rs    # Input validation
│
├── # Phase 5: Deadline Detection
├── test_deadline_trigger.rs           # RequestRevealEvent emission
├── test_deadline_batching.rs          # Batch processing
│
├── # Phase 6: Share Submission
├── test_share_submission.rs           # Shares appear after deadline
├── test_share_validation.rs           # Share validation rules
│
├── # Phase 7: Threshold & Aggregation
├── test_threshold_below.rs            # Below threshold behavior
├── test_threshold_met.rs              # At/above threshold behavior
├── test_aggregation.rs                # Lagrange aggregation
│
├── # Phase 8: E2E Cryptographic
├── test_ibe_encrypt.rs                # Client encryption
├── test_ibe_decrypt.rs                # Client decryption
│
├── # Integration Tests
├── test_phase_1_to_2.rs               # Init → DKG
├── test_phase_2_to_3.rs               # DKG → MPK
├── test_phase_4_to_5.rs               # Registration → Deadline
├── test_phase_5_to_7.rs               # Deadline → Aggregation
├── test_phase_3_to_8.rs               # MPK → Decrypt (full crypto path)
│
├── # Full E2E
├── test_e2e_happy_path.rs             # Complete happy path scenarios
├── test_e2e_edge_cases.rs             # Edge cases
│
├── # Failure Modes
├── test_failure_dkg.rs                # DKG failure scenarios
├── test_failure_timing.rs             # Timing-related failures
└── test_failure_threshold.rs          # Threshold-related failures
```

---

## Running Tests

**IMPORTANT**: Tests must be run with `--test-threads=1` to prevent conflicts.
Each test spins up a local validator swarm, and concurrent swarms will compete
for ports and resources.

```bash
# Run all timelock tests (REQUIRED: --test-threads=1)
cargo test -p smoke-test timelock:: -- --test-threads=1

# Run a specific phase
cargo test -p smoke-test timelock::test_dkg -- --test-threads=1

# Run a specific test
cargo test -p smoke-test timelock::test_dkg_completion::test_dkg_completes_by_epoch_2 -- --test-threads=1

# Run with logging
RUST_LOG=info cargo test -p smoke-test timelock:: -- --test-threads=1 --nocapture

# Recommended: create an alias
alias timelock-test='cargo test -p smoke-test timelock:: -- --test-threads=1'
```

### Why `--test-threads=1`?

- Each test creates a `LocalSwarm` with multiple validator nodes
- Validators bind to network ports that may conflict between tests
- RocksDB and other storage resources may conflict
- Running sequentially ensures clean state between tests

---

## Known Limitations

1. **MPK Publication Not Yet Implemented**: As of this writing, validators do not automatically publish MPK to `threshold_dsa` after DKG. Tests for M1-M4 will fail until this is implemented.

2. **Share Submission Not Yet Implemented**: Validators do not yet process `RequestRevealEvent` to submit DK shares. Tests for S1-S4, T1-T3, A1-A4 will fail until this is implemented.

3. **Event Query API**: Some event queries may require specific indexer configuration. Tests should include fallback mechanisms or skip conditions.

4. **Timing Sensitivity**: Tests involving deadlines are inherently timing-sensitive. Use generous timeouts and polling intervals.
