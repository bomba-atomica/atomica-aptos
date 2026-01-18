# IBE/DKG Comparison: Same PVSS, Different Output Types

## Key Correction

Both Atomica and upstream use **PVSS** (Publicly Verifiable Secret Sharing). The difference is the **PVSS variant** and its **output type**:

```
PVSS (Shared Library: crates/aptos-dkg/src/pvss/)
    │
    ├─── das (Atomica uses this)
    │     Output: DealtSecretKey = G1Projective
    │     Used for: Randomness (WVUF)
    │
    └─── chunky (Upstream uses this)
          Output: DealtSecretKey = Scalar
          Used for: FPTX/BIBE (batch encryption)
```

---

## The Real Difference

### DAS (Atomica) - G1 Element Output

```rust
// crates/aptos-dkg/src/pvss/das/unweighted_protocol.rs
type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;

// crates/aptos-dkg/src/pvss/dealt_secret_key.rs
pub struct DealtSecretKey {
    h_hat: G1Projective,  // Group element, ~48 bytes
}
```

### Chunky (Upstream) - Scalar Output

```rust
// crates/aptos-dkg/src/pvss/chunky/keys.rs
pub type DealtSecretKey<F: PrimeField> = Scalar<F>;  // Field element, ~32 bytes
```

---

## Why Different Output Types?

| Use Case                         | Output Type | Reason                                     |
| -------------------------------- | ----------- | ------------------------------------------ |
| **WVUF (Randomness)**            | G1 element  | Pinkas VUF needs G1 for `e(sk, h)` pairing |
| **FPTX/BIBE (Batch Encryption)** | Scalar      | KZG polynomial commitments need scalars    |

**Both are valid PVSS instantiations** - just different cryptographic constructions for different purposes.

---

## Code Evidence

```bash
# Atomica uses DAS for randomness
$ grep -r "type.*SecretKey.*=" crates/aptos-dkg/src/pvss/das/
type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;  # G1!

# Upstream uses chunky for FPTX
$ grep -r "type.*SecretKey.*=" crates/aptos-dkg/src/pvss/chunky/
pub type DealtSecretKey<F: PrimeField> = Scalar<F>;  # Scalar!
```

---

## Our Problem in Context

```
Atomica:
  DAS PVSS → DealtSecretKey(G1) → WVUF ✓ (works)
                                    └──► IBE ✗ (expects Scalar)

Upstream:
  Chunky PVSS → DealtSecretKey(Scalar) → FPTX/BIBE ✓ (works)
                                         └──► IBE ✓ (works)
```

**The mismatch is between DAS's G1 output and our IBE's scalar requirement.**

---

## Solution Paths

| Option | Description                     | Risk to Randomness           |
| ------ | ------------------------------- | ---------------------------- |
| **A**  | Modify IBE to accept G1 element | 🟢 None                      |
| **B**  | Switch to chunky PVSS           | 🔴 High (breaks WVUF)        |
| **C**  | Adopt upstream's FPTX/BIBE      | 🟡 Medium (different scheme) |

---

## Summary Table

| Aspect              | Atomica (DAS)   | Upstream (Chunky)        |
| ------------------- | --------------- | ------------------------ |
| **PVSS Variant**    | das             | chunky                   |
| **Output Type**     | G1 element      | Scalar                   |
| **Bytes**           | 48              | 32                       |
| **WVUF Compatible** | ✅ Yes          | ❌ No (different scheme) |
| **IBE Compatible**  | ❌ No (our IBE) | ✅ Yes (FPTX)            |
| **Batching**        | ❌ No           | ✅ Yes (KZG)             |

---

## Conclusion

Both use the same `aptos-dkg` PVSS library. The difference is:

- **DAS** → G1 output → Good for WVUF, bad for our IBE
- **Chunky** → Scalar output → Good for FPTX/BIBE, bad for WVUF

Our IBE expects scalar, but DAS gives G1. **The fix is to change IBE, not DKG.**

---
