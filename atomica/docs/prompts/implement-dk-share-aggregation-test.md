# Implement DK Share Aggregation Roundtrip Test

## Context

You need to implement a test that validates the **distributed decryption key (DK) reconstruction pattern** for IBE (Identity-Based Encryption) using DKG shares.

Currently, tests exist that reconstruct the master secret scalar first, then derive a DK. We need a test that shows the **distributed pattern** where validators compute G1 DK shares and aggregate them using Lagrange interpolation.

## Test Goals

**Primary**: Validate that aggregating G1 DK shares (`DK = Σ λ_i × dk_share_i`) produces a working decryption key.

**Verification**: Reconstruct the master secret scalar and verify that both approaches (G1 aggregation vs scalar reconstruction) produce identical decryption keys. This proves mathematical correctness.

## Test Flow Diagram

```
DKG Scalar Shares: [s_0, s_1, s_2, s_3, s_4]
                            │
                    Select threshold (3)
                            │
            ┌───────────────┴───────────────┐
            │                               │
    Path A (Primary)               Path B (Verification)
            │                               │
            ▼                               ▼
  Compute DK shares:              Reconstruct master secret:
  dk_i = s_i × H(id)              s = Σ λ_i × s_i
  [dk_0, dk_1, dk_2]                        │
            │                               ▼
            ▼                       Derive DK from secret:
  Aggregate with Lagrange:          DK_B = s × H(id)
  DK_A = Σ λ_i × dk_i                       │
            │                               │
            └───────────────┬───────────────┘
                            │
                    Assert: DK_A == DK_B  ✓
                            │
                            ▼
                Decrypt(DK_A, ciphertext)
                            │
                            ▼
                Assert: plaintext matches  ✓
```

## What's Already Implemented

The following infrastructure exists and works:

1. **IBE primitives** in `crates/aptos-dkg/src/ibe/mod.rs`:
   - `hash_to_g1(&identity)` - Hashes identity to G1 point
   - `derive_decryption_key(&scalar, &identity)` - Derives DK from scalar and identity
   - `ibe_encrypt(&mpk, &identity, plaintext, &mut rng)` - Encrypts plaintext
   - `ibe_decrypt(&dk, &ciphertext)` - Decrypts ciphertext

2. **Scalar ElGamal PVSS** in `crates/aptos-dkg/src/pvss/scalar_elgamal/`:
   - `WeightedTranscript::deal()` - Creates PVSS transcript with scalar shares
   - `transcript.decrypt_own_share()` - Each validator decrypts their scalar share

3. **Lagrange coefficients** in `crates/aptos-dkg/src/algebra/lagrange.rs`:
   - `lagrange_coefficients(batch_size, &indices, &point)` - Computes Lagrange coefficients

4. **Test utilities** in `crates/aptos-dkg/src/pvss/test_utils.rs`:
   - `setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng)` - Sets up DKG dealing

## The Pattern You Need to Implement

### Current Pattern (Already Tested)
```rust
// Reconstruct scalar secret first
let s = Σ λ_i × s_i  // Lagrange interpolation on scalars
let DK = s × H(identity)  // Derive single DK
let plaintext = decrypt(DK, ciphertext)  // Works ✅
```

### Required Pattern (Not Yet Tested)
```rust
// Each validator computes their DK share
let dk_share_i = s_i × H(identity)  // G1 point for each validator

// Aggregate DK shares using Lagrange coefficients
let DK = Σ λ_i × dk_share_i  // Lagrange interpolation on G1 points

// Decrypt with aggregated DK
let plaintext = decrypt(DK, ciphertext)  // Must work ✅
```

## Mathematical Equivalence

These two patterns should be mathematically equivalent due to bilinear properties:
- Pattern 1: `DK = (Σ λ_i × s_i) × H(identity)`
- Pattern 2: `DK = Σ (λ_i × (s_i × H(identity)))`

By distributive property: Both equal the same G1 point.

**Your test must prove this equivalence holds in practice.**

## Implementation Steps

### Step 1: Create the Test Function

Add this test to `crates/aptos-dkg/src/ibe/tests.rs`:

```rust
#[test]
fn test_dk_share_aggregation_roundtrip() {
    use crate::algebra::lagrange::lagrange_coefficients;
    use crate::pvss::input_secret::InputSecret;
    use crate::pvss::scalar_elgamal::WeightedTranscript;
    use crate::pvss::test_utils::setup_dealing;
    use crate::pvss::traits::Transcript as TranscriptTrait;
    use crate::pvss::{Player, WeightedConfig};
    use blstrs::{G1Projective, Scalar};
    use group::Group;
    use rand::thread_rng;

    let mut rng = thread_rng();

    // TODO: Implement steps 2-8 below
}
```

### Step 2: Setup DKG with Threshold Configuration

Create a weighted config with 5 validators and threshold 3:

```rust
let weights = vec![1, 1, 1, 1, 1];
let wconfig = WeightedConfig::new(3, weights).unwrap();
let dealing_args = setup_dealing::<WeightedTranscript, _>(&wconfig, &mut rng);
```

### Step 3: Deal a Secret and Create Transcript

```rust
let input_secret = InputSecret::generate(&mut rng);
let secret = *input_secret.get_secret_a();  // Save for verification

let transcript = WeightedTranscript::deal(
    &wconfig,
    &dealing_args.pp,
    &dealing_args.ssks[0],
    &dealing_args.eks,
    &input_secret,
    &vec![0u8],
    &Player { id: 0 },
    &mut rng,
);
```

### Step 4: Each Validator Decrypts Their Scalar Share

```rust
let mut scalar_shares = Vec::new();
for i in 0..5 {
    let (sk_share, _pk_share) = transcript
        .decrypt_own_share(&wconfig, &Player { id: i }, &dealing_args.dks[i], &dealing_args.pp)
        .expect("decrypt_own_share should succeed");
    scalar_shares.push((i, sk_share.s));  // Extract scalar from share
}
```

### Step 5: Create Identity and Hash to G1

```rust
let identity = compute_identity(12345, 1000000000);
let h_identity = hash_to_g1(&identity);  // G1 point
```

### Step 6: Each Validator Computes Their DK Share (G1 Point)

```rust
let dk_shares_g1: Vec<G1Projective> = scalar_shares
    .iter()
    .map(|(_idx, scalar_share)| {
        // dk_share_i = s_i × H(identity)
        h_identity.mul(scalar_share)
    })
    .collect();
```

### Step 7: Compute Lagrange Coefficients and Aggregate DK Shares

```rust
// Use first 3 validators (threshold = 3)
let participating_indices = vec![0, 1, 2];
let base_coeffs = lagrange_coefficients(3, &participating_indices, &Scalar::ZERO);

// Apply weights to Lagrange coefficients
let total_weight = 5u64;
let lagrange_coeffs: Vec<Scalar> = base_coeffs
    .iter()
    .enumerate()
    .map(|(i, base_coeff)| {
        let validator_weight = weights[participating_indices[i]];
        *base_coeff * Scalar::from(validator_weight) / Scalar::from(total_weight)
    })
    .collect();

// Aggregate DK shares: DK = Σ λ_i × dk_share_i
let mut aggregated_dk = G1Projective::identity();
for i in 0..3 {
    let validator_idx = participating_indices[i];
    let weighted_share = dk_shares_g1[validator_idx].mul(&lagrange_coeffs[i]);
    aggregated_dk += weighted_share;
}

let aggregated_dk = aggregated_dk.to_affine();
```

### Step 8: Test Encryption/Decryption with Aggregated DK

```rust
// Create MPK from original secret
let mpk = G2Projective::generator().mul(&secret).to_affine();

// Encrypt a message
let plaintext = b"Test message for DK share aggregation";
let ciphertext = ibe_encrypt(&mpk, &identity, plaintext, &mut rng);

// Decrypt with aggregated DK
let decrypted = ibe_decrypt(&aggregated_dk, &ciphertext);

// Verify roundtrip
assert_eq!(
    decrypted, plaintext,
    "Decryption with aggregated DK shares should recover plaintext"
);
```

### Step 9: Reconstruct Master Secret and Verify Equivalence

**IMPORTANT**: This step is the key validation that proves the DK share aggregation is correct.

Reconstruct the master secret scalar using Lagrange interpolation, then verify that both methods produce identical decryption keys:

```rust
// Reconstruct master secret scalar: s = Σ λ_i × s_i
let reconstructed_master_secret: Scalar = scalar_shares
    .iter()
    .take(3)
    .enumerate()
    .map(|(i, (_idx, s))| lagrange_coeffs[i] * s)
    .sum();

// Verify reconstructed secret matches original dealt secret
assert_eq!(
    reconstructed_master_secret, secret,
    "Reconstructed master secret should match original dealt secret"
);

// Derive DK using the traditional method (scalar-first approach)
let dk_from_master_secret = derive_decryption_key(&reconstructed_master_secret, &identity);

// CRITICAL ASSERTION: Both methods must produce the same DK
assert_eq!(
    aggregated_dk, dk_from_master_secret,
    "DK from G1 share aggregation MUST equal DK from master secret scalar"
);

println!("✅ Verification passed: Both methods produce identical decryption keys");
```

**Why This Matters**: This proves that `Σ λ_i × (s_i × H(id)) = (Σ λ_i × s_i) × H(id)`, validating the mathematical equivalence of both approaches.

## Expected Behavior

When complete, the test should:

1. ✅ Pass all assertions
2. ✅ **Reconstruct master secret scalar and verify correctness** (matches original dealt secret)
3. ✅ **Prove mathematical equivalence**: DK from G1 share aggregation equals DK from master secret
4. ✅ Successfully decrypt ciphertext using the aggregated DK
5. ✅ Work with any threshold number of validators (test uses 3 out of 5)

**Primary Goal**: Validate that the distributed DK share aggregation pattern works correctly.

**Secondary Goal**: Use master secret reconstruction as a verification oracle to prove correctness.

## Location

Add the test to: `crates/aptos-dkg/src/ibe/tests.rs`

After the existing `test_scalar_elgamal_pvss_ibe_multiple_identities()` function (around line 420).

## How to Run

```bash
cd crates/aptos-dkg
cargo test test_dk_share_aggregation_roundtrip -- --nocapture
```

## Success Criteria

The test passes when all of these hold:

1. **Master secret reconstruction works**: `reconstructed_master_secret == original_dealt_secret`
2. **Mathematical equivalence verified**: `aggregated_dk == dk_from_master_secret`
3. **Decryption succeeds**: `decrypted_plaintext == original_plaintext`
4. **No panics or assertion failures**
5. **Performance**: Test runs in < 1 second

**The most important assertion** is #2 - proving that aggregating G1 DK shares produces the same result as deriving DK from the master secret. This validates the correctness of the distributed pattern.

## Common Pitfalls to Avoid

1. **Incorrect Lagrange point**: Use `Scalar::ZERO` as the evaluation point for reconstruction
2. **Missing weight adjustment**: Lagrange coefficients must be weighted: `λ_i = w_i × base_coeff_i / total_weight`
3. **Wrong share indexing**: Ensure you use the correct validator indices when selecting shares
4. **Group element type mismatch**: DK shares are `G1Projective`, convert to `G1Affine` before decrypt
5. **Scalar multiplication order**: For G1 points, use `.mul(&scalar)` not `scalar * point`

## Reference Files

- Existing similar test: `crates/aptos-dkg/src/ibe/tests.rs:267-345` (reconstructs scalar first)
- IBE module: `crates/aptos-dkg/src/ibe/mod.rs`
- Lagrange functions: `crates/aptos-dkg/src/algebra/lagrange.rs`
- WeightedConfig: `crates/aptos-dkg/src/pvss/weighted/weighted_config.rs`

## Questions?

If you encounter issues:
1. Check that all imports are correct
2. Verify `WeightedTranscript` trait implementations
3. Ensure `hash_to_g1()` and `derive_decryption_key()` are in scope
4. Look at the existing `test_scalar_elgamal_pvss_ibe_roundtrip()` for patterns to follow
