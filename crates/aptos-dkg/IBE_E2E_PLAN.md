# IBE End-to-End Reference Implementation Plan

## Executive Summary

This document outlines the implementation plan for creating a comprehensive end-to-end IBE (Identity-Based Encryption) protocol test in Rust for the Aptos DKG codebase. The test demonstrates the complete flow from distributed key generation through IBE encryption and decryption without requiring Move VM or validator functions.

## Background

### Current State

The test file `tests/ibe_end_to_end.rs` exists but contains fundamental architectural and type system issues that prevent compilation. Previous attempts focused on module visibility and type aliases, but the root cause is a misunderstanding of the PVSS/DKG type system for weighted configurations.

### Goal

Create a reference implementation demonstrating:

1. **BLS Key Setup** - Configure DKG with weighted validators
2. **Distributed Key Generation** - Generate secret shares and master public key (MPK)
3. **IBE Encryption** - Encrypt a message for a timelock identity
4. **Secret Key Share Decryption** - Decrypt shares from DKG transcripts
5. **Threshold Reconstruction** - Reconstruct the decryption key
6. **Message Decryption** - Successfully decrypt and verify the message

## Root Cause Analysis

### Issue 1: Type System Misunderstanding

**Problem**: The `DealtSecretKeyShare` type for `das::WeightedTranscript` is not a single share object, but a **vector of shares**.

From `pvss/das/weighted_protocol.rs:92`:

```rust
type DealtSecretKeyShare = Vec<pvss::dealt_secret_key_share::g1::DealtSecretKeyShare>;
```

**Why**: Weighted configurations assign multiple share points to each validator based on their weight. A validator with weight `w` receives `w` shares.

**Impact**:

- Cannot use `DealtSecretKeyShare` directly as a single object
- Type inference fails because Rust sees `Vec<...>` instead of the expected tuple type
- Methods like `as_group_element()` don't exist on `Vec<T>`

### Issue 2: Cryptographic Architecture Error

**Problem**: The test attempts to reconstruct MSK as a scalar by converting group elements to scalars.

From `ibe_end_to_end.rs:225-232`:

```rust
let scalar = Scalar::from_repr_vartime(share_element.to_bytes())
    .expect("Failed to convert group element to scalar");
```

**Why This Fails**:

- `DealtSecretKey` is a **group element** (G1Projective), not a scalar
- The relationship is one-way: `DealtSecretKey = G1::generator() * InputSecret`
- Cannot extract the discrete logarithm efficiently (computationally infeasible)
- PVSS is designed to reconstruct **group elements**, not scalars

**Correct Approach**: Reconstruct the `DealtSecretKey` as a G1 group element using Lagrange interpolation in the group.

### Issue 3: Compilation Errors

1. **Duplicate imports**: `use anyhow::Result;` appears twice (lines 18-19)
2. **Wrong import path**: Using `das::DealingArgs` instead of `test_utils::DealingArgs`
3. **Undefined type**: `DkgShareType` is not defined in scope

## Implementation Plan

### Phase 1: Fix Compilation Errors

#### Task 1.1: Clean Up Imports

**File**: `tests/ibe_end_to_end.rs`

**Changes**:

```rust
// BEFORE (lines 18-29)
use anyhow::Result;
use anyhow::Result;  // ❌ Duplicate
use aptos_dkg::{
    ibe::{...},
    pvss::{
        das,
        test_utils::{setup_dealing, NoAux},
        ...
    },
};

// AFTER
use anyhow::Result;
use aptos_dkg::{
    ibe::{
        compute_timelock_identity, ibe_decrypt, ibe_encrypt,
        serialize_g1, serialize_g2, Ciphertext,
    },
    pvss::{
        das,
        dealt_secret_key::g1::DealtSecretKey,  // ✅ Add this
        test_utils::{setup_dealing, DealingArgs, NoAux},  // ✅ Add DealingArgs
        traits::{Reconstructable, SecretSharingConfig, ThresholdConfig, Transcript},
        Player, WeightedConfig,
    },
};
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::PrimeField;
use group::Group;
use rand::thread_rng;
```

#### Task 1.2: Add Type Alias

**Location**: After imports, before test functions

**Add**:

```rust
// Type alias for weighted DKG shares (Vec of shares, one per weight unit)
type WeightedDkgShare = <das::WeightedTranscript as Transcript>::DealtSecretKeyShare;
```

#### Task 1.3: Fix E2EResult Structure

**Change** (line 73):

```rust
// BEFORE
dealing_args: das::DealingArgs<das::WeightedTranscript>,

// AFTER
dealing_args: DealingArgs<das::WeightedTranscript>,
```

**Remove** (lines 76, 299-301):

```rust
// REMOVE - Cannot verify scalar directly
reconstructed_msk_scalar: Scalar,
```

### Phase 2: Fix Core Reconstruction Logic

This is the **critical architectural change**.

#### Task 2.1: Update Share Decryption Pattern

**Location**: Lines 184-209

**BEFORE** (Incorrect):

```rust
let mut decrypted_shares: Vec<(Player, DkgShareType)> = Vec::new();
for player in &eligible_players {
    let player_id = player.get_id();
    let (sk_share, pk_share): (DkgShareType, _) = aggregated_trx.decrypt_own_share(
        &wconfig,
        player,
        &dealing_args.dks[player_id],
        &dealing_args.pp,
    );
    // ...
    decrypted_shares.push((*player, sk_share));
}
```

**AFTER** (Correct):

```rust
debug!("Decrypting secret key shares from transcripts...");

let eligible_players: Vec<Player> = wconfig
    .get_random_eligible_subset_of_players(&mut rng)
    .into_iter()
    .take(config.threshold)
    .collect();

debug!(
    "Selected {} players: {:?}",
    eligible_players.len(),
    eligible_players.iter().map(|p| p.get_id()).collect::<Vec<_>>()
);

// Decrypt shares from each player
let players_and_shares: Vec<(Player, WeightedDkgShare)> = eligible_players
    .iter()
    .map(|player| {
        let player_id = player.get_id();
        let (sk_share, pk_share): (WeightedDkgShare, _) = aggregated_trx.decrypt_own_share(
            &wconfig,
            player,
            &dealing_args.dks[player_id],
            &dealing_args.pp,
        );

        assert_eq!(
            pk_share,
            aggregated_trx.get_public_key_share(&wconfig, player),
            "Public key share mismatch for player {}",
            player_id
        );

        debug!("Player {} decrypted {} share(s)", player_id, sk_share.len());
        (*player, sk_share)
    })
    .collect();
```

#### Task 2.2: Replace Scalar Reconstruction with Group Element Reconstruction

**Location**: Lines 210-244

**BEFORE** (Incorrect - tries to reconstruct scalars):

```rust
// Compute H(identity) = Q_id
let identity = compute_timelock_identity(config.timelock_id, config.deadline_us);
let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

// Reconstruct MSK scalar from shares
let mut shares_for_reconstruction: Vec<(Player, Scalar)> = Vec::new();
for (player, share) in &decrypted_shares {
    let share_element = share.as_group_element();
    let scalar = Scalar::from_repr_vartime(share_element.to_bytes())  // ❌ WRONG!
        .expect("Failed to convert group element to scalar");
    shares_for_reconstruction.push((*player, scalar));
}

let reconstructed_msk_scalar =
    Scalar::reconstruct(wconfig.get_threshold_config(), &shares_for_reconstruction);

assert_eq!(reconstructed_msk_scalar, msk_scalar, "MSK reconstruction failed");
```

**AFTER** (Correct - reconstructs group elements):

```rust
debug!("Reconstructing decryption key shares for IBE...");

// Reconstruct the DealtSecretKey (MSK as G1 group element)
let reconstructed_msk = DealtSecretKey::reconstruct(
    wconfig.get_threshold_config(),
    &players_and_shares,
);

debug!("MSK reconstructed from {} shares", players_and_shares.len());
```

#### Task 2.3: Fix IBE Decryption Key Computation

**Location**: Lines 245-259

**BEFORE**:

```rust
// DK = MSK * Q_id
let decryption_key = q_id * reconstructed_msk_scalar;  // ❌ Using scalar

// Verify DK matches what we get from the scalar directly
let expected_dk = q_id * msk_scalar;
assert_eq!(decryption_key, expected_dk, "DK mismatch");
```

**AFTER**:

```rust
debug!("Computing IBE decryption key...");

// Compute the identity hash Q_id = H(identity)
let identity = compute_timelock_identity(config.timelock_id, config.deadline_us);
let q_id = G1Projective::hash_to_curve(&identity, b"APTOS_BLS_WVUF_DST", b"H(m)");

// Compute decryption key: DK = MSK * Q_id (scalar multiplication in G1)
// MSK is the reconstructed DealtSecretKey (a G1 group element)
let msk_element = reconstructed_msk.as_group_element();
let decryption_key = q_id * msk_element;  // ✅ Correct: G1 * G1 = G1

debug!("Decryption key computed");
```

### Phase 3: Verify MSK Functional Correctness

Since we cannot compare the reconstructed MSK to the original scalar directly, we verify functionality.

#### Task 3.1: Add MPK Verification

**Location**: After MSK reconstruction (new code)

**Add**:

```rust
debug!("Verifying MSK functional correctness...");

// Verify the reconstructed MSK produces the correct MPK
// MPK should equal G2::generator() * InputSecret
// We can verify this by checking e(G1::gen, MPK) == e(MSK, G2::gen)
// Since MSK = G1::gen * InputSecret
use blstrs::{pairing, G1Affine, G2Affine};
use group::Curve;

let g1_gen = G1Projective::generator();
let g2_gen = G2Projective::generator();

let lhs = pairing(&g1_gen.to_affine(), &mpk.to_affine());
let rhs = pairing(&msk_element.to_affine(), &g2_gen.to_affine());

assert_eq!(
    lhs, rhs,
    "MSK verification failed: reconstructed MSK does not match MPK"
);

debug!("MSK verification passed - matches MPK via pairing");
```

**Explanation**: This uses bilinear pairing to verify:

- `e(g1, MPK) = e(g1, g2^s)` where `s` is the InputSecret
- `e(MSK, g2) = e(g1^s, g2)` where `MSK = g1^s` is the reconstructed DealtSecretKey
- These are equal if MSK was reconstructed correctly

#### Task 3.2: Add Decryption Key Verification

**Location**: After decryption key computation

**Add**:

```rust
// Additional verification: Different share subsets should produce the same DK
// This is implicitly verified by successful decryption, but we can be explicit
let dk_bytes = serialize_g1(&decryption_key).expect("DK serialization failed");
debug!("Decryption key: {} bytes", dk_bytes.len());
```

### Phase 4: Update Logging Strategy

#### Task 4.1: Add Debug Macro Usage

**Location**: Top of file, after imports

**Add**:

```rust
// Debug logging (controlled by RUST_LOG environment variable)
use log::debug;

// For tests, enable logging with: RUST_LOG=debug cargo test
```

#### Task 4.2: Replace println! with debug!

**Strategy**:

- Replace verbose `println!` statements with `debug!` macro
- Keep critical assertions and test structure visible
- Remove redundant hex dumps (only show in debug mode)

**Example transformations**:

```rust
// BEFORE
println!("  - Generated {} signing key pairs", dealing_args.ssks.len());
println!("  - MSK scalar (from aggregated InputSecrets): {:02x?}",
    &msk_scalar.to_bytes_le()[0..16]);

// AFTER
debug!("Generated {} signing key pairs", dealing_args.ssks.len());
debug!("MSK scalar: {:02x?}", &msk_scalar.to_bytes_le()[0..16]);
```

Keep only:

```rust
println!("\n=== IBE End-to-End Protocol Test ===\n");
println!("PHASE 1: Setting up DKG...");
println!("PHASE 2: Running DKG...");
// ... etc
println!("\n=== IBE END-TO-END TEST PASSED ===\n");
```

### Phase 5: Update All Test Functions

Apply the same fixes to all 4 test functions:

1. **`test_ibe_end_to_end()`** - Main comprehensive test
2. **`test_ibe_end_to_end_3_of_5()`** - Different validator count
3. **`test_ibe_end_to_end_share_subset()`** - Multiple share subsets
4. **`test_ibe_end_to_end_weighted()`** - Non-uniform weights

#### Task 5.1: Extract Common Logic

Create a helper function to reduce duplication:

```rust
/// Reconstructs the MSK from decrypted shares and computes the IBE decryption key
fn reconstruct_and_compute_decryption_key(
    wconfig: &WeightedConfig,
    players_and_shares: &Vec<(Player, WeightedDkgShare)>,
    identity: &[u8],
) -> (DealtSecretKey, G1Projective) {
    debug!("Reconstructing MSK from {} shares", players_and_shares.len());

    let reconstructed_msk = DealtSecretKey::reconstruct(
        wconfig.get_threshold_config(),
        players_and_shares,
    );

    let q_id = G1Projective::hash_to_curve(identity, b"APTOS_BLS_WVUF_DST", b"H(m)");
    let decryption_key = q_id * reconstructed_msk.as_group_element();

    (reconstructed_msk, decryption_key)
}
```

#### Task 5.2: Update Each Test Function

For each test:

1. Fix imports and type aliases
2. Update share decryption to use `WeightedDkgShare`
3. Replace scalar reconstruction with group element reconstruction
4. Use helper function for common logic
5. Add MPK verification via pairing
6. Update logging to use `debug!`

### Phase 6: Testing and Validation

#### Task 6.1: Compilation Check

```bash
cargo build -p aptos-dkg --tests
```

**Expected**: Clean compilation with no errors or warnings

#### Task 6.2: Run Tests

```bash
# Run without debug output
cargo test -p aptos-dkg test_ibe_end_to_end

# Run with debug output
RUST_LOG=debug cargo test -p aptos-dkg test_ibe_end_to_end -- --nocapture
```

**Expected**: All 4 tests pass

#### Task 6.3: Verify Test Coverage

Ensure tests cover:

- ✅ Basic 3-of-4 threshold (main test)
- ✅ Different configuration: 3-of-5
- ✅ Share subset variation: Different player subsets yield same result
- ✅ Weighted validators: Non-uniform weights

#### Task 6.4: Code Quality Check

```bash
cargo clippy -p aptos-dkg --tests
cargo fmt -p aptos-dkg --check
```

**Expected**: No warnings or formatting issues

## Success Criteria

### Functional Requirements

1. ✅ All 4 test functions compile successfully
2. ✅ All tests pass when executed
3. ✅ MSK reconstruction verified via pairing with MPK
4. ✅ IBE encryption/decryption works end-to-end
5. ✅ Different share subsets produce identical decryption keys
6. ✅ Weighted configurations handle multiple shares per validator correctly

### Code Quality Requirements

1. ✅ No compilation errors or warnings
2. ✅ No clippy warnings
3. ✅ Proper formatting (rustfmt)
4. ✅ Clear, concise comments explaining the protocol flow
5. ✅ Debug logging for troubleshooting without verbose println!

### Documentation Requirements

1. ✅ Test module documentation explains the IBE protocol flow
2. ✅ Comments explain cryptographic operations (pairing verification, etc.)
3. ✅ Helper functions have clear docstrings
4. ✅ Non-obvious type conversions are documented

## Future Enhancements (Post Happy-Path)

After the happy path works:

### Negative Test Cases

1. **Insufficient shares**: Verify reconstruction fails with < threshold shares
2. **Invalid identity**: Verify decryption fails with mismatched identity
3. **Corrupted ciphertext**: Verify decryption fails with tampered data
4. **Invalid player subset**: Verify reconstruction fails with invalid player IDs

### Performance Tests

1. **Benchmark reconstruction time** with varying threshold values
2. **Measure encryption/decryption overhead**
3. **Profile memory usage** for large messages

### Additional Scenarios

1. **Time-based identity testing**: Multiple timelock deadlines
2. **Batch encryption**: Multiple messages with same identity
3. **Share refresh**: Update shares without changing MSK

## Technical Reference

### Key Type Relationships

```
InputSecret (Scalar in F_r)
    ↓ (one-way: multiply by G1::gen)
DealtSecretKey (G1Projective)
    ↓ (Shamir secret sharing in G1)
DealtSecretKeyShare (G1Projective, per share)
    ↓ (for weighted configs)
Vec<DealtSecretKeyShare> (WeightedDkgShare)
    ↓ (Lagrange reconstruction in G1)
DealtSecretKey (reconstructed)
```

### Pairing Verification

```
Given:
- InputSecret s ∈ F_r
- MPK = g2^s ∈ G2
- MSK = g1^s ∈ G1

Verify: e(g1, MPK) ?= e(MSK, g2)

Proof:
  e(g1, MPK) = e(g1, g2^s) = e(g1, g2)^s
  e(MSK, g2) = e(g1^s, g2) = e(g1, g2)^s
  ∴ They are equal iff MSK = g1^s
```

### IBE Decryption Key

```
Given:
- Identity: id (bytes)
- MSK: g1^s ∈ G1

Compute:
1. Q_id = H(id) ∈ G1  (hash to curve)
2. DK = Q_id · s = H(id)^s ∈ G1  (scalar mult)

Note: Since MSK = g1^s, we have:
  DK = Q_id * MSK (in additive notation)
  DK = Q_id^MSK (in multiplicative notation)
```

## Dependencies and Prerequisites

- Rust toolchain (edition 2021)
- `aptos-dkg` crate with all dependencies
- `blstrs` for BLS12-381 curve operations
- `log` crate for debug logging
- Test environment with sufficient entropy for RNG

## Timeline Estimate

- **Phase 1** (Compilation fixes): 30 minutes
- **Phase 2** (Core reconstruction): 1 hour
- **Phase 3** (MSK verification): 45 minutes
- **Phase 4** (Logging updates): 30 minutes
- **Phase 5** (Apply to all tests): 1 hour
- **Phase 6** (Testing and validation): 1 hour

**Total**: ~5 hours for complete happy-path implementation

## Conclusion

This plan provides a systematic approach to fixing the IBE end-to-end test by:

1. Correcting fundamental misunderstandings of the type system
2. Using the proper cryptographic reconstruction approach (group elements, not scalars)
3. Verifying MSK functionality via pairing with MPK
4. Creating clean, maintainable test code with appropriate logging

The resulting implementation will serve as a **reference example** for how to use the Aptos DKG and IBE libraries correctly, demonstrating the complete protocol flow from key generation through encryption and decryption.

---

## Cooperative IBE Test Pattern (Production Reference)

### Background: Atomica Timelock as Reference Implementation

After comprehensive research, we found that **Atomica Timelock** implements the cooperative threshold decryption pattern we need for IBE tests. There is NO separate "encrypted mempool" - the timelock system IS the production cooperative pattern.

**Key Production Files:**

- `/dkg/src/epoch_manager.rs:866-954` - Share revelation logic
- `/aptos-move/framework/aptos-framework/sources/threshold_dsa.move:232-267` - On-chain aggregation
- `/crates/aptos-dkg/src/ibe/mod.rs:125-136` - `derive_decryption_key()`
- `/atomica/docs/definitions.md:342-347` - Architecture overview

### Production Pattern Flow

```
1. DKG Setup → Each validator stores IbeShare { scalar: Scalar, comm: G1 }
2. Reveal Request → Event triggers cooperative share revelation
3. DK Share Computation → derive_decryption_key(scalar, identity) returns G1 point
4. Submission → ValidatorTransaction::TimelockShare { validator_idx, share: G1 }
5. Aggregation → On-chain Lagrange: DK = Σ(λ_i × DK_share_i) [G1 group]
```

**Critical Insight:** Validators submit **G1 points** (48 bytes), NOT scalars!

### Test Implementation Strategy

#### Phase 1: Deterministic Test (FIRST)

- **File:** `tests/ibe_cooperative_deterministic.rs`
- **Purpose:** Prove cooperative pattern works with known values
- **Approach:** Hardcoded master secret and validator shares
- **Benefits:** Deterministic, debuggable, reproducible

#### Phase 2: DKG-Based Deterministic Test

- **File:** Same file, additional test
- **Purpose:** Use real DKG with fixed seed
- **Approach:** `StdRng::from_seed([42u8; 32])`

#### Phase 3: Random Tests

- **File:** Add to existing `ibe_end_to_end.rs` or separate
- **Purpose:** Full randomized testing
- **Approach:** Use `thread_rng()`

### Comparison: Sanity vs Cooperative Tests

| Aspect             | Sanity Tests                  | Cooperative Tests                            |
| ------------------ | ----------------------------- | -------------------------------------------- |
| **Location**       | `ibe_end_to_end.rs`           | `ibe_cooperative_deterministic.rs`           |
| **MSK Access**     | Direct scalar (test shortcut) | No MSK reconstruction                        |
| **DK Computation** | `h_id × msk_scalar`           | Per-validator: `derive_decryption_key()`     |
| **Share Format**   | Uses full MSK                 | Uses `IbeShare.as_scalar()` or manual shares |
| **Aggregation**    | None (single DK)              | Lagrange in G1 group                         |
| **Pattern Source** | Test-only                     | Production (epoch_manager.rs)                |
| **Randomness**     | Random                        | Deterministic first, then random             |
| **Realism**        | Low                           | High                                         |

### Mathematical Correctness

Each validator computes:

```
DK_share_i = s_i × H(identity)  [G1 point, not scalar!]
```

Aggregation reconstructs full DK:

```
DK = Σ (λ_i × DK_share_i)
   = Σ (λ_i × (s_i × H(identity)))
   = (Σ λ_i × s_i) × H(identity)
   = s × H(identity)  [by Lagrange interpolation]
```

where `s` is the master secret reconstructed from threshold shares.

### Key Differences from Initial Plan

**WRONG (Initial Understanding):**

- ❌ Reconstruct MSK as scalar from G1 points (impossible - discrete log)
- ❌ Each validator decrypts separately then combines
- ❌ Scalar aggregation

**RIGHT (Production Pattern):**

- ✅ Each validator computes G1 point: `scalar_share × H(identity)`
- ✅ Aggregate G1 points using Lagrange coefficients
- ✅ G1 group aggregation (not scalar field)
- ✅ Follows drand timelock pattern (BLS signature ≡ IBE DK)

### Implementation Notes

**Hardcoded Test Values:**

```rust
// Master secret (for verification only, not used in cooperative flow)
let msk_scalar = Scalar::from(0x1234_5678_9abc_def0u64);

// Validator shares (would come from DKG in production)
let validator_shares = vec![
    (0, Scalar::from(100u64)),  // Simple values for clarity
    (1, Scalar::from(200u64)),
    (2, Scalar::from(300u64)),
];

// Fixed identity
let timelock_id = 42u64;
let deadline = 1704070800000000u64;
```

**Lagrange Aggregation:**

```rust
// For educational clarity, implement manually first
fn compute_lagrange_coeff(j: u64, ids: &[u64]) -> Scalar {
    // λ_j = Π_{k≠j} (0 - x_k) / (x_j - x_k)
    // Interpolate at point 0 (standard for threshold crypto)
}

// Later, use production code:
use aptos_dkg::algebra::lagrange::lagrange_coefficients;
```

### Success Criteria

**Phase 1 Complete When:**

- ✅ Deterministic test with hardcoded shares passes
- ✅ Manual Lagrange implementation verified
- ✅ Test reproduces same output every run
- ✅ Code matches production pattern from `epoch_manager.rs`

**Phase 2 Complete When:**

- ✅ DKG-based test with fixed seed passes
- ✅ Uses actual PVSS/DKG infrastructure
- ✅ Still deterministic (same seed = same output)

**Phase 3 Complete When:**

- ✅ Random tests pass
- ✅ Multiple runs with different keys all pass
- ✅ Weighted configurations tested
