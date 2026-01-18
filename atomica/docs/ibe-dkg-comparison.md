# IBE/DKG Comparison: Same Underlying Crypto

## The Short Answer

Yes, both use BLS12-381. The **same underlying cryptography**, but **different protocol choices** for how the secret is structured and shared.

```
Both: BLS12-381 ──► DKG ──► Threshold Crypto
                    │
        ┌───────────┴───────────┐
        │                       │
   Atomica (DAS)          Upstream (PVSS)
   Output: G1            Output: Scalar
        │                       │
        ▼                       ▼
   Different IBE        FPTX/BIBE
   (Boneh-Franklin)     + KZG
```

---

## What's Actually Different

### 1. DKG Protocol Variant

| Protocol          | Our Atomica                            | Upstream                                  |
| ----------------- | -------------------------------------- | ----------------------------------------- |
| **Name**          | DAS (Distributed Aggregation Protocol) | PVSS (Publicly Verifiable Secret Sharing) |
| **Secret Output** | `DealtSecretKey` (G1 element)          | `MSK Share` (Scalar)                      |
| **Threshold**     | Weighted Shamir over players           | Weighted Shamir over stake                |

**Same cryptographic primitive:** BLS12-381 pairing-friendly curve

**Different encoding choice:**

- DAS: Secret = group element `h^a` in G1
- PVSS: Secret = scalar field element `s`

### 2. The Root Cause

```rust
// DAS Protocol (Atomica)
// Secret is an element of G1, not the scalar itself
pub struct DealtSecretKey {
    h_hat: G1Projective,  // h^a where h is generator, a is secret
}

// PVSS (Upstream)
// Secret is a scalar field element
pub struct SecretShare {
    value: Fr,  // The actual secret scalar s
}
```

**This is just a design choice**, not a fundamental requirement. The DAS protocol could be modified to output the scalar.

### 3. IBE Scheme Differences

| Aspect             | Our IBE (Boneh-Franklin)        | Upstream FPTX/BIBE                 |
| ------------------ | ------------------------------- | ---------------------------------- |
| **Key Derivation** | `dk = H(id)^s` (scalar mult)    | `dk = H(digest) * s` (scalar mult) |
| **Encryption**     | Pairing-based key encapsulation | KZG commitments over IDs           |
| **Batching**       | No                              | Yes (polynomial over IDs)          |
| **Ciphertext**     | 2 G2 elements                   | 3 G2 elements + KZG proof          |
| **Verification**   | Direct pairing check            | KZG opening proof                  |

**Same underlying math:**

- Both use `e(G1, G2)` pairing
- Both use scalar multiplication on G1
- Both use BLS12-381 field arithmetic

### 4. Why Upstream Chose Different

**Upstream's design goals:**

- Batch encryption for many recipients
- Weighted threshold (stake-based)
- Efficient proof aggregation

**Our design goals:**

- Simple timelock encryption
- Player-based threshold (1 validator = 1 share)
- Minimal protocol overhead

---

## Can We Bridge the Gap?

### The Scalar Connection

```
G1 element h^a  ←→  scalar a  (the "discrete log")
         │
         │  (This is what DAS hides)
         ▼
   "What's the secret?"

   DAS: "The secret IS h^a" (group element)
   PVSS: "The secret IS a" (scalar)
```

**Mathematically equivalent** - just different representation.

### If We Extract the Scalar from DAS

The DAS protocol generates `h^a` where `a` is the secret. If we could extract `a` from `h^a`:

```rust
// Hypothetical: Extract scalar from G1 element
fn g1_to_scalar(g1: &G1Projective) -> Scalar {
    // This is the "discrete log" - computationally hard!
    // Not feasible in practice.
}
```

**Problem:** You can't efficiently extract `a` from `h^a`. That's the point of discrete log cryptography.

### The Real Fix: Change DAS Protocol

To get a scalar, DAS needs to be modified at the protocol level:

```
Current DAS:
  Input: secret scalar s
  Output: h^s (G1 element)

Modified DAS:
  Input: secret scalar s
  Output: s (Scalar)
```

This requires changing how the secret is encoded throughout the protocol.

---

## Implications for Randomness

### Our WVUF Uses G1 Element

```rust
// pinkas/mod.rs:185-188
fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
    let h = Self::hash_to_curve(msg).to_affine();
    pairing(&sk.as_group_element().to_affine(), &h)  // Expects G1!
}
```

**WVUF needs the G1 element for pairing.** If we change DAS to output scalar, we break WVUF.

### Upstream's Approach

Upstream uses a **different VUF scheme** that works with scalar secrets:

```rust
// upstream: Scalar-based evaluation
fn eval(sk: &Scalar, msg: &[u8]) -> Gt {
    pairing(&G1Projective::generator().mul(sk), &hash_to_g2(msg))
}
```

---

## Summary Table

| Question            | Answer                                                       |
| ------------------- | ------------------------------------------------------------ |
| Same crypto?        | ✅ Yes, both BLS12-381                                       |
| Same DKG?           | ❌ Different protocols (DAS vs PVSS)                         |
| Same secret format? | ❌ G1 element vs scalar                                      |
| Can we bridge?      | ⚠️ Only with protocol changes                                |
| Risk to randomness? | ✅ Upstream's approach doesn't break WVUF (different scheme) |

---

## Conclusion

The core difference is **encoding**, not cryptography:

- **Atomica:** DKG secret = G1 element (for WVUF pairing)
- **Upstream:** DKG secret = scalar (for IBE + KZG)

**To fix our IBE-DKG mismatch:**

1. Option A: Modify DAS to output scalar (break WVUF)
2. Option B: Modify IBE to work with G1 element (cleaner)
3. Option C: Adopt upstream's FPTX/BIBE (most work, most features)

**Option B is the path of least disruption** for Atomica - keep G1 output, change IBE to accept it.
