# IBE Weighted G1 Reconstruction: Summary & Action Items

## TL;DR

**YES, higher-level APIs exist in aptos-dkg** - but they expect per-virtual-player shares, while IBE uses pre-aggregated shares. The current implementation is mathematically correct but likely has a **threshold configuration bug** at line 744.

## Existing APIs You Can Leverage

### 1. `Reconstructable<WeightedConfig>` Trait
**Location:** `crates/aptos-dkg/src/pvss/weighted/generic_weighting.rs:27-54`

Any type implementing `Reconstructable<ThresholdConfigBlstrs>` automatically gets weighted reconstruction:
- ✅ `Scalar` - already implemented
- ✅ `DealtSecretKey::g1` - already implemented for G1 points!
- ✅ Handles virtual player flattening automatically
- ✅ Computes Lagrange coefficients correctly

### 2. G1 Multi-Exponentiation
**Location:** `crates/aptos-dkg/src/utils/`

```rust
pub fn g1_multi_exp(bases: &[G1Projective], scalars: &[Scalar]) -> G1Projective
```

More efficient than manual accumulation loop (uses Pippenger algorithm).

## Why You Can't Use Them Directly

**API Mismatch:**
```rust
// What aptos-dkg APIs expect:
Vec<(Player, Vec<G1Projective>)>  // Per-virtual-player shares

// What IBE provides:
Vec<(Player, G1Projective)>  // Pre-aggregated shares
```

**IBE aggregates before sharing:**
```rust
// Each validator computes:
dk_share = (Σ s_{i,j}) × H(identity)  // Sum scalars first
```

This is intentional (saves storage/bandwidth), but prevents using the generic APIs.

## Two Paths Forward

### Path A: Quick Fix (Recommended)

**Fix the threshold bug in the current implementation.**

**Location:** `crates/aptos-dkg/src/ibe/mod.rs:744`

**Current (WRONG):**
```rust
let wconfig = WeightedConfig::new(1, weights_usize)
//                                 ^ Threshold = 1 is incorrect!
```

**Should be:**
```rust
// The threshold should match the polynomial degree
// For weighted PVSS, this is the sum of participating weights
let threshold = weights.iter().sum::<usize>();
let wconfig = WeightedConfig::new(threshold, weights_usize)?;
```

**Why this matters:**
- The threshold determines the polynomial degree
- This affects the evaluation domain (which roots of unity to use)
- Wrong threshold → wrong virtual player positions → wrong Lagrange coefficients

**Other potential issues:**

1. **Weight array semantics** - Clarify whether `weights[i]` is:
   - The weight of `validator_indices[i]`, OR
   - The weight of validator with ID `i`

2. **Sparse indices handling** - If validator_indices = [0, 2, 5]:
   - Need to map correctly to virtual players
   - Current code may assume contiguous indices

### Path B: Use Generic APIs (Long-term)

**Change IBE to produce per-virtual-player shares.**

**Benefits:**
- Leverage battle-tested DKG reconstruction
- Less custom crypto code
- Automatic correctness

**Costs:**
- ❌ More storage (w_i G1 points per validator instead of 1)
- ❌ More bandwidth
- ❌ API changes in native bridge and Move contracts

**Implementation:**
```rust
// In IBE share computation:
pub fn compute_dk_shares(
    scalar_shares: &[Vec<Scalar>],  // Per-virtual-player
    identity: &[u8],
) -> Vec<Vec<G1Projective>> {
    let h_id = hash_to_g1(identity);
    scalar_shares
        .iter()
        .map(|validator_shares| {
            validator_shares
                .iter()
                .map(|s| G1Projective::from(h_id) * s)
                .collect()
        })
        .collect()
}

// In reconstruction:
use crate::pvss::dealt_secret_key::g1::DealtSecretKey;
use crate::pvss::dealt_secret_key_share::g1::DealtSecretKeyShare;
use crate::pvss::traits::Reconstructable;

pub fn reconstruct_dk(
    shares: Vec<(Player, Vec<G1Projective>)>,
    wconfig: &WeightedConfig,
) -> G1Affine {
    let dealt_shares: Vec<(Player, Vec<DealtSecretKeyShare>)> = shares
        .into_iter()
        .map(|(player, g1_shares)| {
            let dk_shares = g1_shares
                .into_iter()
                .map(|g1| {
                    DealtSecretKeyShare::new(DealtSecretKey::new(g1))
                })
                .collect();
            (player, dk_shares)
        })
        .collect();

    let result = DealtSecretKey::reconstruct(wconfig, &dealt_shares);
    result.as_group_element().to_affine()
}
```

## Immediate Action Items

### 1. Debug Current Implementation

**Add logging to understand what's happening:**

```rust
// In reconstruct_ibe_dk_from_g1_shares:
eprintln!("=== Reconstruction Debug ===");
eprintln!("Validator indices: {:?}", validator_indices);
eprintln!("Weights: {:?}", weights);
eprintln!("Total weight: {}", total_weight);
eprintln!("Threshold: {}", wconfig.get_threshold());
eprintln!("Virtual player IDs: {:?}", participating_virtual_player_ids);
eprintln!("Lagrange coeffs[0..5]: {:?}", &lagrange_coeffs[0..lagrange_coeffs.len().min(5)]);
eprintln!("Coeff sums per validator:");
for (i, &validator_idx) in validator_indices.iter().enumerate() {
    eprintln!("  Validator {}: coeff_sum = {:?}", validator_idx, validator_coeff_sum);
}
```

**Run the failing tests:**
```bash
cargo test -p aptos-dkg test_compare_scalar_and_g1_reconstruction -- --nocapture
```

### 2. Fix Threshold Configuration

**Try this fix:**
```rust
// Line 744 - Change from:
let wconfig = WeightedConfig::new(1, weights_usize)?;

// To:
let threshold = weights.iter().sum::<usize>();
let wconfig = WeightedConfig::new(threshold, weights_usize)?;
```

### 3. Verify Coefficient Sum Property

**Add this assertion:**
```rust
// After computing all coefficient sums
let total_coeff_sum: Scalar = /* sum of all validator_coeff_sums */;
assert!(
    (total_coeff_sum - Scalar::ONE).is_zero(),
    "Lagrange coefficients must sum to 1, got: {:?}",
    total_coeff_sum
);
```

This is a fundamental property of Lagrange interpolation and must hold.

### 4. Compare with Scalar Reconstruction

The `test_compare_scalar_and_g1_reconstruction` is perfect for debugging:
- It uses the same shares for both methods
- Scalar reconstruction works (confirmed by existing tests)
- G1 reconstruction fails
- Compare intermediate values to find where they diverge

### 5. Check Move Test Fixtures

**Failing fixtures:**
- Fixture 215 (3 validators, weights: 1, 2, 3)
- Fixture 2321 (4 validators, weights: 2, 3, 2, 1)

**Run them individually:**
```bash
cd aptos-move/framework/aptos-framework
aptos move test --filter test_ibe_golden_vector_215
aptos move test --filter test_ibe_golden_vector_2321
```

## Reference: How DKG Does It

**Scalar reconstruction** (`scalar_secret_key.rs:18-40`):
```rust
let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();
let lagr = lagrange_coefficients(domain, &ids, &Scalar::ZERO);
shares.iter().zip(lagr.iter())
    .map(|(&share, &lagr)| share * lagr)
    .sum::<Scalar>()
```

**G1 reconstruction** (`dealt_secret_key.rs:88-119`):
```rust
let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();
let lagr = lagrange_coefficients(domain, &ids, &Scalar::ZERO);
let bases = shares.iter().map(|(_, share)| share.0.h_hat).collect();
DealtSecretKey { h_hat: g1_multi_exp(&bases, &lagr) }
```

**Weighted wrapper** (`generic_weighting.rs:30-52`):
```rust
// Flatten virtual player shares
for (player, sub_shares) in shares {
    for (pos, share) in sub_shares.iter().enumerate() {
        let virtual_player = sc.get_virtual_player(player, pos);
        flattened_shares.push((virtual_player, share.clone()));
    }
}
// Delegate to unweighted reconstruction
SK::reconstruct(sc.get_threshold_config(), &flattened_shares)
```

**Your implementation does this manually for pre-aggregated shares.**

## Key Files Reference

```
crates/aptos-dkg/src/
├── ibe/
│   ├── mod.rs:704-814           # reconstruct_ibe_dk_from_g1_shares (FIX LINE 744)
│   └── tests.rs:797-891         # test_compare_scalar_and_g1_reconstruction
├── pvss/
│   ├── weighted/
│   │   └── generic_weighting.rs:27-54   # Reconstructable<WeightedConfig>
│   ├── dealt_secret_key.rs:83-120      # G1 reconstruction example
│   ├── scalar_secret_key.rs:15-41      # Scalar reconstruction example
│   └── traits/mod.rs:95-99             # Reconstructable trait
└── algebra/
    └── lagrange.rs                      # lagrange_coefficients

aptos-move/framework/aptos-framework/tests/
└── ibe_native_test.move                # Move integration tests
```

## Questions to Answer

1. **What is the correct threshold?**
   - Sum of participating weights?
   - Number of participating validators?
   - Total weight of all validators?

2. **How should sparse indices work?**
   - If we have 5 validators but only [0, 2, 4] participate
   - Does `weights` contain 3 or 5 elements?

3. **Should we switch to per-virtual-player shares?**
   - Trade-off: correctness/simplicity vs. efficiency
   - What's the typical validator weight? (1-10 or 100-1000?)

## Next Steps

1. ✅ **Try threshold fix first** - Most likely culprit
2. ✅ **Add debug logging** - Understand what values are being computed
3. ✅ **Run comparison test** - See where scalar vs G1 diverge
4. ❓ **Consider API refactor** - If efficiency isn't critical, use generic APIs
5. ❓ **Update documentation** - Once working, document the correct approach

## Conclusion

You **DO** have higher-level APIs available:
- `Reconstructable<WeightedConfig>` trait
- `DealtSecretKey::g1` reconstruction
- `g1_multi_exp` utility

But they don't directly fit the pre-aggregated share model.

**Recommended approach:** Fix the threshold bug, then decide if the efficiency gain of pre-aggregated shares is worth maintaining custom reconstruction logic.
