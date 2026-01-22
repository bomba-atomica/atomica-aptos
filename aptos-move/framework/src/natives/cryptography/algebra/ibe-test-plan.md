# IBE DK Reconstruction Test Plan

This document outlines the test strategy for the IBE DK reconstruction native function.

## Overview

The IBE (Identity-Based Encryption) DK reconstruction is a critical component of the timelock encryption system. It allows validators to combine their secret key shares to reconstruct a decryption key (DK) for a given identity.

## Function Signature

```rust
pub fn reconstruct_ibe_dk(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> G1Affine
```

## Parameters

| Parameter           | Description                                                                   |
| ------------------- | ----------------------------------------------------------------------------- |
| `validator_indices` | Indices of participating validators (0-based, matching DKG player IDs)        |
| `scalar_shares`     | Scalar shares organized by validator (one vector per participating validator) |
| `weights`           | Full weights array for ALL validators in the network                          |
| `total_weight`      | Sum of all validator weights                                                  |
| `identity`          | 32-byte IBE identity (from `compute_identity()`)                              |

## Test Categories

### 1. Golden Vector Tests

- Load and parse `ibe_golden_vectors.json`
- Verify golden vector structure and content
- **Location**: `test_golden_vectors_loadable()`

### 2. End-to-End PVSS Tests

These tests use real PVSS transcripts to generate valid shares:

| Test                                            | Configuration                                                  | Purpose                           |
| ----------------------------------------------- | -------------------------------------------------------------- | --------------------------------- |
| `test_dk_reconstruction_three_validators_equal` | 3 validators, weights [1,1,1], threshold 3                     | Basic equal-weight reconstruction |
| `test_dk_reconstruction_unequal_weights_215`    | 3 validators, weights [2,1,2], threshold 3                     | Unequal-weight reconstruction     |
| `test_dk_reconstruction_five_validators`        | 5 validators, weights [1,1,1,1,1], threshold 3                 | Larger network reconstruction     |
| `test_dk_reconstruction_sparse_indices`         | 4 validators, weights [1,1,1,1], threshold 2, validators [0,2] | Sparse validator participation    |
| `test_dk_reconstruction_different_identities`   | 3 validators, equal weights                                    | Identity diversity verification   |

### 3. Panic/Edge Case Tests

| Test                                                    | Expected Panic                                              | Scenario                         |
| ------------------------------------------------------- | ----------------------------------------------------------- | -------------------------------- |
| `test_dk_reconstruction_share_weight_mismatch_panics`   | "Validator 0 has 2 shares but weight is 1"                  | Share count doesn't match weight |
| `test_dk_reconstruction_empty_shares_panics`            | "Cannot reconstruct DK from empty shares"                   | No validators participating      |
| `test_dk_reconstruction_indices_shares_mismatch_panics` | "validator_indices and scalar_shares must have same length" | Mismatched input lengths         |
| `test_dk_reconstruction_weight_sum_mismatch_panics`     | "total_weight (5) must equal sum of weights (3)"            | Total weight doesn't match sum   |

## Key Testing Principles

1. **Use Real PVSS Transcripts**: All positive tests use `setup_dealing()`, `WeightedTranscript::deal()`, and `decrypt_own_share()` to generate cryptographically valid shares. Randomly generated scalars don't form valid secret sharing scheme shares.

2. **End-to-End Verification**: Every positive test verifies encryption/decryption roundtrip:
   - Encrypt plaintext using MPK and identity
   - Decrypt using reconstructed DK
   - Assert decrypted matches original

3. **Deterministic RNG**: All tests use `StdRng::seed_from_u64()` for reproducibility.

## Running Tests

```bash
# Run all IBE tests
cargo test --package aptos-framework --lib ibe

# Run specific test
cargo test --package aptos-framework --lib ibe::test_dk_reconstruction_three_validators_equal

# Run with output
cargo test --package aptos-framework --lib ibe -- --nocapture
```

## Expected Results

All tests should pass:

- 5 positive end-to-end tests (encrypt/decrypt roundtrips)
- 4 panic/edge case tests
- 1 golden vector loading test

## Implementation Notes

The `reconstruct_ibe_dk()` function delegates to the apt-dkg framework's weighted reconstruction, ensuring cryptographic correctness. The tests verify:

1. **Weight handling**: The function correctly handles unequal validator weights
2. **Sparse indices**: Reconstruction works with non-consecutive validator indices
3. **Identity diversity**: Different identities produce different DKs
4. **Input validation**: Proper panics for invalid inputs

## Related Files

- `ibe.rs`: Native function implementation
- `ibe_tests.rs`: This test file
- `crates/aptos-dkg/src/ibe/mod.rs`: Canonical implementation
- `atomica/golden_vectors/ibe_golden_vectors.json`: Golden vectors
