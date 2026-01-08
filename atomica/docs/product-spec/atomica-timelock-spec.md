# Atomica Timelock DKG & IBE Specification

**Version:** 1.0
**Last Updated:** January 2, 2026
**Status:** Reference Specification

---

## Table of Contents

1. [Overview](#overview)
2. [Cryptographic Foundations](#cryptographic-foundations)
3. [Architecture](#architecture)
4. [Validator Specification](#validator-specification)
5. [Explorer Integration](#explorer-integration)
6. [dApp Developer Guide](#dapp-developer-guide)
7. [End User Experience](#end-user-experience)
8. [Infrastructure Requirements](#infrastructure-requirements)
9. [Aptos-Provided Infrastructure](#aptos-provided-infrastructure)
10. [Security Considerations](#security-considerations)

---

## Overview

The Atomica Timelock system enables **time-locked encryption** on the Aptos blockchain, where messages can be encrypted such that decryption is only possible after a specific blockchain deadline has elapsed.

### Key Properties

- **Trustless**: No single party can decrypt before the scheduled time
- **Decentralized**: Validators collectively generate and reveal keys via DKG
- **Deterministic**: Encryption keys are derived from blockchain deadlines
- **Threshold-Secure**: Requires 2/3 + 1 validators to reveal secrets

### Use Cases

1. **Sealed Bid Auctions**: Bids remain encrypted until the auction closes
2. **Voting Systems**: Votes hidden until the poll ends
3. **Time-Delayed Transactions**: Execute transactions only after a delay
4. **Fair Randomness**: Commit-reveal schemes with guaranteed reveals

---

## Cryptographic Foundations

### Primitives

**Curve:** BLS12-381 pairing-friendly elliptic curve

**Identity-Based Encryption (IBE):** Boneh-Franklin scheme

**Groups:**

- **G1**: First elliptic curve group (48 bytes compressed)
- **G2**: Second elliptic curve group (96 bytes compressed)
- **Gt**: Target group for pairings

**Pairing Function:** `e: G1 × G2 → Gt`

### Key Derivation

**Master Public Key (MPK):**

```
MPK = s × G2_generator
where s = master secret (generated via DKG)
```

**Identity Derivation:**

```
identity = Keccak256("timelock_id:" || timelock_id || ":deadline_timestamp_microseconds:" || deadline)
```

> [!IMPORTANT]
> The identity is **application-agnostic**. It contains no auction, bid, or other application semantics.
> The `deadline_timestamp_microseconds` is the Unix epoch timestamp when decryption becomes available.

_Note: Uses Keccak256 (original SHA-3 competition winner, same as Ethereum) rather than SHA3-256 (NIST FIPS 202 standard). This is an intentional design choice for developer familiarity and ecosystem tool compatibility. While Aptos Move framework provides multiple hash options (SHA2-256, SHA3-256, Keccak256, BLAKE2b-256), Keccak256 was selected for consistency with the broader blockchain ecosystem. The pre-hashing step provides fixed-length 32-byte identities; cryptographic security comes from the subsequent hash-to-curve operation._

**Decryption Key (DK) for Identity:**

```
Q_id = HashToCurve_G1(identity)
DK = s × Q_id  (G1 point, 48 bytes)
```

**Threshold Reconstruction:**

- Validators hold DK_shares (scalars, 32 bytes)
- On reveal, each validator computes `DK_contribution_i = DK_share_i × Q_id` (G1 point)
- Contract aggregates G1 contributions using Lagrange coefficients
- Result is the full DK (G1 point)

### Encryption Algorithm (Boneh-Franklin IBE)

**Input:** Message M, MPK (G2), identity

**Process:**

1. Generate random scalar `r`
2. Compute `U = r × G2_generator` (96 bytes)
3. Compute `Q_id = HashToCurve_G1(identity)`
4. Compute `g_id = e(Q_id, MPK)^r`
5. Derive symmetric key: `K = Keccak256(g_id)[0..32]`
6. Encrypt: `V = M ⊕ K` (XOR with key expansion)
7. Output: `Ciphertext = (U, V)`

### Decryption Algorithm

**Input:** Ciphertext (U, V), DK (G1)

**Process:**

1. Compute `g_id = e(DK, U) = e(s×Q_id, r×G2) = e(Q_id, G2)^(s×r)`
2. Derive symmetric key: `K = Keccak256(g_id)[0..32]`
3. Decrypt: `M = V ⊕ K`
4. Output: Plaintext M

**Security Property:**
`e(DK, U) = e(s×Q_id, r×G2) = e(Q_id, s×G2)^r = e(Q_id, MPK)^r`
This matches the encryption's `g_id`, enabling decryption.

### Canonical Serialization (Cross-Implementation Requirement)

> [!IMPORTANT]
> All implementations MUST use identical serialization for Gt elements to ensure cross-implementation compatibility.

**Gt Serialization (Canonical):**

```
Gt (Fp12) → 576 bytes (12 × 48 bytes, one per Fp coefficient)
Order: c0.c0.c0, c0.c0.c1, c0.c1.c0, c0.c1.c1, c0.c2.c0, c0.c2.c1,
       c1.c0.c0, c1.c0.c1, c1.c1.c0, c1.c1.c1, c1.c2.c0, c1.c2.c1
Each Fp: 48 bytes, big-endian
```

**Symmetric Key Derivation:**

```
K = Keccak256(Gt_bytes)[0..32]
where Gt_bytes = canonical 576-byte serialization
```

**Implementation Notes:**

- TypeScript (`@noble/curves`): Use `bls12_381.fields.Fp12.toBytes(gt)`
- Rust (`blstrs`): Extract Fp12 coefficients and serialize manually (blstrs does not expose native Gt serialization)

> [!CAUTION]
> Using non-canonical serialization (e.g., debug format strings) will cause decryption failures when encrypting with one implementation and decrypting with another.

---

## Timestamp Specification

### Timestamp Format

**All timelock deadlines use Unix epoch time in MICROSECONDS (u64).**

### Why Microseconds?

- **Aptos native**: `timestamp::now_microseconds()` returns u64 microseconds
- **Precision**: Avoids ambiguity between seconds/milliseconds/microseconds
- **Range**: u64 microseconds supports dates up to year 586,524 AD

### Examples

| Date/Time (UTC)     | Microseconds (u64) |
| ------------------- | ------------------ |
| 2024-01-01 00:00:00 | 1704067200000000   |
| 2024-01-01 01:00:00 | 1704070800000000   |
| 2024-12-31 23:59:59 | 1735689599000000   |

### Conversion

```rust
// Seconds → Microseconds
let seconds = 1704067200_u64;
let microseconds = seconds * 1_000_000;

// Microseconds → Seconds
let microseconds = 1704067200000000_u64;
let seconds = microseconds / 1_000_000;
```

### Checkpoint Alignment

Deadlines must be aligned to `checkpoint_period_microseconds`:

```rust
// Valid if deadline is multiple of checkpoint period
deadline_timestamp_microseconds % checkpoint_period_microseconds == 0

// Example: 1-hour checkpoints
checkpoint_period_microseconds = 3_600_000_000  // 1 hour in microseconds

// Valid deadlines:
// 1704067200000000 (2024-01-01 00:00:00) ✓ aligned
// 1704070800000000 (2024-01-01 01:00:00) ✓ aligned

// Invalid deadline:
// 1704071234567890 (arbitrary timestamp)  ✗ not aligned
```

### Field Naming Convention

> [!IMPORTANT]
> All deadline fields MUST use the suffix `_microseconds` to avoid ambiguity.

```
✓ deadline_timestamp_microseconds: u64
✗ deadline: u64                         // Ambiguous!
✗ deadline_timestamp: u64               // Ambiguous units!
```

---

## Architecture

### Component Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                             │
│  - TypeScript SDK (encryption/decryption)                        │
│  - dApp interfaces                                               │
└────────────────────────┬────────────────────────────────────────┘
                         │ View Functions & Events
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ON-CHAIN (Move)                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ timelock     │  │ timelock_    │  │ block        │          │
│  │ .move        │◄─│ config.move  │◄─│ .move        │          │
│  │              │  │              │  │              │          │
│  │ TimelockState│  │ Interval cfg │  │ on_new_block │          │
│  │ Events       │  │              │  │ (rotation)   │          │
│  │ Aggregation  │  │              │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└────────────────────────┬────────────────────────────────────────┘
                         │ Events
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   VALIDATOR NODE (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ epoch_       │─►│ dkg_         │─►│ vtxn/        │          │
│  │ manager.rs   │  │ manager.rs   │  │ timelock.rs  │          │
│  │              │  │              │  │              │          │
│  │ Event listen │  │ DKG execute  │  │ VM dispatch  │          │
│  │ Share store  │  │ Aggregation  │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                         │                                        │
│  ┌──────────────────────▼────────────────────────┐              │
│  │ PersistentSafetyStorage                       │              │
│  │ (Share persistence)                           │              │
│  └───────────────────────────────────────────────┘              │
└────────────────────────┬────────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   CRYPTOGRAPHY (Rust)                            │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ aptos-dkg/src/ibe/                                        │  │
│  │                                                           │  │
│  │  - ibe_encrypt(mpk, identity, message) → Ciphertext     │  │
│  │  - ibe_decrypt(dk, ciphertext) → Plaintext              │  │
│  │  - derive_decryption_key(msk, identity) → DK            │  │
│  │  - serialize_g1/g2, deserialize_g1/g2                   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### State Machine

> **Terminology Note**: The term "interval" is deprecated. The current term is **deadline**. The diagram below uses the old terminology for historical reference.
>
> **Key Concept**: DKG runs per **Epoch**, not per deadline. The MPK and DK_shares generated at epoch boundary are used for all deadlines within that epoch.

```
┌──────────────┐
│  DEADLINE N  │
│  (Within     │
│   Epoch E)   │
└──────┬───────┘
       │ Time passes (on_new_block)
       │ No new DKG until next epoch
       ▼
┌──────────────────────────────────────────────────────────┐
│  RequestRevealEvent(N)                                    │
└──────┬─────────────────────────────┬─────────────────────┘
       │                             │
       │ PARALLEL                    │
       ▼                             ▼
┌──────────────────┐          ┌──────────────────┐
│  (No DKG)        │          │  Reveal for N    │
│  (Same keys from │          │  (DK_shares)     │
│   Epoch E DKG)   │          │                  │
└──────┬───────────┘          └──────┬───────────┘
       │                             │
       ▼                             ▼
┌──────────────────┐          ┌──────────────────┐
│  KeyPublished    │          │  SecretRevealed  │
│  Event(E)        │          │  Event(N)        │
└──────────────────┘          └──────────────────┘
       │                             │
       │                             │  (DK for Deadline N
       │                             │   now revealed)
       └─────────────┬───────────────┘
                     │ Epoch Boundary
                     ▼
              ┌──────────────┐
              │  EPOCH E+1   │
              │  New DKG     │
              └──────────────┘
```

**Flow Summary:**

- **DKG** → Runs once per epoch boundary → produces MPK + DK_shares
- **Deadlines** → Multiple deadlines within an epoch → all use same DKG output
- **Reveal** → Each deadline triggers share submission → DK revealed after threshold

---

## Validator Specification

### Prerequisites

**Configuration Requirements:**

```yaml
# validator.yaml
consensus:
  validator_txn_enabled: true # CRITICAL: Must be enabled

randomness_config:
  # Optional: If randomness is enabled, it shares DKG infrastructure
  enabled: true
```

**Storage Requirements:**

- PersistentSafetyStorage configured (consensus keys)
- Minimum 10 GB disk for share storage
- Reliable network connectivity to validator set

### Validator Lifecycle

#### Phase 1: Initialization (Genesis)

**On node start:**

```rust
// EpochManager initialization (automatic)
let epoch_manager = EpochManager::new(
    safety_rules_config,
    my_addr,
    reconfig_events,
    dkg_start_events,
    // ... network components
);
```

**Verification:**

- Check `consensus_config.is_vtxn_enabled() == true`
- Verify consensus key is loaded in PersistentSafetyStorage

#### Phase 2: DKG Execution (Per Epoch)

> **Important**: DKG runs per **Epoch**, not per deadline. The MPK and DK_shares generated in one epoch are used for all timelock deadlines within that epoch. A new DKG is only triggered when the epoch changes.

**Trigger:** `StartKeyGenEvent` emitted on-chain at epoch boundary

**Process:**

1. **Event Detection** (`epoch_manager.rs:163`)

   ```rust
   if let Ok(timelock_start) = StartKeyGenEvent::try_from(&event) {
       self.start_timelock_dkg(timelock_start);
   }
   ```

2. **Validator Eligibility Check**

   ```rust
   let my_index = epoch_state
       .verifier
       .address_to_validator_index()
       .get(&self.my_addr)?;
   ```

   Skip if not in active validator set.

3. **DKG Session Spawn**

   ```rust
   let dkg_manager = DKGManager::<DefaultDKG>::new(
       dealer_sk,      // Consensus secret key
       my_index,
       my_addr,
       epoch_state,
       agg_trx_producer,
       vtxn_pool,
       true,           // is_timelock flag
   );
   tokio::spawn(dkg_manager.run(...));
   ```

4. **Secret Sharing**
   - Generate random polynomial coefficients
   - Compute transcript (encrypted shares for all validators)
   - Broadcast via P2P network (ReliableBroadcast)

5. **Aggregation**
   - Collect transcripts from >= threshold validators
   - Verify each transcript (PVSS verification)
   - Compute aggregated transcript

6. **Submission**
   ```rust
   let vtxn = ValidatorTransaction::TimelockDKGResult(DKGTranscript {
       metadata: DKGTranscriptMetadata {
           epoch: current_epoch,
           author: my_addr,
       },
       transcript_bytes: bcs::to_bytes(&aggregated_transcript)?,
   });
   vtxn_pool.put(Topic::TIMELOCK, Arc::new(vtxn), None);
   ```

**Expected Duration:** 5-15 seconds (network dependent)

#### Phase 3: Share Extraction

**Trigger:** `KeyPublishedEvent` emitted after transcript is accepted

**Process:**

1. **Event Detection** (`epoch_manager.rs:166`)

   ```rust
   if let Ok(timelock_key) = KeyPublishedEvent::try_from(&event) {
       self.process_timelock_key_published(timelock_key);
   }
   ```

2. **Transcript Deserialization**

   ```rust
   let transcript: Transcript = bcs::from_bytes(&event.public_key)?;
   ```

3. **Share Decryption**

   ```rust
   let (share, _pk_share) = DefaultDKG::decrypt_secret_share_from_transcript(
       &pub_params,
       &transcript,
       my_index,
       &decryption_key,  // From consensus SK
   )?;
   ```

4. **Persistent Storage**
   ```rust
   let share_bytes = bcs::to_bytes(&share)?;
   key_storage.set_timelock_share(deadline, share_bytes)?;
   ```

**Storage Format:** BCS-serialized `DealtSecretKeyShares` (varies, ~100-500 bytes)

#### Phase 4: Share Reveal

**Trigger:** `RequestRevealEvent` emitted (deadline passed)

**Process:**

1.  **Share Retrieval**

    ```rust
    let share_bytes = key_storage.get_timelock_share(deadline)?;
    let shares: DealtSecretKeyShares = bcs::from_bytes(&share_bytes)?;
    ```

2.  **G1 Point Extraction**

    ```rust
    let dk_g1 = shares.main[0].as_group_element().clone();
    ```

3.  **Serialization**

    ```rust
    let dk_bytes = aptos_dkg::ibe::serialize_g1(&dk_g1)?;  // 48 bytes
    ```

4.  **Submission**
    ```rust
    let share = TimelockShare {
        deadline,
        author: my_addr,
        share: dk_bytes,  // 48 bytes, G1 compressed
    };
    let vtxn = ValidatorTransaction::TimelockShare(share);
    vtxn_pool.put(Topic::TIMELOCK, Arc::new(vtxn), None);
    ```

**Expected Duration:** < 1 second

### Validator Transaction Types

**1. TimelockDKGResult**

```rust
struct DKGTranscript {
    metadata: DKGTranscriptMetadata {
        epoch: u64,
        author: AccountAddress,
    },
    transcript_bytes: Vec<u8>,  // BCS-serialized PVSS transcript
}
```

- **Submitted:** After DKG aggregation (per epoch)
- **Frequency:** Once per epoch (first validator to submit)
- **Topic:** `Topic::TIMELOCK`

**2. TimelockShare**

```rust
struct TimelockShare {
    deadline: u64,
    author: AccountAddress,
    share: Vec<u8>,  // 48 bytes, G1 compressed
}
```

- **Submitted:** After reveal request (per deadline)
- **Frequency:** Once per deadline per validator
- **Topic:** `Topic::TIMELOCK`

### Monitoring & Logging

**Key Log Messages:**

```
INFO [Timelock] Starting DKG for epoch {epoch}
INFO [Timelock] Spawned and triggered DKG manager
INFO [Timelock] Processing KeyPublishedEvent
INFO [Timelock] Share stored successfully
INFO [Timelock] Revealing share for deadline {deadline}
INFO [Timelock] Successfully computed and submitted decryption key share
```

**Error Scenarios:**

- `ERROR [Timelock] Cannot start DKG - no epoch state available`
  → Wait for reconfig notification
- `ERROR [Timelock] Failed to load consensus secret key`
  → Check PersistentSafetyStorage configuration
- `ERROR [Timelock] No secret share found for deadline`
  → Validator joined after this deadline's DKG

---

## Explorer Integration

> **Note**: The Move API uses `interval` field names (e.g., `get_current_interval`, `interval: u64` in structs). This is legacy terminology from an earlier design. In documentation and UI, use **deadline** instead. The mapping is:
>
> - `get_current_interval` → Get current deadline number
> - `get_public_key(interval)` → Get MPK for deadline
> - `get_secret(interval)` → Get DK for deadline

### View Functions (REST API)

**Get Current Deadline:**

```typescript
GET /v1/view
{
  "function": "0x1::timelock::get_current_interval",
  "type_arguments": [],
  "arguments": []
}
// Returns: "42" (string representation of u64 - current deadline number)
```

**Get Public Key for Deadline:**

```typescript
GET /v1/view
{
  "function": "0x1::timelock::get_public_key",
  "type_arguments": [],
  "arguments": ["42"]
}
// Returns: Option<vector<u8>>
// Some: "0x96abc..." (hex-encoded G2 point, 96 bytes)
// None: Key not yet published
```

**Get Secret for Deadline:**

```typescript
GET /v1/view
{
  "function": "0x1::timelock::get_secret",
  "type_arguments": [],
  "arguments": ["42"]
}
// Returns: Option<vector<u8>>
// Some: "0x48def..." (hex-encoded G1 point, 48 bytes)
// None: Secret not yet revealed
```

**Check Secret Status:**

```typescript
GET /v1/view
{
  "function": "0x1::timelock::is_secret_revealed",
  "type_arguments": [],
  "arguments": ["42"]
}
// Returns: true | false
```

**Get Deadline Configuration:**

```typescript
GET /v1/view
{
  "function": "0x1::timelock::get_interval_config",
  "type_arguments": [],
  "arguments": ["42"]
}
// Returns: Option<DeadlineConfig> (legacy name: IntervalConfig)
// {
//   "threshold": "3",
//   "total_validators": "4",
//   "created_at": "1704153600000000"
// }
```

### Events to Index

> **Note**: Move event structs use `interval: u64` field. This maps to the deadline number.

**1. StartKeyGenEvent** (at epoch boundary)

```move
struct StartKeyGenEvent {
    interval: u64,  // Epoch number (triggers new DKG)
    config: TimelockConfig,
}
```

**Event Key:** `0x1::timelock::TimelockState.start_keygen_events`

**2. KeyPublishedEvent**

```move
struct KeyPublishedEvent {
    interval: u64,  // Epoch number
    public_key: vector<u8>,  // BCS-serialized PVSS Transcript
}
```

**Event Key:** `0x1::timelock::TimelockState.key_published_events`

> [!NOTE]
> The `public_key` field contains the **BCS-serialized PVSS transcript**, not the raw G2 MPK.
> To extract the MPK: `let transcript = bcs::from_bytes(&event.public_key); let mpk = transcript.main.get_dealt_public_key();`

**3. RequestRevealEvent**

```move
struct RequestRevealEvent {
    interval: u64,  // Deadline number (when deadline passes)
}
```

**Event Key:** `0x1::timelock::TimelockState.request_reveal_events`

**4. SecretRevealedEvent**

```move
struct SecretRevealedEvent {
    interval: u64,  // Deadline number
    secret: vector<u8>,  // 48 bytes, G1 DK
}
```

**Event Key:** `0x1::timelock::TimelockState.secret_revealed_events`

### Explorer UI Recommendations

> **Terminology Note**: The term "interval" is deprecated in documentation. The current term is **deadline**. However, the Move API uses `interval` field names (e.g., `interval: u64` in structs). For UI display purposes, use "Deadline" to match current terminology.

**Timelock Dashboard:**

- **Current Deadline:** Display as "Deadline #42"
- **Next Rotation:** Estimate based on `last_rotation_time + interval_microseconds`
- **Recent Deadlines:** Table showing:
  - Deadline number
  - Public Key status (Published / Pending)
  - Secret status (Revealed / Pending)
  - Timestamp

**Deadline Detail Page:**

```
Deadline #42
Status: Active Encryption

Master Public Key (MPK)
  Published: Yes (2026-01-01 14:00:00 UTC)
  Hex: 0x96abc...def (96 bytes)
  [Copy] [Download]

Decryption Key (DK)
  Status: Pending Reveal
  Expected: 2026-01-01 15:00:00 UTC
  Shares Collected: 2/3

Configuration
  Threshold: 3 validators
  Total: 4 validators
  Created: 2026-01-01 14:00:00 UTC

Timeline
  ✓ Deadline started
  ✓ DKG completed
  ✓ MPK published
  ○ Rotation triggered (pending)
  ○ Shares revealed (pending)
  ○ Secret aggregated (pending)
```

---

## dApp Developer Guide

### Installation

**TypeScript/JavaScript:**

```bash
npm install @noble/curves @noble/hashes
```

### Quick Start

**1. Encrypt a Message**

```typescript
import { AptosClient } from "aptos";
import { IBECrypto } from "./ibe-crypto";

const client = new AptosClient("https://fullnode.mainnet.aptoslabs.com");

// Register a timelock (returns timelock_id and deadline)
// In practice, call 0x1::timelock::register_timelock
const timelockId = 42n; // From registration
const deadlineTimestampMicroseconds = 1704070800000000n; // 2024-01-01 01:00:00 UTC

// Wait for MPK to be published (poll or event listener)
let mpkBytes: Uint8Array;
while (true) {
  const result = await client.view({
    function: "0x1::timelock::get_public_key_for_deadline",
    type_arguments: [],
    arguments: [deadlineTimestampMicroseconds.toString()],
  });
  if (result) {
    mpkBytes = new Uint8Array(result as number[]);
    break;
  }
  await new Promise((resolve) => setTimeout(resolve, 5000)); // Poll every 5s
}

// Compute identity (application-agnostic)
const identity = IBECrypto.computeTimelockIdentity(timelockId, deadlineTimestampMicroseconds);

// Encrypt message
const message = new TextEncoder().encode("My secret data");
const ciphertext = IBECrypto.ibeEncrypt(mpkBytes, identity, message);

// Store ciphertext off-chain or on-chain
// Format: { u: Uint8Array(96), v: Uint8Array(variable) }
const serialized = {
  u: Array.from(ciphertext.u),
  v: Array.from(ciphertext.v),
};
```

**2. Decrypt a Message**

```typescript
// Wait for secret to be revealed (after deadline passes)
let dkBytes: Uint8Array;
while (true) {
  const result = await client.view({
    function: "0x1::timelock::get_secret_for_deadline",
    type_arguments: [],
    arguments: [deadlineTimestampMicroseconds.toString()],
  });
  if (result) {
    dkBytes = new Uint8Array(result as number[]);
    break;
  }
  await new Promise((resolve) => setTimeout(resolve, 5000));
}

// Decrypt
const plaintext = IBECrypto.ibeDecrypt(dkBytes, identity, mpkBytes, ciphertext);
const message = new TextDecoder().decode(plaintext);
console.log(message); // "My secret data"
```

### TypeScript IBE Implementation

**Complete Library (`ibe-crypto.ts`):**

```typescript
import { bls12_381 } from "@noble/curves/bls12-381";
import { keccak_256 } from "@noble/hashes/sha3";

export interface Ciphertext {
  u: Uint8Array; // 96 bytes - G2 point
  v: Uint8Array; // encrypted message
}

export class IBECrypto {
  /**
   * Compute the IBE identity for a timelock.
   *
   * @param timelockId - Unique identifier for this timelock
   * @param deadlineTimestampMicroseconds - Unix epoch timestamp in MICROSECONDS when decryption becomes available
   * @returns 32-byte Keccak256 hash
   */
  static computeTimelockIdentity(timelockId: bigint, deadlineTimestampMicroseconds: bigint): Uint8Array {
    // Canonical identity format (application-agnostic)
    // Format: "timelock_id:{id}:deadline_timestamp_microseconds:{deadline}"
    const identityString = `timelock_id:${timelockId}:deadline_timestamp_microseconds:${deadlineTimestampMicroseconds}`;
    return keccak_256(new TextEncoder().encode(identityString));
  }

  static ibeEncrypt(mpkG2: Uint8Array, identity: Uint8Array, message: Uint8Array): Ciphertext {
    // Hash identity to G1
    const pointId = bls12_381.G1.hashToCurve(identity);

    // Parse MPK (G2)
    const mpkPoint = bls12_381.G2.Point.fromHex(Buffer.from(mpkG2).toString("hex"));

    // Random scalar r
    const r = bls12_381.utils.randomSecretKey();
    const rScalar = BigInt("0x" + Buffer.from(r).toString("hex"));

    // U = r * G2_generator
    const uPoint = bls12_381.G2.Point.BASE.multiply(rScalar);
    const u = uPoint.toBytes(true);

    // Shared secret = e(H_id * r, mpk)
    const pointIdTimesR = pointId.multiply(rScalar);
    const sharedSecret = bls12_381.pairing(pointIdTimesR, mpkPoint);

    // Symmetric key
    const sharedSecretBytes = bls12_381.fields.Fp12.toBytes(sharedSecret);
    const symmetricKey = keccak_256(sharedSecretBytes).slice(0, 32);

    // V = message XOR key
    const v = this.xorBytes(message, symmetricKey);

    return { u, v };
  }

  static ibeDecrypt(skG1: Uint8Array, identity: Uint8Array, mpkG2: Uint8Array, ciphertext: Ciphertext): Uint8Array {
    // Parse U (G2) and SK (G1)
    const uPoint = bls12_381.G2.Point.fromHex(Buffer.from(ciphertext.u).toString("hex"));
    const skPoint = bls12_381.G1.Point.fromHex(Buffer.from(skG1).toString("hex"));

    // Shared secret = e(sk, U)
    const sharedSecret = bls12_381.pairing(skPoint, uPoint);

    // Symmetric key
    const sharedSecretBytes = bls12_381.fields.Fp12.toBytes(sharedSecret);
    const symmetricKey = keccak_256(sharedSecretBytes).slice(0, 32);

    // Decrypt
    return this.xorBytes(ciphertext.v, symmetricKey);
  }

  private static xorBytes(a: Uint8Array, b: Uint8Array): Uint8Array {
    const result = new Uint8Array(a.length);
    for (let i = 0; i < a.length; i++) {
      result[i] = a[i] ^ b[i % b.length];
    }
    return result;
  }
}
```

### Move Smart Contract Integration

> **Note**: Move struct fields and function parameters use legacy `interval` terminology. This maps to the deadline number in current documentation.

**Storing Encrypted Data On-Chain:**

```move
module my_addr::sealed_auction {
    use std::vector;
    use aptos_framework::timelock;

    struct Bid has store {
        bidder: address,
        encrypted_amount: vector<u8>,  // Ciphertext.v
        ciphertext_u: vector<u8>,      // Ciphertext.u (96 bytes)
        target_deadline: u64,  // Note: Move uses target_interval field
    }

    public entry fun submit_bid(
        account: &signer,
        encrypted_amount: vector<u8>,
        ciphertext_u: vector<u8>,
        target_deadline: u64,  // Maps to timelock_id
    ) {
        // Verify target_deadline has a published key
        assert!(
            option::is_some(&timelock::get_public_key(target_deadline)),
            ERROR_KEY_NOT_PUBLISHED
        );

        // Store bid (implementation details omitted)
        // ...
    }

    public fun reveal_bids(deadline: u64): vector<u64> {
        // Verify secret is revealed
        assert!(
            timelock::is_secret_revealed(deadline),
            ERROR_SECRET_NOT_REVEALED
        );

        // Decryption happens off-chain
        // This function just marks the deadline as ready for reveal
        // ...
    }
}
```

### Best Practices

**1. Timing Strategy**

> **Note**: The Move API uses `current_interval` field. The term "deadline" is used in documentation for conceptual clarity.

- Encrypt for `current_interval + 1` or later
- Allow buffer time (2-3 deadlines) for DKG reliability
- Monitor events for key publication confirmation

**2. Error Handling**

```typescript
try {
  const ciphertext = IBECrypto.ibeEncrypt(mpk, identity, message);
} catch (error) {
  if (error.message.includes("Invalid G2")) {
    // MPK is malformed, wait for correct publication
  }
  throw error;
}
```

**3. Ciphertext Storage**

- U component: Always 96 bytes (store as `vector<u8>` in Move)
- V component: Same length as plaintext
- Total overhead: 96 bytes

**4. Gas Optimization**

- Store ciphertext off-chain when possible
- Use on-chain storage only for auction commitments
- Consider IPFS for large encrypted data

---

## End User Experience

### User Journey: Sealed Bid Auction

**Step 1: Check Auction Timeline**

```
Auction closes at: Block 1,000,000
Current block: 999,900
Time remaining: ~100 seconds (estimated)
```

**Step 2: Prepare Bid**

```
Enter bid amount: 100 APT
Encryption target: Deadline 42 (auction close time)
Status: Waiting for encryption key...
```

**Step 3: Encrypt & Submit**

```
✓ Encryption key published
✓ Bid encrypted locally (browser)
✓ Submitting to blockchain...
✓ Bid submitted! Transaction: 0xabc...def

Your bid is now sealed and cannot be viewed until the auction closes.
```

**Step 4: Wait for Auction Close**

```
Auction Status: Closed
Decryption key: Pending (2/3 validator shares collected)
```

**Step 5: Reveal**

```
✓ Decryption key revealed!
✓ Decrypting bids...

Results:
- Alice: 100 APT (Your bid)
- Bob: 150 APT (Winner)
- Charlie: 80 APT
```

### Trust Model

**What users must trust:**

- 2/3+ validators are honest (standard Aptos assumption)
- Aptos blockchain liveness
- Cryptographic primitives (BLS12-381, IBE)

**What users don't need to trust:**

- No single validator can decrypt early
- No trusted third party
- No centralized key server
- Code is open source and auditable

### Privacy Considerations

**What is hidden:**

- Message content (until reveal)
- Message size (obfuscated by padding if implemented)

**What is visible on-chain:**

- Fact that encryption occurred
- Sender address
- Target deadline
- Ciphertext size (reveals plaintext length)

**Mitigation: Padding**

```typescript
function padMessage(message: Uint8Array, targetSize: number): Uint8Array {
  const padded = new Uint8Array(targetSize);
  padded.set(message);
  // Fill rest with random bytes or zeros
  return padded;
}
```

---

## Infrastructure Requirements

### Blockchain Network Requirements

**Consensus Configuration:**

```move
// In genesis or via governance
OnChainConsensusConfig::V5 {
    validator_txn_enabled: true,
    // ... other settings
}
```

**Timelock Configuration:**

```move
// aptos_framework::timelock_config
TimelockConfig {
    interval_microseconds: 3600 * 1000000,  // 1 hour (production)
}
```

**Storage Tables:**

```move
struct TimelockState has key {
    current_interval: u64,
    last_rotation_time: u64,
    public_keys: Table<u64, vector<u8>>,          // interval -> MPK
    validator_shares: Table<u64, vector<ValidatorShare>>,
    revealed_secrets: Table<u64, vector<u8>>,     // interval -> DK
    interval_configs: Table<u64, IntervalConfig>,
    // Event handles...
}
```

### Validator Requirements

**Hardware:**

- CPU: 4+ cores
- RAM: 16 GB minimum
- Disk: 500 GB SSD (NVMe preferred)
- Network: 1 Gbps, low latency to other validators

**Software:**

- Aptos validator binary (with timelock support)
- PersistentSafetyStorage configuration
- Consensus key loaded

**Network:**

- P2P connectivity to validator set
- Firewall: Allow inbound on consensus port
- Recommended: Direct peering with other validators

### Client Requirements

**For Encryption (Off-Chain):**

- JavaScript runtime (browser or Node.js)
- Libraries: `@noble/curves`, `@noble/hashes`
- No blockchain interaction needed for encryption

**For Key Retrieval:**

- Access to Aptos fullnode (REST API)
- Optional: Indexer for event monitoring

**For Decryption (Off-Chain):**

- Same as encryption
- Decryption key from blockchain (48 bytes)

### Testing Infrastructure

**Local Testnet:**

```bash
# Use docker-test-harness
cd atomica/timelock-tests
bun run test:basic  # Basic DKG flow
bun run test:ibe    # Full encryption/decryption
```

**Devnet:**

- Shorter deadlines (5 seconds for fast testing)
- Use `set_interval_for_testing()` entry function
- Monitor via explorer

**Testnet/Mainnet:**

- Standard deadlines (1 hour)
- Full validator participation
- Production-like conditions

---

## Aptos-Provided Infrastructure

The Atomica Timelock system builds upon existing Aptos infrastructure. This section clarifies what was available **before** the timelock implementation.

### Pre-Existing Components

#### 1. Validator Transaction Framework

**Provided by:** Aptos Core (`aptos-types/src/validator_txn/`)

**Purpose:** System-level transactions that bypass mempool

**Features:**

- `ValidatorTransaction` enum (extensible)
- `Topic` routing (e.g., `Topic::DKG_RANDOMNESS`)
- `VTxnPoolState` for validator transaction pool
- VM execution dispatcher (`aptos-vm/src/validator_txns/`)

**What we added:**

- `Topic::TIMELOCK` variant
- `TimelockDKGResult` transaction type
- `TimelockShare` transaction type
- Dispatcher: `process_timelock_dkg_result()`, `process_timelock_share()`

#### 2. DKG Infrastructure (Randomness)

**Provided by:** Aptos DKG (`dkg/`, `crates/aptos-dkg/`)

**Purpose:** Distributed key generation for on-chain randomness

**Features:**

- `DKGManager`: Orchestrates DKG execution
- `EpochManager`: Event listener and coordinator
- Publicly Verifiable Secret Sharing (PVSS)
- ReliableBroadcast for transcript aggregation
- Threshold cryptography (BLS12-381)

**What we reused:**

- `DKGManager<DefaultDKG>` (with `is_timelock` flag)
- `EpochManager` event handling pattern
- PVSS transcript structure
- Network layer (P2P DKG messages)

**What we added:**

- Timelock-specific event handlers:
  - `start_timelock_dkg()`
  - `process_timelock_key_published()`
  - `process_timelock_reveal()`
- Per-deadline session management (HashMap)
- Share storage/retrieval for timelock

#### 3. Persistent Storage

**Provided by:** Safety Rules (`aptos-safety-rules/`)

**Purpose:** Persist validator consensus keys securely

**Features:**

- `PersistentSafetyStorage` trait
- Backends: File, Vault, InMemory
- Key-value storage for validator state

**What we added:**

- `set_timelock_share(deadline, bytes)`
- `get_timelock_share(deadline) -> bytes`
- Per-deadline namespacing

#### 4. Move Framework Foundation

**Provided by:** Aptos Framework (`aptos-move/framework/aptos-framework/`)

**Modules we used:**

- `aptos_framework::block` (rotation trigger)
- `aptos_framework::timestamp` (time-based logic)
- `aptos_framework::stake` (validator set queries)
- `aptos_framework::validator_consensus_info`
- `aptos_std::crypto_algebra` (G1 operations)
- `aptos_std::bls12381_algebra` (curve-specific)
- `aptos_std::table` (storage primitives)
- `aptos_framework::event` (event emission)

**What we added:**

- `aptos_framework::timelock` (new module)
- `aptos_framework::timelock_config` (new module)
- Integration: `block::on_new_block()` calls `timelock::on_new_block()`

#### 5. Cryptography Primitives

**Provided by:** `aptos-crypto`, `blstrs` crate

**Features:**

- BLS12-381 curve (G1, G2, Gt)
- Pairing operations
- Hash-to-curve
- Serialization (compressed/uncompressed)

**What we added:**

- IBE implementation (`aptos-dkg/src/ibe/`)
  - `ibe_encrypt()`
  - `ibe_decrypt()`
  - `derive_decryption_key()`
- Timelock identity derivation
- Serialization helpers (`serialize_g1`, `deserialize_g1`)

#### 6. Event System

**Provided by:** Aptos Event Notifications

**Features:**

- `EventNotificationListener` (subscribe to on-chain events)
- `ReconfigNotificationListener` (epoch changes)
- Event filtering by struct type

**What we added:**

- Event struct definitions:
  - `StartKeyGenEvent`
  - `KeyPublishedEvent`
  - `RequestRevealEvent`
  - `SecretRevealedEvent`
- Event subscriptions in `EpochManager`

### Infrastructure Integration Points

```
┌────────────────────────────────────────────────────────┐
│         APTOS-PROVIDED (Before Timelock)                │
├────────────────────────────────────────────────────────┤
│  ValidatorTransaction Framework                         │
│  DKG Infrastructure (Randomness V2)                     │
│  PersistentSafetyStorage                                │
│  Move Framework (block, stake, events)                  │
│  BLS12-381 Cryptography                                 │
│  Event Notification System                              │
└─────────────────────┬──────────────────────────────────┘
                      │ Extended by Atomica
                      ▼
┌────────────────────────────────────────────────────────┐
│              ATOMICA ADDITIONS                          │
├────────────────────────────────────────────────────────┤
│  Move Modules:                                          │
│    - timelock.move (state, logic, events)              │
│    - timelock_config.move (deadline config)            │
│                                                         │
│  Rust Extensions:                                       │
│    - Timelock DKG handlers in EpochManager             │
│    - Share storage in PersistentSafetyStorage          │
│    - IBE crypto library (aptos-dkg/src/ibe)            │
│    - VM dispatchers (process_timelock_*)               │
│                                                         │
│  Transaction Types:                                     │
│    - Topic::TIMELOCK                                   │
│    - TimelockDKGResult                                 │
│    - TimelockShare                                     │
│                                                         │
│  Client SDK:                                            │
│    - ibe-crypto.ts (TypeScript IBE)                    │
│    - Test harness (Docker testnet)                     │
└────────────────────────────────────────────────────────┘
```

---

## Security Considerations

### Threat Model

**Assumptions:**

1. **Honest Majority:** At least 2/3 + 1 validators are honest
2. **Network Liveness:** Validators can communicate
3. **Cryptographic Security:** BLS12-381 and IBE are secure

**Attacks Prevented:**

- **Early Decryption:** Single validator cannot decrypt (threshold security)
- **Key Leakage:** Master secret never reconstructed on single machine
- **Replay Attacks:** Shares are deadline-specific
- **Censorship:** Any validator can trigger rotation via entry function
- **Early Reveal:** Share publication MUST validate `deadline < current_deadline` (only past deadlines can be revealed)

**Attacks Not Prevented:**

- **Collusion:** 2/3+ validators can collude to reveal early
- **Network Partition:** DKG may fail if too many validators offline
- **Quantum Computers:** Pairing-based crypto vulnerable (future concern)

### Security Properties

**Confidentiality:**

- Ciphertext reveals no information about plaintext (IND-ID-CPA)
- Decryption impossible before reveal (assuming honest threshold)

**Integrity:**

- Invalid shares rejected by G1 deserialization check
- Transcript verification via PVSS

**Availability:**

- Manual rotation fallback (`trigger_rotation()`)
- DKG retry on failure (implementation pending)

### Failure Modes

> [!IMPORTANT]
> Applications MUST handle these failure scenarios gracefully.

**1. DKG Timeout (No Transcript Published)**

- **Cause:** Network partition, insufficient validator participation
- **Effect:** `get_public_key(deadline)` returns `None`
- **Recovery:** Deadline is skipped; next deadline proceeds normally
- **User Impact:** Messages cannot be encrypted for this deadline; encrypt for future deadlines instead

**2. Reveal Threshold Not Met**

- **Cause:** Fewer than 2/3+1 validators reveal shares
- **Effect:** `get_secret(deadline)` returns `None` indefinitely
- **Recovery:** None — secret is permanently unrecoverable
- **User Impact:** Messages encrypted for this deadline cannot be decrypted
- **Mitigation:** Encrypt for deadlines with buffer time; check validator liveness before encrypting

**3. New Validator After DKG**

- **Cause:** New validator added to set after deadline's DKG completed
- **Effect:** New validator has no share for that deadline
- **Recovery:** Validator logs warning and does not participate in reveal
- **User Impact:** None (threshold is based on original participant set)

**4. Validator Key Rotation**

- **Cause:** Validator rotates consensus key between DKG and reveal
- **Effect:** Cannot decrypt own share from transcript
- **Recovery:** Validator logs error and does not participate in reveal
- **User Impact:** Reduces available shares; may prevent threshold if many validators rotate

### Audit Recommendations

**Critical Components:**

1. IBE implementation (`aptos-dkg/src/ibe/mod.rs`)
2. Share aggregation logic (`timelock.move:235-316`)
3. Identity derivation (both Rust and TypeScript)
4. Transcript deserialization/verification
5. Share storage security (PersistentSafetyStorage)

**Test Vectors:**

- Encrypt/decrypt round-trip
- Invalid share rejection
- Threshold boundary conditions
- Cross-implementation compatibility (Rust ↔ TypeScript)

---

## Appendix

### Glossary

- **DKG**: Distributed Key Generation
- **IBE**: Identity-Based Encryption
- **MPK**: Master Public Key (G2 point, 96 bytes)
- **DK**: Decryption Key (G1 point, 48 bytes)
- **PVSS**: Publicly Verifiable Secret Sharing
- **VTxn**: Validator Transaction
- **Gt**: Target group of pairing (Fp12 element)

### References

- [BLS12-381 Specification](https://electriccoin.co/blog/new-snark-curve/)
- [Boneh-Franklin IBE Paper](https://crypto.stanford.edu/~dabo/papers/bfibe.pdf)
- [Aptos DKG Documentation](https://github.com/aptos-labs/aptos-core/tree/main/dkg)
- [Aptos Randomness AIP](https://github.com/aptos-foundation/AIPs)

### Version History

- **v1.0** (2026-01-02): Initial specification
