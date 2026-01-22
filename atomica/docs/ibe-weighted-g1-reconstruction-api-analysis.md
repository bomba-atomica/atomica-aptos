# API Analysis: Using Existing aptos-dkg APIs for IBE Weighted G1 Reconstruction

## Executive Summary

**YES, higher-level APIs exist** - but they don't directly apply to the current IBE architecture where shares are pre-aggregated per validator.

The current implementation in `reconstruct_ibe_dk_from_g1_shares` already uses the correct approach (coefficient sum method). If tests are failing, the issue is likely in:
1. Threshold configuration
2. Virtual player ID mapping
3. Test setup (not the reconstruction math itself)

## Available Higher-Level APIs

### 1. `Reconstructable` Trait
**Location:** `crates/aptos-dkg/src/pvss/traits/mod.rs:95-99`

```rust
pub trait Reconstructable<SSC: SecretSharingConfig> {
    type Share: Clone;

    fn reconstruct(sc: &SSC, shares: &Vec<(Player, Self::Share)>) -> Self;
}
```

This trait provides a generic reconstruction interface that:
- Works with any secret sharing configuration (threshold or weighted)
- Automatically handles Lagrange coefficient computation
- Supports both scalar and group element reconstruction

### 2. Scalar Reconstruction (Already Implemented)
**Location:** `crates/aptos-dkg/src/pvss/scalar_secret_key.rs:15-41`

```rust
impl Reconstructable<ThresholdConfigBlstrs> for Scalar {
    type Share = Scalar;

    fn reconstruct(sc: &ThresholdConfigBlstrs, shares: &Vec<(Player, Self::Share)>) -> Self {
        // 1. Get player IDs from shares
        let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();

        // 2. Compute Lagrange coefficients at 0
        let lagr = lagrange_coefficients(
            sc.get_batch_evaluation_domain(),
            ids.as_slice(),
            &Scalar::ZERO,
        );

        // 3. Compute: Σ (share_i × lagr_i)
        shares
            .iter()
            .zip(lagr.iter())
            .map(|(&share, &lagr)| share * lagr)
            .sum::<Scalar>()
    }
}
```

### 3. G1 Point Reconstruction (Already Implemented!)
**Location:** `crates/aptos-dkg/src/pvss/dealt_secret_key.rs:83-120`

```rust
impl traits::Reconstructable<ThresholdConfigBlstrs> for DealtSecretKey {
    type Share = DealtSecretKeyShare;

    fn reconstruct(sc: &ThresholdConfigBlstrs, shares: &Vec<(Player, Self::Share)>) -> Self {
        // 1. Extract player IDs
        let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();

        // 2. Compute Lagrange coefficients
        let lagr = lagrange_coefficients(
            sc.get_batch_evaluation_domain(),
            ids.as_slice(),
            &Scalar::ZERO,
        );

        // 3. Extract G1 bases from shares
        let bases = shares
            .iter()
            .map(|(_, share)| share.0.h_hat)
            .collect::<Vec<G1Projective>>();

        // 4. Multi-exponentiation: Σ (lagr_i × base_i)
        DealtSecretKey {
            h_hat: g1_multi_exp(bases.as_slice(), lagr.as_slice()),
        }
    }
}
```

**Key Insight:** This already does G1 reconstruction using multi-exponentiation!

### 4. Weighted Reconstruction (Generic Wrapper)
**Location:** `crates/aptos-dkg/src/pvss/weighted/generic_weighting.rs:27-54`

```rust
impl<SK: Reconstructable<ThresholdConfigBlstrs>> Reconstructable<WeightedConfig> for SK {
    type Share = Vec<SK::Share>;  // Vector of shares per validator

    fn reconstruct(sc: &WeightedConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
        // 1. Flatten virtual player shares
        let mut flattened_shares = Vec::with_capacity(sc.get_total_weight());

        for (player, sub_shares) in shares {
            for (pos, share) in sub_shares.iter().enumerate() {
                let virtual_player = sc.get_virtual_player(player, pos);
                flattened_shares.push((virtual_player, share.clone()));
            }
        }

        // 2. Delegate to unweighted reconstruction
        SK::reconstruct(sc.get_threshold_config(), &flattened_shares)
    }
}
```

**This provides weighted reconstruction for FREE!** Any type that implements `Reconstructable<ThresholdConfigBlstrs>` automatically gets `Reconstructable<WeightedConfig>`.

## Why Can't We Use These Directly?

### The Architectural Mismatch

**What the APIs expect:**
```rust
// Per-virtual-player shares
Vec<(Player, Vec<G1Projective>)>
// Example for validator 0 with weight 2:
// (Player{id: 0}, vec![share_0_0, share_0_1])
```

**What IBE provides:**
```rust
// Pre-aggregated shares per validator
Vec<(Player, G1Projective)>
// Example for validator 0 with weight 2:
// (Player{id: 0}, share_0_0 + share_0_1)  // Already summed!
```

### Why IBE Pre-Aggregates

In the IBE flow, each validator computes:
```rust
// For validator i with weight w_i:
let aggregated_scalar = Σ s_{i,j}  for j in 0..w_i
let dk_share = H(identity) × aggregated_scalar
```

This aggregation happens **before** the shares leave the validator, so:
1. Only one G1 point needs to be stored/transmitted per validator (not w_i points)
2. More efficient bandwidth/storage
3. The scalar aggregation is secure (no MSK exposure)

### Could We Change IBE to Use the Generic APIs?

**Option A: Store Per-Virtual-Player G1 Shares**

```rust
// Instead of aggregating scalars first:
// OLD: (Σ s_{i,j}) × H(id)

// Compute individual DK shares:
// NEW: For each j: s_{i,j} × H(id)

pub fn compute_dk_shares_per_virtual_player(
    scalar_shares: Vec<Scalar>,  // One per virtual player
    identity: &[u8],
) -> Vec<G1Projective> {
    let h_id = hash_to_g1(identity);
    scalar_shares
        .iter()
        .map(|s| G1Projective::from(h_id) * s)
        .collect()
}
```

Then use the generic reconstruction:
```rust
use crate::pvss::dealt_secret_key::g1::DealtSecretKey;
use crate::pvss::traits::Reconstructable;

pub fn reconstruct_using_generic_api(
    shares: &Vec<(Player, Vec<G1Projective>)>,
    wconfig: &WeightedConfig,
) -> G1Affine {
    let dealt_sk_shares: Vec<(Player, Vec<DealtSecretKeyShare>)> = shares
        .iter()
        .map(|(player, g1_shares)| {
            let dk_shares = g1_shares
                .iter()
                .map(|&g1| DealtSecretKeyShare::new(DealtSecretKey::new(g1)))
                .collect();
            (*player, dk_shares)
        })
        .collect();

    let dealt_sk = DealtSecretKey::reconstruct(wconfig, &dealt_sk_shares);
    dealt_sk.as_group_element().to_affine()
}
```

**Trade-offs:**
- ✅ Uses battle-tested DKG reconstruction code
- ✅ Simpler, less custom crypto logic
- ✅ Automatic correctness (if DKG works, this works)
- ❌ More storage (w_i G1 points per validator instead of 1)
- ❌ More bandwidth for transmission
- ❌ API changes required (native bridge, Move contracts)

## Current Implementation Analysis

The current `reconstruct_ibe_dk_from_g1_shares` implementation (lines 704-814) is **mathematically sound**:

```rust
// 1. Create weighted config
let wconfig = WeightedConfig::new(1, weights_usize)?;

// 2. Get all participating virtual player IDs
let mut participating_virtual_player_ids = Vec::new();
for &validator_idx in validator_indices {
    let player = wconfig.get_player(validator_idx as usize);
    let weight = wconfig.get_player_weight(&player);
    let starting_index = wconfig.get_player_starting_index(&player);
    for j in 0..weight {
        participating_virtual_player_ids.push(starting_index + j);
    }
}

// 3. Compute Lagrange coefficients for all virtual players
let lagrange_coeffs = lagrange_coefficients(
    wconfig.get_batch_evaluation_domain(),
    &participating_virtual_player_ids,
    &Scalar::from(0u64),
);

// 4. Sum coefficients per validator and multiply by aggregated share
let mut reconstructed = G1Projective::identity();
let mut coeff_idx = 0;

for (i, &validator_idx) in validator_indices.iter().enumerate() {
    let g1_affine = G1Affine::from_compressed(&dk_shares[i])?;
    let player = wconfig.get_player(validator_idx as usize);
    let weight = wconfig.get_player_weight(&player);

    // Sum Lagrange coefficients for this validator's virtual players
    let mut validator_coeff_sum = Scalar::ZERO;
    for _ in 0..weight {
        validator_coeff_sum += lagrange_coeffs[coeff_idx];
        coeff_idx += 1;
    }

    // Accumulate
    reconstructed += G1Projective::from(g1_affine) * validator_coeff_sum;
}
```

This is the **coefficient sum approach** - mathematically equivalent to what the generic APIs do, but adapted for pre-aggregated shares.

## Root Cause of Failures

If tests are failing, investigate:

### 1. Threshold Configuration
```rust
// CURRENT (Line 744):
let wconfig = WeightedConfig::new(1, weights_usize)?;
//                                 ^^^^ Threshold = 1

// Should it be:
let threshold = validator_indices.len();
let wconfig = WeightedConfig::new(threshold, weights_usize)?;
```

**Why this matters:** The `WeightedConfig` uses the threshold to determine:
- The degree of the polynomial
- The evaluation domain size
- Which roots of unity to use for virtual player positions

If the threshold is wrong, the virtual player mappings will be incorrect.

### 2. Virtual Player ID Mapping

The current code assumes validator indices are **contiguous** (0, 1, 2, ...). For **sparse** indices (e.g., [0, 2, 4]), the virtual player ID computation may be wrong:

```rust
// CURRENT: Assumes validators 0, 1, 2, ...
for &validator_idx in validator_indices {
    let player = wconfig.get_player(validator_idx as usize);
    // ...
}

// ISSUE: If validator_indices = [0, 2], but wconfig was created for 3 validators,
// then virtual player IDs won't align correctly.
```

### 3. Weight Array Alignment

```rust
pub fn reconstruct_ibe_dk_from_g1_shares(
    validator_indices: &[u64],      // [0, 2]
    dk_shares: &[Vec<u8>],          // 2 shares
    weights: &[u64],                // Should be [w0, w2] or [w0, w1, w2, w3]?
    total_weight: u64,
) -> Result<G1Affine, IbeError>
```

**Question:** Does `weights[i]` correspond to `validator_indices[i]` or to validator ID `i`?

## Recommendations

### Short-term: Fix Current Implementation

1. **Verify threshold configuration:**
   ```rust
   // The threshold should equal the total weight being used for reconstruction
   let threshold = weights.iter().sum::<u64>() as usize;
   let wconfig = WeightedConfig::new(threshold, weights_usize)?;
   ```

2. **Clarify weight array semantics:**
   ```rust
   // Option A: weights aligned with validator_indices
   assert_eq!(validator_indices.len(), weights.len());

   // Option B: weights for ALL validators (use full array)
   let participating_weights: Vec<u64> = validator_indices
       .iter()
       .map(|&idx| all_weights[idx as usize])
       .collect();
   ```

3. **Add comprehensive logging:**
   ```rust
   eprintln!("Threshold: {}", threshold);
   eprintln!("Weights: {:?}", weights);
   eprintln!("Virtual player IDs: {:?}", participating_virtual_player_ids);
   eprintln!("Lagrange coeffs: {:?}", lagrange_coeffs);
   ```

### Long-term: Consider Generic APIs

If the storage/bandwidth overhead is acceptable:
1. Modify IBE share computation to produce per-virtual-player G1 shares
2. Use `DealtSecretKey::g1::reconstruct` directly via the `Reconstructable<WeightedConfig>` impl
3. Remove custom reconstruction logic entirely

This would reduce code duplication and leverage battle-tested DKG primitives.

## Utility Functions Available

### From `crates/aptos-dkg/src/algebra/lagrange.rs`
```rust
pub fn lagrange_coefficients(
    domain: &GeneralEvaluationDomain<Scalar>,
    points: &[usize],
    at: &Scalar,
) -> Vec<Scalar>
```
- Already used by current implementation ✅

### From `crates/aptos-dkg/src/utils/`
```rust
pub fn g1_multi_exp(bases: &[G1Projective], scalars: &[Scalar]) -> G1Projective
```
- More efficient than manual loop (Pippenger algorithm)
- Could replace the current accumulation loop

### From `WeightedConfig`
```rust
impl WeightedConfig {
    pub fn get_virtual_player(&self, player: &Player, pos: usize) -> Player
    pub fn get_player_weight(&self, player: &Player) -> usize
    pub fn get_player_starting_index(&self, player: &Player) -> usize
    pub fn get_batch_evaluation_domain(&self) -> &GeneralEvaluationDomain<Scalar>
}
```
- All used correctly ✅

## Conclusion

**The current implementation already uses the right approach.** The math is correct (coefficient sum method). The issue is likely:

1. **Threshold mismatch** - Line 744 uses threshold=1 instead of the actual threshold
2. **Weight array alignment** - Unclear if weights correspond to indices or validator IDs
3. **Test setup** - The test may be passing incorrect parameters

**Recommendation:** Debug the threshold configuration and weight array semantics before considering a rewrite. The existing higher-level APIs don't directly apply without changing the IBE share format.
