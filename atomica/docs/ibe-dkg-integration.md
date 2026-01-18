# IBE-DKG Integration: Complete Analysis & Decision

## Executive Summary

The IBE (Identity-Based Encryption) module and DKG (Distributed Key Generation) have incompatible secret representations:

| Component          | Output Type  | Size     |
| ------------------ | ------------ | -------- |
| **DKG (DAS PVSS)** | G1Projective | 48 bytes |
| **IBE**            | Scalar       | 32 bytes |

**Root Cause:** Both Atomica and upstream use the same PVSS library, but different PVSS variants:

- **DAS** → G1 output (for WVUF/randomness)
- **Chunky** → Scalar output (for FPTX/BIBE)

Our IBE expects a scalar, but DAS outputs a G1 element.

---

## Part 1: Architecture Analysis

### PVSS Variants in apts-dkg

```rust
// PVSS library: crates/aptos-dkg/src/pvss/
pub mod das;        // Atomica uses this → G1 output
pub mod chunky;     // Upstream uses this → Scalar output
```

| PVSS Variant | Output Type                     | Consumer                     |
| ------------ | ------------------------------- | ---------------------------- |
| **das**      | `DealtSecretKey = G1Projective` | WVUF (randomness)            |
| **chunky**   | `DealtSecretKey = Scalar`       | FPTX/BIBE (batch encryption) |

### Code References

```rust
// DAS (Atomica) - G1 output
// crates/aptos-dkg/src/pvss/das/unweighted_protocol.rs
type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;

// Chunky (Upstream) - Scalar output
// crates/aptos-dkg/src/pvss/chunky/keys.rs
pub type DealtSecretKey<F: PrimeField> = Scalar<F>;
```

### IBE Module Requirements

```rust
// crates/aptos-dkg/src/ibe/mod.rs
pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine {
    let h = hash_to_g1(identity);
    h.mul(secret).to_affine()  // Scalar multiplication
}
```

### WVUF Module Requirements

```rust
// crates/aptos-dkg/src/weighted_vuf/pinkas/mod.rs:185-188
fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
    let h = Self::hash_to_curve(msg).to_affine();
    pairing(&sk.as_group_element().to_affine(), &h)  // Expects G1!
}
```

---

## Part 2: The Mismatch

```
Atomica:
  DAS PVSS → DealtSecretKey(G1) → WVUF ✓ (works)
                                    └──► IBE ✗ (expects Scalar)

Upstream:
  Chunky PVSS → DealtSecretKey(Scalar) → FPTX/BIBE ✓ (works)
```

| Module                        | Input Type            | Operation                |
| ----------------------------- | --------------------- | ------------------------ |
| IBE `derive_decryption_key()` | `Scalar`              | Scalar mult: `H(id) * s` |
| WVUF `eval()`                 | `DealtSecretKey` (G1) | Pairing: `e(secret, h)`  |

---

## Part 3: Solution Options

### Option A: Modify DKG to Output Scalar

**Change DAS to produce scalar instead of G1.**

```rust
// Before:
pub struct DealtSecretKey {
    h_hat: G1Projective,  // ~48 bytes
}

// After:
pub struct DealtSecretKey {
    secret: Scalar,  // ~32 bytes
}
```

**Changes Required:**

1. `DealtSecretKey` type: `G1Projective` → `Scalar`
2. `InputSecret::to()` conversion
3. PVSS dealing logic
4. `reconstruct_secret_from_shares()`
5. **WVUF `eval()` refactor** - breaks!

**Risk:** 🔴 **HIGH**

- WVUF expects G1 element for pairing
- Would break production randomness system

---

### Option B: Modify IBE to Accept G1 Element

**Change IBE to work with G1 directly.**

```rust
// New function for G1-based derivation
pub fn derive_decryption_key_from_g1(
    secret: &G1Projective,
    identity: &[u8],
    mpk: &G2Affine
) -> G1Affine {
    // Use pairing-based derivation
    let h = hash_to_g1(identity);
    let pairing_result = pairing(secret, mpk);  // e(g1_secret, mpk)
    // Derive key from pairing result
    derive_key_from_pairing(&pairing_result, identity)
}
```

**Risk:** 🟢 **LOW**

- DKG unchanged
- WVUF continues working
- Only IBE module changes

**Downside:** Requires new cryptographic construction + security review

---

### Option C: Hash G1 to Scalar

**Deterministic conversion: G1 → Scalar → IBE**

```rust
fn g1_to_scalar(g1: &G1Projective) -> Scalar {
    let mut hasher = Sha3_256::default();
    hasher.update(b"APTOS_IBE_SECRET_V1");
    hasher.update(g1.to_compressed());
    reduce_to_scalar(&hasher.finalize())
}

fn derive_decryption_key(secret: &G1Projective, identity: &[u8]) -> G1Affine {
    let scalar = g1_to_scalar(secret);
    let h = hash_to_g1(identity);
    h.mul(&scalar).to_affine()
}
```

**Risk:** 🟢 **LOW**

- DKG unchanged
- WVUF continues working

**Downside:**

- Changes cryptographic properties
- Must recompute MPK as `g2^hash(g1_secret)`
- Needs security analysis

---

## Part 4: Upstream Comparison

### Upstream-Main Architecture

| Aspect                 | Atomica        | Upstream        |
| ---------------------- | -------------- | --------------- |
| **PVSS Variant**       | das            | chunky          |
| **DKG Output**         | G1 element     | Scalar          |
| **IBE Scheme**         | Boneh-Franklin | FPTX/BIBE + KZG |
| **Batching**           | ❌ No          | ✅ Yes          |
| **Weighted Threshold** | ❌ No          | ✅ Yes          |

### Key Insight

Upstream **doesn't have this problem** because:

1. Uses **chunky PVSS** which outputs scalars
2. Uses **FPTX/BIBE** which accepts scalars
3. **WVUF not used** (different randomness approach)

---

## Part 5: Decision & Recommendation

### Decision Matrix

| Criteria                 | Option A | Option B    | Option C  |
| ------------------------ | -------- | ----------- | --------- |
| Complexity               | High     | Medium      | Low       |
| Risk to Randomness       | 🔴 High  | 🟢 None     | 🟢 None   |
| Cryptographic Changes    | None     | Significant | Moderate  |
| Security Analysis Needed | WVUF     | IBE         | IBE + MPK |
| Future Alignment         | Poor     | Medium      | Poor      |

### Recommended Path

**Short-term (Current):** Option C

- Use hash-based conversion for testing
- Document limitation
- `mpk_encrypt_decrypt` = BLOCKED

**Long-term:** Option B

- Modify IBE to accept G1 element
- Use pairing-based key derivation
- Requires security review

**Avoid:** Option A

- Too risky for production randomness

---

## Part 6: Files Impacted

| File                                                       | Change                       | Risk   |
| ---------------------------------------------------------- | ---------------------------- | ------ |
| `crates/aptos-dkg/src/ibe/mod.rs`                          | Accept G1 or hash conversion | Low    |
| `crates/aptos-dkg/src/pvss/dealt_secret_key.rs`            | Option A: Change type        | High   |
| `aptos-vm/src/validator_txns/dkg.rs`                       | MPK computation              | Medium |
| `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` | Update test                  | Low    |

---

## Part 7: Test Status

| Test                            | Status     |
| ------------------------------- | ---------- |
| `randomness::e2e_correctness`   | ✅ PASSING |
| `timelock::mpk_on_chain`        | ✅ PASSING |
| `timelock::mpk_encrypt_decrypt` | 🔲 BLOCKED |

---

## Conclusion

The IBE-DKG mismatch is a **protocol design choice**, not a fundamental incompatibility:

- **DAS PVSS** → G1 output → Good for WVUF
- **IBE (Scalar)** → Expects scalar → Mismatch!

**Fix:** Change IBE to accept G1 element (Option B), avoiding risk to production randomness.

---

## References

- PVSS Library: `crates/aptos-dkg/src/pvss/`
- DAS Protocol: `crates/aptos-dkg/src/pvss/das/`
- Chunky Protocol: `crates/aptos-dkg/src/pvss/chunky/`
- IBE Module: `crates/aptos-dkg/src/ibe/mod.rs`
- WVUF: `crates/aptos-dkg/src/weighted_vuf/pinkas/mod.rs`
- Upstream FPTX: `crates/aptos-batch-encryption/src/schemes/`
