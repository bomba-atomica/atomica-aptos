# IBE DK Reconstruction: Implementation vs Plan

**Date:** January 22, 2026
**Status:** Phase 5.2 COMPLETE

---

## Original Plan (What We Thought We'd Do)

### Plan A: G1 Reconstruction with Pre-Aggregated Shares

The original documentation described a path using **pre-aggregated G1 shares**:

```
// What we PLANNED to implement:
dk_share = (Σ s_{i,j}) × H(identity)  // Sum scalars first, then multiply
// One G1 point per validator
```

**Characteristics:**

- Each validator submits 1 G1 point (sum of their virtual player shares)
- Manual Lagrange coefficient computation
- Custom threshold configuration (bug-prone)
- Weight mapping complexity

### Plan B: Per-Virtual-Player G1 Shares (Alternative)

**Reference:** `ibe-weighted-g1-reconstruction-summary.md`

Change IBE to produce per-virtual-player shares:

```rust
// What the summary RECOMMENDED:
Vec<(Player, Vec<G1Projective>)>  // Per-virtual-player shares
```

**Benefits:**

- Leverage battle-tested DKG reconstruction
- Automatic correctness

**Costs:**

- More storage (w_i G1 points per validator)
- More bandwidth
- API changes

---

## What We Actually Implemented

### Implementation: Scalar Shares + Framework Delegation

**We did NOT use G1 shares for DK reconstruction. Instead:**

1. **Scalar shares from Chunked ElGamal PVSS** - Used directly
2. **Delegation to framework's `DealtSecretKey::reconstruct()`** - No custom crypto
3. **DK derived after reconstruction** - `derive_decryption_key(secret, identity)`

```
Input: (validator_indices, scalar_shares, weights, total_weight, identity)
       │
       ▼
Step 1: Wrap scalar_shares in DealtSecretKeyShare format
       │
       ▼
Step 2: Delegate to WeightedConfig::new() + DealtSecretKey::reconstruct()
       │
       ▼
Step 3: Derive DK = H(identity)^secret
       │
       ▼
Output: G1Affine (reconstructed decryption key)
```

### Key Files

| File                                                              | What Changed                                              |
| ----------------------------------------------------------------- | --------------------------------------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | Added `reconstruct_ibe_dk()` - delegates to framework     |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function - parses args, delegates to apt-dkg       |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move API accepting `vector<vector<u8>>` for scalar shares |
| `crates/aptos-dkg/src/ibe/golden_vectors.rs`                      | Golden vectors now use nested format for testing          |

### API Signatures

**Rust SDK:**

```rust
pub fn reconstruct_ibe_dk(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],  // Per-validator, per-virtual-player
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> G1Affine
```

**Move VM:**

```move
public fun reconstruct_ibe_dk<G1>(
    validator_indices: vector<u64>,
    scalar_shares: vector<vector<u8>>,  // Nested: validator -> virtual_player
    weights: vector<u64>,
    threshold: u64,
    total_weight: u64,
    identity: vector<u8>,
): crypto_algebra::Element<G1>
```

---

## Golden Vectors: Per-Virtual-Player G1 Format

**For testing only**, golden vectors now include per-virtual-player G1 shares:

```json
{
  "dk_shares_g1_hex": [
    ["<validator_0_virtual_0>", "<validator_0_virtual_1>"], // weight 2
    ["<validator_1_virtual_0>"], // weight 1
    ["<validator_2_virtual_0>", "<validator_2_virtual_1>"] // weight 2
  ]
}
```

**Format:** `Vec<Vec<String>>` = `validator_shares[validator_idx][virtual_player_idx]`

**Purpose:** Verify that per-virtual-player G1 shares reconstruct correctly to the same DK.

---

## What Changed From Original Plan

| Aspect               | Original Plan            | Actual Implementation              |
| -------------------- | ------------------------ | ---------------------------------- |
| **Share type**       | Pre-aggregated G1        | Scalar (via ElGamal PVSS)          |
| **Reconstruction**   | Custom Lagrange          | Framework delegation               |
| **Crypto location**  | Native function          | Rust SDK (apt-dkg)                 |
| **G1 shares**        | N/A (for reconstruction) | Only in golden vectors for testing |
| **Threshold config** | Manual (buggy)           | Framework automatic                |
| **Sparse indices**   | Complex manual mapping   | Framework handles                  |

---

## Why This Approach

### Decision: Unified DK Reconstruction Path

**Date:** January 22, 2026

**Rationale:**

1. **Security** - Native functions should not implement crypto. Delegation to apt-dkg reduces attack surface.

2. **Consistency** - Move VM uses the exact same crypto as Rust SDK. No implementation divergence.

3. **Maintainability** - Crypto updates only in Rust SDK. Single source of truth.

4. **Simplicity** - Framework handles virtual player expansion, threshold configuration, Lagrange coefficients.

### Trade-offs

| Pro                          | Con                               |
| ---------------------------- | --------------------------------- |
| Correct by construction      | Slightly more data (scalar vs G1) |
| Single code path             | N/A                               |
| Framework handles edge cases | N/A                               |

---

## Test Results

```
Rust SDK: 28 IBE tests pass ✅
- test_reconstruct_ibe_dk_equal_weights ✅
- test_reconstruct_ibe_dk_unequal_weights ✅
- test_reconstruct_ibe_dk_sparse_indices ✅
- test_golden_vectors_file_validity ✅
```

---

## Files Reference

### Implementation

| File                                                              | Purpose                                      |
| ----------------------------------------------------------------- | -------------------------------------------- |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | `reconstruct_ibe_dk()` - main implementation |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function bridge                       |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move API                                     |

### Tests

| File                                                              | Purpose                |
| ----------------------------------------------------------------- | ---------------------- |
| `crates/aptos-dkg/src/ibe/tests.rs`                               | Rust SDK tests         |
| `aptos-move/framework/aptos-framework/tests/ibe_native_test.move` | Move integration tests |

### Golden Vectors

| File                                                                           | Purpose                                  |
| ------------------------------------------------------------------------------ | ---------------------------------------- |
| `crates/aptos-dkg/src/ibe/golden_vectors.rs`                                   | Generator (generates Vec<Vec<G1Affine>>) |
| `atomica/golden_vectors/ibe_golden_vectors.json`                               | JSON fixtures (Vec<Vec<String>>)         |
| `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move` | Move fixtures                            |

---

## Legacy: G1 Reconstruction Functions (Deprecated)

The original G1 reconstruction code in `mod.rs:704-814` (`reconstruct_ibe_dk_from_g1_shares`) is **NOT used** for production IBE. It remains for:

1. Debugging comparison tests
2. Verifying golden vectors
3. Historical reference

**To use for testing:**

```rust
// In test_compare_scalar_and_g1_reconstruction
use super::reconstruct_ibe_dk_from_g1_shares;
```

---

## Summary

| Question                                | Answer                           |
| --------------------------------------- | -------------------------------- |
| Did we use pre-aggregated G1 shares?    | NO - we use scalar shares        |
| Did we implement custom reconstruction? | NO - delegated to framework      |
| Are golden vectors in nested format?    | YES - Vec<Vec<String>>           |
| Is native function implementing crypto? | NO - only parsing and delegation |

**Bottom line:** The implementation diverged from the original G1-focused plan to use the simpler, more secure, and more maintainable scalar share approach with framework delegation.
