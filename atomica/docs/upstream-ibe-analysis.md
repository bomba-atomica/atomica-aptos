# Upstream-Main IBE Analysis: Implications for Atomica

## Executive Summary

Aptos upstream-main uses a **completely different IBE scheme** (FPTX/BIBE with KZG commitments) that avoids our scalar mismatch problem. Their approach uses scalar MSK shares from DKG, while our DKG produces G1 elements.

---

## Upstream IBE Architecture (FPTX/BIBE)

### Key Components

| Component   | Type          | Description                                          |
| ----------- | ------------- | ---------------------------------------------------- |
| `MSK`       | `Fr` (scalar) | Master secret key (arkworks BLS12-381 field element) |
| `MPK`       | `G2Affine`    | Master public key: `g^MSK`                           |
| `MSK Share` | `Fr` (scalar) | Shamir share of MSK                                  |
| `DK`        | `G1Affine`    | Decryption key: reconstructed G1 element             |
| `EK`        | (G2, G2)      | Encryption key: (MPK, τ·g) where τ is KZG setup      |

### Key Derivation Flow (Upstream)

```
1. DKG → PVSS Transcript (encrypted scalar shares)
2. Validator extracts MSK share (scalar) from transcript via decrypt_priv_key
3. MSK share → DK share:  dk_share = H(digest) * msk_share
4. Threshold reconstruction → Full DK (G1 element)
5. Encryption uses MPK (G2), Decryption uses DK (G1)
```

### Code Reference

```rust
// Master secret is a scalar
pub struct BIBEMasterSecretKeyShare {
    mpk_g2: G2Affine,           // MPK (G2)
    player: Player,
    shamir_share_eval: Fr,      // ← MSK share is a SCALAR
}

// Decryption key is G1
pub struct BIBEDecryptionKey {
    signature_g1: G1Affine,     // ← Reconstructed from G1 shares
}

// Integration with DKG
let msk_share: Fr = subtranscript
    .decrypt_own_share(...)
    .0
    .into_fr();  // ← Extracts as scalar!
```

---

## Why Upstream Avoids Our Problem

| Aspect                    | Our Atomica                   | Upstream Main          |
| ------------------------- | ----------------------------- | ---------------------- |
| \*\*DKG Output            | `DealtSecretKey` (G1 element) | `MSK Share` (scalar)   |
| **Secret Type**           | G1 projective                 | Scalar `Fr`            |
| **DKG-PVSS Type**         | DAS protocol                  | Different PVSS variant |
| **Decryption Key Source** | Direct from DKG               | Shamir reconstruction  |
| **IBE Scheme**            | Boneh-Franklin                | FPTX/BIBE + KZG        |

**Root Cause:**

- Our DKG (DAS protocol) outputs `DealtSecretKey` as a G1 element
- Upstream's PVSS outputs scalar shares that get combined via Shamir

---

## Our Problem in Context

```
Atomica (Current):
  DKG (DAS) → DealtSecretKey(G1) ──┐
                                    ├──► Type mismatch! (G1 vs Scalar)
  IBE requires → Scalar for dk = H(id)^s ─┘

Upstream (Main):
  DKG (PVSS) → MSK Share(Fr) ────────┐
                                      ├──► Compatible! (both scalars)
  IBE requires → Scalar for dk = ... ─┘
```

---

## Analysis: Can We Adopt Upstream's Approach?

### Option 1: Switch to Upstream's FPTX/BIBE

**Pros:**

- ✅ Proven integration with DKG
- ✅ Scalar MSK shares (no mismatch)
- ✅ Additional features (batching, KZG proofs, weighted threshold)

**Cons:**

- ❌ Completely different cryptographic scheme
- ❌ Requires KZG polynomial commitment setup
- ❌ Larger ciphertexts (3 G2 elements vs 2)
- ❌ More complex implementation
- ❌ May not fit our timelock use case

### Option 2: Modify DKG to Output Scalar

**Pros:**

- ✅ Keep our IBE scheme
- ✅ Align with upstream's approach

**Cons:**

- ❌ WVUF `eval()` expects G1 element (breaks randomness)
- ❌ Large refactor of DKG/PVSS
- ❌ Changes core DAS protocol

### Option 3: Keep Current Path (Hash G1→Scalar)

**Pros:**

- ✅ Minimal changes
- ✅ Works for our use case

**Cons:**

- ❌ Cryptographic properties unclear
- ❌ MPK must be recomputed as `g2^hash(g1_secret)`

---

## Recommendation for Atomica

Given that **Atomica is a separate project with different goals** than upstream:

### Short Term (Current Implementation)

1. Keep the hash-based G1→scalar conversion for testing
2. Document the limitation
3. Mark `mpk_encrypt_decrypt` as experimental

### Long Term (Future Consideration)

**If Atomica converges with upstream:**

- Adopt FPTX/BIBE for production
- Benefits: batch encryption, weighted threshold, proven DKG integration

**If Atomica diverges (custom timelock):**

- Consider modifying DKG to output scalar
- Requires WVUF refactor or alternative

---

## Key Differences Summary

| Feature         | Our IBE                  | Upstream FPTX      |
| --------------- | ------------------------ | ------------------ |
| Scheme          | Boneh-Franklin           | FPTX/BIBE          |
| Master Secret   | G1 element (problematic) | Scalar (clean)     |
| Batching        | No                       | Yes (KZG)          |
| Threshold       | Protocol-level           | Native (Shamir)    |
| Weighted        | No                       | Yes (FPTXWeighted) |
| DKG Integration | G1 element               | Scalar shares      |
| Ciphertext      | 2 G2 elements            | 3 G2 elements      |

---

## Files to Reference

| Upstream File                                                | Purpose                        |
| ------------------------------------------------------------ | ------------------------------ |
| `crates/aptos-batch-encryption/src/shared/key_derivation.rs` | MSK handling                   |
| `crates/aptos-batch-encryption/src/schemes/fptx.rs`          | FPTX scheme                    |
| `crates/aptos-batch-encryption/src/traits.rs`                | BatchThresholdEncryption trait |
| `types/src/secret_sharing.rs`                                | Type definitions               |

---

## Conclusion

The upstream-main branch demonstrates that **scalar-based DKG outputs are the clean solution** for IBE integration. Our G1-based DKG output is the root cause of the mismatch.

**For Atomica:**

- If we want to align with upstream: adopt FPTX/BIBE or modify DKG to output scalar
- If we want to stay independent: keep current approach with hash-based conversion (documented limitation)

The most principled fix is **Option A: Modify DKG to output scalar**, accepting that this may eventually require WVUF changes or using a different WVUF scheme.
