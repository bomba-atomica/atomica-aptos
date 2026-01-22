# IBE Implementation Summary

**Status:** Phase 5.2 COMPLETE (January 22, 2026)
**Branch:** `timelock-elgamal-pvss`

---

## Quick Reference

| Component           | Status                 | Location                            |
| ------------------- | ---------------------- | ----------------------------------- |
| IBE Primitives      | ✅ Complete            | `crates/aptos-dkg/src/ibe/mod.rs`   |
| Scalar ElGamal PVSS | ✅ Complete            | `crates/aptos-dkg/src/pvss/`        |
| Timelock Registry   | ✅ Complete            | `ibe_config.move`                   |
| DK Reconstruction   | ✅ Complete            | `reconstruct_ibe_dk()`              |
| Golden Vectors      | ✅ Complete            | `ibe_golden_vector_fixtures.move`   |
| Move Tests          | ✅ Complete (11 tests) | `ibe_native_test.move`              |
| Rust Tests          | ✅ Complete (28 tests) | `crates/aptos-dkg/src/ibe/tests.rs` |

---

## Architecture

### Dual-Output DKG

```
InputSecret (scalar a)
        │
        ├─────────────────────┬────────────────────────────┐
        │                     │                            │
        ▼                     ▼                            ▼
   ┌─────────┐      ┌─────────────────────┐      ┌─────────────────┐
   │DAS PVSS │      │Chunked Lifted       │      │   MPK = g2^a    │
   │         │      │ElGamal PVSS         │      │  (on-chain)     │
   └────┬────┘      └──────────┬──────────┘      └─────────────────┘
        │                      │
        ▼                      ▼
   G1 shares              Scalar shares
   (WVUF/randomness)      (IBE DK reconstruction)
```

### DK Reconstruction Flow

```
Input: (validator_indices, scalar_shares, weights, total_weight, identity)
       │
       ▼
1. Wrap scalar_shares in DealtSecretKeyShare format
       │
       ▼
2. Delegate to WeightedConfig::new() + DealtSecretKey::reconstruct()
       │
       ▼
3. Derive DK = H(identity)^secret
       │
       ▼
Output: G1Affine (reconstructed decryption key)
```

---

## Key Files

### Core Implementation

| File                                                              | Purpose                                          |
| ----------------------------------------------------------------- | ------------------------------------------------ |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | Rust SDK IBE primitives + `reconstruct_ibe_dk()` |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function                                  |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move API                                         |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move`    | Timelock registry                                |

### Tests

| File                                                              | Tests | Purpose                    |
| ----------------------------------------------------------------- | ----- | -------------------------- |
| `crates/aptos-dkg/src/ibe/tests.rs`                               | 28    | Rust SDK tests             |
| `aptos-move/framework/aptos-framework/tests/ibe_native_test.move` | 11    | Move native function tests |
| `crates/aptos-dkg/src/ibe/golden_vectors.rs`                      | -     | Golden vector generation   |

### Fixtures

| File                                                                           | Format         |
| ------------------------------------------------------------------------------ | -------------- |
| `atomica/golden_vectors/ibe_golden_vectors.json`                               | JSON           |
| `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move` | Move constants |

---

## API Reference

### Rust SDK

```rust
// Main reconstruction function
pub fn reconstruct_ibe_dk(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],  // Per-validator, per-virtual-player
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> G1Affine

// Identity computation
pub fn compute_identity(timelock_id: u64, deadline_us: u64) -> [u8; 32]

// DK derivation
pub fn derive_decryption_key(msk: &Scalar, identity: &[u8; 32]) -> G1Affine
```

### Move VM

```move
// Native function call
public fun reconstruct_ibe_dk<G1>(
    validator_indices: vector<u64>,
    scalar_shares: vector<vector<u8>>,  // Nested: validator -> virtual_player
    weights: vector<u64>,
    threshold: u64,
    total_weight: u64,
    identity: vector<u8>,
): crypto_algebra::Element<G1>
```

### Golden Vector Format

```json
{
  "dk_shares_g1_hex": [
    ["<validator_0_virtual_0>", "<validator_0_virtual_1>"],
    ["<validator_1_virtual_0>"],
    ["<validator_2_virtual_0>", "<validator_2_virtual_1>"]
  ]
}
```

---

## Test Results

### Rust SDK Tests

```
28 tests pass ✅
├── test_reconstruct_ibe_dk_single_share
├── test_reconstruct_ibe_dk_equal_weights
├── test_reconstruct_ibe_dk_unequal_weights
├── test_reconstruct_ibe_dk_sparse_indices
├── test_compare_scalar_and_g1_reconstruction
├── test_dk_share_aggregation_roundtrip
├── test_golden_vectors_file_validity
└── ... (21 more)
```

### Move Native Function Tests

```
11 tests pass ✅
├── test_native_reconstruction_5_validators_equal_weights
├── test_native_reconstruction_4_validators_threshold_2
├── test_native_reconstruction_unequal_weights_215
├── test_native_reconstruction_unequal_weights_2321
├── test_sparse_validator_participation
├── test_empty_shares_should_abort
├── test_mismatched_indices_and_shares_should_abort
└── ... (4 more)
```

---

## Design Decisions

### Decision: Scalar Share Reconstruction

**Date:** January 22, 2026

We implemented scalar share reconstruction with framework delegation, NOT G1-based reconstruction.

**Rationale:**

1. **Security** - Native functions should not implement crypto
2. **Consistency** - Move VM uses same crypto as Rust SDK
3. **Simplicity** - Framework handles virtual player expansion

### Decision: Golden Vector Format

**Format:** `Vec<Vec<String>>` = per-validator, per-virtual-player G1 shares

**Purpose:** Testing verification only - not for production reconstruction.

---

## Related Documentation

| Document                                      | Purpose                        |
| --------------------------------------------- | ------------------------------ |
| `plan/implementation-plan-unified-dkg-ibe.md` | Master implementation plan     |
| `ibe-implementation-vs-plan.md`               | What we planned vs what we did |
| `adr/adr-001-dual-output-dkg.md`              | Architecture decision record   |
| `definitions.md`                              | Cryptographic definitions      |
| `product-spec/atomica-timelock-spec.md`       | Product specification          |

---

## Open Items

| Item                             | Status            |
| -------------------------------- | ----------------- |
| `mpk_encrypt_decrypt` smoke test | Not yet run       |
| `timelock_e2e` smoke test        | Not yet run       |
| Security review                  | Not yet completed |
