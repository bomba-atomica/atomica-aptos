# IBE Move Testing Gap & Security Analysis (REVISED)

**Date:** 2026-01-22
**Author:** Security Audit
**Scope:** IBE DK reconstruction testing coverage and security verification

---

## Executive Summary

This analysis identifies three critical issues:

1. **Testing Gap:** Move native function has zero Move-level tests (complete Rust coverage exists)
2. **Security Gap:** Validators submit DK shares without cryptographic proof of correctness
3. **Implementation Bug:** Code mismatch between expected share types (32-byte scalars vs 48-byte G1)

**Most Critical:** The system accepts unverified DK shares on-chain. A malicious validator can submit invalid shares and prevent decryption.

---

## Part 1: Cryptographic Objects - Precise Definitions

### Layer 1: Master Key Pair (DKG Output)

| Name | Symbol | Type | Size | Created By | Visibility | On-Chain? |
|------|--------|------|------|------------|------------|-----------|
| **Master Secret Key** | MSK | Scalar (Fr) | 32 bytes | DKG (distributed) | SECRET - never revealed | ❌ NO |
| **Master Public Key** | MPK | G2 point | 96 bytes compressed | DKG | PUBLIC | ✅ YES |

**Relationship:**
```
MPK = MSK × g₂
```

**Lifecycle:**
- MSK is created via distributed key generation (DKG)
- MSK never exists as a single value on any machine
- MSK is distributed as PVSS shares to validators
- MPK is published on-chain after DKG completes

**Storage:** `IBEPublicParams.mpk` (96 bytes)

---

### Layer 2: PVSS Secret Shares (Validator Private State)

| Name | Symbol | Type | Size | Held By | Visibility | On-Chain? |
|------|--------|------|------|---------|------------|-----------|
| **PVSS Secret Share** | s_i | Vec\<Scalar\> | 32 bytes × weight | Validator i | SECRET - off-chain only | ❌ NO |
| **PVSS Public Key Share** | pk_i | Vec\<G2\> | 96 bytes × weight | Public (from transcript) | PUBLIC | ❌ **NOT STORED** |

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
4. **Problem:** `pk_i` values are NOT stored on-chain after DKG

**What validators know:**
- ✅ Their own secret shares: `s_i = [s_i[0], s_i[1], ..., s_i[weight-1]]`
- ✅ Their own public key shares: `pk_i` (from transcript)
- ✅ All other validators' public key shares: `pk_j` (from transcript)
- ✅ The master public key: `MPK` (on-chain)

**Security properties:**
- `s_i` values MUST remain secret
- Knowledge of threshold `t` shares allows MSK reconstruction
- `pk_i` provides verifiability without revealing `s_i`

---

### Layer 3: IBE Decryption Key Shares (On-Chain Submissions)

| Name | Symbol | Type | Size | Submitted By | Visibility | On-Chain? |
|------|--------|------|------|--------------|------------|-----------|
| **DK Share** | dk_share_i | G1 point | 48 bytes compressed | Validator i | PUBLIC | ✅ YES |

**Computation (off-chain by validator):**
```
H = hash_to_G1(identity)                 // Map identity to G1
dk_share_i = sum_j(s_i[j] × H)           // For all virtual players j
           = (sum_j s_i[j]) × H
```

**What's happening:**
1. Validator computes `H = hash_to_G1(SHA3-256(timelock_id || deadline_us))`
2. Validator multiplies each secret share by `H`
3. Validator sums the results to get one G1 point
4. Validator compresses the point to 48 bytes and submits on-chain

**Storage:** `TimelockInfo.submitted_shares` (vector of 48-byte G1 points)

**Intended verification (NOT IMPLEMENTED):**
```
e(dk_share_i, g₂) =? e(H, sum_j(pk_i[j]))
```
This would prove `dk_share_i = (sum_j s_i[j]) × H` without revealing `s_i`.

**Current reality:** ❌ NO VERIFICATION - validators can submit arbitrary G1 points!

---

### Layer 4: Reconstructed Decryption Key (Final Output)

| Name | Symbol | Type | Size | Created By | Visibility | On-Chain? |
|------|--------|------|------|------------|------------|-----------|
| **Decryption Key** | DK | G1 point | 48 bytes compressed | Native function | PUBLIC after threshold | ✅ YES |

**Reconstruction:**
```
DK = sum_i(Lagrange_i(validators) × dk_share_i)
   = sum_i(Lagrange_i × sum_j(s_i[j]) × H)
   = (sum_i sum_j (Lagrange_i × s_i[j])) × H
   = MSK × H
```

**Verification:**
```
e(DK, g₂) =? e(H, MPK)
```
This proves `DK = MSK × H` is correctly computed.

**Storage:** `TimelockInfo.decryption_key` (48 bytes)

---

## Part 2: What Move Land Sees vs. What It Needs

### Currently On-Chain

```move
struct IBEPublicParams has key {
    mpk: vector<u8>,        // ✅ 96-byte G2 MPK
    epoch: u64,
}

struct TimelockInfo has store {
    identity: vector<u8>,                  // ✅ 32-byte identity
    submitted_shares: vector<vector<u8>>,  // ✅ 48-byte G1 dk_shares
    validator_indices: vector<u64>,
    validator_weights: vector<u64>,
    decryption_key: vector<u8>,           // ✅ 48-byte G1 DK
    ...
}
```

### Missing from On-Chain (SECURITY GAP)

```move
// NOT STORED:
struct ValidatorPubKeyShares {
    validator_index: u64,
    pk_shares: vector<vector<u8>>,  // Vec<G2> public key shares
}
```

**Why this matters:**
- Without `pk_i`, we cannot verify that submitted `dk_share_i` is correctly computed
- A malicious validator can submit `dk_share_i = random_G1_point()`
- The reconstruction will produce garbage: `DK = sum(Lagrange_i × garbage_i) = garbage`
- Users cannot decrypt their timelocks

---

## Part 3: The Critical Security Gap

### Current Submission Flow (UNSAFE)

```move
// aptos-move/framework/aptos-framework/sources/ibe_config.move:217-248
public(friend) fun submit_dk_share(
    timelock_id: u64,
    share: vector<u8>,              // 48-byte G1 point
    validator_address: address,
    weight: u64,
    total_weight: u64
) {
    // ❌ ONLY checks length
    assert!(vector::length(&share) == G1_LENGTH, ...);

    // ❌ NO CRYPTOGRAPHIC VERIFICATION
    vector::push_back(&mut timelock_info.submitted_shares, share);

    // Reconstructs once threshold met
    if (share_count >= threshold) {
        reconstruct_and_store_dk(...)  // Uses unverified shares!
    }
}
```

### What's Missing: Proof of Correctness

Validators should submit:
```move
struct DKShareSubmission {
    dk_share: vector<u8>,              // 48-byte G1 point
    proof: vector<u8>,                 // Proof that dk_share is correctly computed
}
```

**Verification options:**

#### Option A: Store PK Shares (Complete Solution)
```move
// During DKG, store:
struct ValidatorKeyShares has store {
    pk_shares: vector<vector<u8>>,     // Vec<G2> from PVSS transcript
}

// On submission, verify:
verify_dk_share(
    dk_share,      // Submitted G1
    identity,      // Timelock identity
    pk_shares,     // Validator's G2 public key shares
)
```

Verification:
```
e(dk_share, g₂) == e(H(identity), aggregate(pk_shares))
```

#### Option B: Zero-Knowledge Proof (Complex)
Validator proves: "I know `s_i` such that `dk_share = s_i × H` and `pk_i = s_i × g₂`"

This is a Schnorr-like proof but in pairing setting. More complex, but doesn't require storing all `pk_i`.

#### Option C: Optimistic with Slashing (Pragmatic)
- Accept shares without proof
- After reconstruction, verify `e(DK, g₂) == e(H, MPK)`
- If verification fails, slash all participating validators
- Requires identifying which validator submitted bad share (non-trivial)

---

## Part 4: The Implementation Bug

### The Type Mismatch

**Move API says:**
```move
// aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move:23
/// * `scalar_shares` - Nested vector of 32-byte little-endian scalar shares
public fun reconstruct_ibe_dk<G1>(
    validator_indices: vector<u64>,
    scalar_shares: vector<vector<u8>>,  // Expects 32-byte scalars
    ...
)
```

**Native function requires:**
```rust
// aptos-move/framework/src/natives/cryptography/algebra/ibe.rs:136-149
for share_bytes in scalar_shares_bytes.iter() {
    if share_bytes.len() != 32 {
        return Err(...("Scalar share must be 32 bytes"));
    }
    let scalar = Scalar::from_bytes_le(&bytes).unwrap();
    ...
}
```

**But submission stores:**
```move
// aptos-move/framework/aptos-framework/sources/ibe_config.move:224
assert!(vector::length(&share) == G1_LENGTH, ...);  // G1_LENGTH = 48
vector::push_back(&mut timelock_info.submitted_shares, share);
```

**And reconstruction passes:**
```move
// aptos-move/framework/aptos-framework/sources/ibe_config.move:257-259
let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
    timelock_info.validator_indices,
    timelock_info.submitted_shares,  // 48-byte G1 shares!
    ...
)
```

### The Bug

**Stored:** 48-byte G1 points
**Expected:** 32-byte scalars
**Result:** Native function will abort with "Scalar share must be 32 bytes"

### Possible Explanations

1. **Work in Progress:** Code is mid-refactor (git log shows recent WIP commits)
2. **Two Paths Exist:** Maybe there's a different reconstruction path using G1 shares directly
3. **Bug:** The check should be 32 bytes, not 48 bytes

**Recommendation:** Clarify the intended architecture:
- **Path A:** Submit scalar shares (32 bytes) - requires more trust/security
- **Path B:** Submit G1 shares (48 bytes) - requires different reconstruction logic

---

## Part 5: Testing Gap Analysis

### What's Tested in Rust ✅

**aptos-dkg tests** (`crates/aptos-dkg/src/ibe/tests.rs`):
- ✅ `reconstruct_ibe_dk()` with scalar shares
- ✅ Equal weights [1,1,1,1,1]
- ✅ Unequal weights [2,1,2]
- ✅ Sparse indices [0,2]
- ✅ Error cases (empty, mismatches)
- ✅ Encryption/decryption roundtrips

**Native function tests** (`aptos-move/framework/src/natives/cryptography/algebra/ibe_tests.rs`):
- ✅ Wrapper around `reconstruct_ibe_dk()`
- ✅ All scenarios above
- ✅ Uses real PVSS transcripts

**Coverage:** Complete for the scalar shares path

### What's NOT Tested in Move ❌

**Move tests** (`aptos-move/framework/aptos-framework/tests/ibe_native_test.move`):
- ✅ Data structure validation
- ✅ Identity computation
- ✅ Golden vector consistency
- ❌ **Native function calls** (zero tests)
- ❌ **DK reconstruction** (zero tests)

### Why Move Tests Can't Test Reconstruction

**The golden vector fixtures contain:**
```move
// aptos-framework/sources/ibe_golden_vector_fixtures.move
roundtrip_1_dk_shares(): vector<vector<u8>>  // 48-byte G1 shares
```

**But to test the native function, we need:**
```move
roundtrip_1_scalar_shares(): vector<vector<u8>>  // 32-byte scalar shares
```

**The problem:**
- Golden vectors have `dk_share_i = s_i × H(identity)` (G1 points)
- Cannot extract `s_i` from `dk_share_i` (discrete log problem)
- Cannot generate valid `s_i` in Move (no crypto primitives)

**Why fixtures have G1 shares:**
- They represent the on-chain data model
- In production, validators never publish raw scalar shares
- G1 shares are what appears in `submit_dk_share()`

### The Architectural Mismatch

```
Production Flow (On-Chain):
Validator holds s_i (32 bytes) → Computes dk_share_i (48 bytes) → Submits G1
    → Reconstruction with G1 shares → DK (48 bytes)

Testing Flow (Rust):
PVSS generates s_i (32 bytes) → Pass to reconstruct_ibe_dk() → DK (48 bytes)

Missing Flow (Move):
Golden vectors have dk_shares (48 bytes) → Cannot test native function (needs 32 bytes)
```

---

## Part 6: Recommendations

### 1. Fix the Security Gap (CRITICAL)

**Option A: Store Validator Public Key Shares**
```move
// During DKG, publish:
public(friend) fun set_validator_pk_shares(
    validator_index: u64,
    pk_shares: vector<vector<u8>>,  // Vec<G2>
)

// During submission, verify:
public(friend) fun submit_dk_share_verified(
    timelock_id: u64,
    dk_share: vector<u8>,
    validator_address: address,
) {
    let pk_shares = get_validator_pk_shares(validator_index);
    assert!(verify_dk_share(dk_share, identity, pk_shares), E_INVALID_DK_SHARE);
    ...
}
```

**Implementation:**
- Add native function: `verify_dk_share(dk: G1, identity: bytes, pk_shares: Vec<G2>) -> bool`
- Check: `e(dk, g₂) == e(H(identity), aggregate(pk_shares))`

**Option B: Verify Reconstructed DK Only**
```move
fun reconstruct_and_store_dk(...) {
    let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(...);

    // Verify before storing
    assert!(
        ibe::verify_decryption_key(reconstructed_dk, identity, mpk),
        E_INVALID_RECONSTRUCTED_DK
    );

    timelock_info.decryption_key = reconstructed_dk;
}
```

**Implementation:**
- Add native function: `verify_decryption_key(dk: G1, identity: bytes, mpk: G2) -> bool`
- Check: `e(dk, g₂) == e(H(identity), mpk)`
- On failure, reject all submitted shares and punish validators

### 2. Fix the Type Mismatch (URGENT)

**Clarify the architecture:**

**If using scalar shares (32 bytes):**
```move
const SCALAR_LENGTH: u64 = 32;

public(friend) fun submit_dk_share_scalar(
    timelock_id: u64,
    scalar_share: vector<u8>,  // 32 bytes!
    ...
) {
    assert!(vector::length(&scalar_share) == SCALAR_LENGTH, ...);
    ...
}
```
- **Problem:** Exposes secret shares on-chain
- **Security:** Requires immediate reconstruction to minimize exposure

**If using G1 shares (48 bytes) - RECOMMENDED:**
```move
const G1_LENGTH: u64 = 48;

public(friend) fun submit_dk_share_g1(
    timelock_id: u64,
    dk_share: vector<u8>,  // 48 bytes!
    ...
) {
    assert!(vector::length(&dk_share) == G1_LENGTH, ...);
    // Verify with pk_shares (see above)
    ...
}
```
- **Requires:** Different reconstruction logic that aggregates G1 points directly
- **Security:** Better - no secret exposure
- **Implementation:** Use Lagrange coefficients on G1 points, not scalars

### 3. Address the Testing Gap (LOW PRIORITY)

**Option A: Accept the Gap**
- Rust tests provide complete coverage
- Move tests validate data structures only
- Document this architectural decision

**Option B: Add Scalar Share Fixtures**
```move
// For testing only - NEVER use in production
#[test_only]
public fun roundtrip_1_scalar_shares(): vector<vector<u8>> {
    vector[
        x"...",  // 32-byte scalar share for validator 0
        x"...",  // 32-byte scalar share for validator 1
        ...
    ]
}

#[test]
fun test_native_function_reconstruction() {
    let dk = ibe::reconstruct_ibe_dk<G1>(
        fixtures::roundtrip_1_validator_indices(),
        fixtures::roundtrip_1_scalar_shares(),
        ...
    );
    assert!(dk == fixtures::roundtrip_1_reconstructed_dk(), 0);
}
```

Generate fixtures from Rust tests where PVSS transcripts exist.

---

## Part 7: Answers to Your Questions

### Q: "How does the validator prove their secret share is part of the MPK?"

**Answer:** Via their public key share `pk_i` from the PVSS transcript.

**Proof:**
```
1. Validator submits dk_share_i
2. System retrieves validator's pk_shares (from on-chain storage)
3. System computes: e(dk_share_i, g₂) vs e(H(identity), aggregate(pk_shares))
4. If equal, then dk_share_i = (sum s_i[j]) × H(identity)
5. Combined with pk_shares = s_i × g₂, proves s_i is valid
```

**Why it works:**
- PVSS transcript ensures `pk_i = s_i × g₂` during DKG
- Pairing check ensures `dk_share_i = s_i × H`
- No way to fake this without knowing `s_i`

**What's needed:**
- ❌ Store `pk_shares` for each validator on-chain (currently missing)
- ❌ Add native `verify_dk_share()` function (currently missing)
- ❌ Call verification in `submit_dk_share()` (currently missing)

### Q: "How does Move land only see MPK and dk_share?"

**Current (buggy):**
```
On-chain: MPK (G2) + dk_shares (48-byte G1)
Off-chain: PVSS shares s_i (32-byte scalars)
Missing: pk_shares (G2) for verification
```

**Should be:**
```
On-chain:
  - MPK (G2) ✅
  - pk_shares per validator (Vec<G2>) ❌ MISSING
  - dk_shares (G1) ✅
  - Reconstructed DK (G1) ✅

Off-chain:
  - PVSS scalar shares s_i ✅ (validator private state)
```

### Q: "Share that evidence on chain?"

**Evidence = Public Key Shares**

**When to store:** During DKG completion
```move
// Called by DKG module after transcript aggregation
public(friend) fun set_dkg_validator_keys(
    epoch: u64,
    validator_indices: vector<u64>,
    pk_shares: vector<vector<vector<u8>>>,  // pk_shares[i][j] is G2
) {
    // Store for verification during DK share submission
}
```

**When to use:** During DK share submission
```move
public(friend) fun submit_dk_share(
    timelock_id: u64,
    dk_share: vector<u8>,
    validator_address: address,
) {
    let validator_index = stake::get_validator_index(validator_address);
    let pk_shares = get_validator_pk_shares(validator_index);

    // Verify: e(dk_share, g₂) == e(H(identity), aggregate(pk_shares))
    assert!(verify_dk_share(dk_share, identity, pk_shares), E_INVALID_DK_SHARE);

    // Store only if valid
    vector::push_back(&mut timelock_info.submitted_shares, dk_share);
}
```

---

## Conclusion

**Testing Gap:** Low severity - Rust tests provide ground truth, Move tests would duplicate coverage

**Security Gap:** HIGH SEVERITY - No verification of submitted DK shares enables:
- Malicious validators submitting garbage
- Griefing attacks preventing decryption
- No slashing mechanism for misbehavior

**Implementation Bug:** CRITICAL - Type mismatch between 32-byte and 48-byte shares breaks reconstruction

**Immediate Actions Required:**
1. Fix type mismatch (clarify scalar vs. G1 submission)
2. Store validator public key shares on-chain
3. Implement cryptographic verification of DK shares
4. Add `verify_dk_share()` and `verify_decryption_key()` native functions

**Testing Gap Resolution:** Accept as architectural decision, document thoroughly
