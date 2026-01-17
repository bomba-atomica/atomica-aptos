# IBE-DKG Scalar Mismatch Problem

## Executive Summary

The IBE (Identity-Based Encryption) module and the DKG (Distributed Key Generation) module have incompatible secret representations, preventing the end-to-end encryption/decryption flow from working.

**Problem:** IBE expects a `Scalar` (~32 bytes), but DKG produces a G1 projective element (~48 bytes).

---

## Current Architecture

### IBE Module (`aptos-dkg/src/ibe/mod.rs`)

The IBE scheme is Boneh-Franklin style:

```rust
pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine {
    let h = hash_to_g1(identity);
    h.mul(secret).to_affine()  // Scalar multiplication
}
```

**IBE Key Types:**

- Master secret: `Scalar` (32 bytes) - field element
- Master public key (MPK): `G2Affine` (96 bytes) - `g2^s`
- Decryption key: `G1Affine` (48 bytes) - `H(id)^s`

### DKG Output (`crates/aptos-dkg/src/pvss/dealt_secret_key.rs`)

```rust
pub struct DealtSecretKey {
    h_hat: G1Projective,  // G1 element, NOT a scalar
}

impl DealtSecretKey {
    pub fn as_group_element(&self) -> &G1Projective {
        &self.h_hat
    }
}
```

**DKG Output Types:**

- Dealt secret: `G1Projective` (48 bytes) - group element
- Dealt public key: `G2Projective` (96 bytes) - paired with secret

### WVUF Usage (Randomness)

The Pinkas WVUF (Weighted Verifiable Unpredictable Function) uses `DealtSecretKey`:

```rust
// pinkas/mod.rs:185-188
fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
    let h = Self::hash_to_curve(msg).to_affine();
    pairing(&sk.as_group_element().to_affine(), &h)  // Expects G1 element!
}
```

**The WVUF expects `DealtSecretKey` to be a G1 element for pairing operations.**

---

## The Mismatch

```
IBE expects:    Scalar (32 bytes) ──► dk = H(id)^s
DKG outputs:    G1Projective (48 bytes) ──► used in pairing
```

| Module                        | Input Type            | Operation                          |
| ----------------------------- | --------------------- | ---------------------------------- |
| IBE `derive_decryption_key()` | `Scalar`              | Scalar multiplication: `H(id) * s` |
| WVUF `eval()`                 | `DealtSecretKey` (G1) | Pairing: `e(secret, h)`            |

---

## Impact

### Current State

- ✅ MPK storage on-chain works (`mpk_on_chain` test passes)
- ✅ DKG produces valid G1 element
- ✅ WVUF randomness generation works
- ❌ IBE encryption/decryption blocked (type mismatch)

### Required for Full Encryption

1. Reconstruct `DealtSecretKey` from validator shares
2. Convert to scalar for IBE `derive_decryption_key()`
3. Encrypt with on-chain MPK
4. Decrypt with derived key

Step 2 fails because there's no canonical conversion from G1 element to scalar.

---

## Solution Options

### Option A: Modify DKG to Output Scalar

**Change the DKG architecture to produce a scalar secret.**

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
2. `InputSecret::to()` conversion to produce scalar
3. PVSS dealing to work with scalar secrets
4. `reconstruct_secret_from_shares()` to return scalar
5. **WVUF `eval()` refactor** - currently expects G1 element

**Risk to Randomness:** 🔴 **HIGH**

- WVUF `eval()` calls `sk.as_group_element()` - would break
- Requires refactoring both DKG and WVUF
- Risk of introducing bugs in production randomness system

---

### Option B: Modify IBE to Accept G1 Element

**Change IBE to work with G1 elements directly.**

```rust
// Before:
pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine

// After:
pub fn derive_decryption_key_from_g1(secret: &G1Projective, identity: &[u8]) -> G1Affine
```

**Approach:** Use a different cryptographic construction:

- Instead of `dk = H(id)^s`, use `dk = e(H(id), mpk)^x` where `x` is derived from the G1
- Or: Use the G1 element as-is in a modified pairing-based derivation

**Risk to Randomness:** 🟢 **LOW**

- DKG unchanged
- WVUF continues working
- Only IBE module changes

**Downside:** Changes the mathematical properties of the IBE scheme; requires security analysis.

---

### Option C: Hash G1 Element to Scalar

**Deterministically convert G1 element to scalar.**

```rust
fn g1_element_to_scalar(g1: &G1Projective) -> Scalar {
    let mut hasher = Sha3_256::default();
    hasher.update(b"APTOS_IBE_SECRET_DERIVATION");
    hasher.update(g1.to_compressed());
    reduce_to_scalar(&hasher.finalize())
}

fn derive_decryption_key(secret: &G1Projective, identity: &[u8]) -> G1Affine {
    let scalar = g1_element_to_scalar(secret);
    let h = hash_to_g1(identity);
    h.mul(&scalar).to_affine()
}
```

**Risk to Randomness:** 🟢 **LOW**

- DKG unchanged
- WVUF continues working

**Downside:**

- Changes cryptographic construction
- Security impact unclear - need formal analysis
- **Must also change MPK computation:** The on-chain MPK is `g2^s`. If we derive `s = hash(g1_secret)`, the MPK must be recomputed as `g2^hash(g1_secret.

---

## Security Considerations)` to match

### For Option B or C

The IBE security relies on the Bilinear Diffie-Hellman (BDH) assumption:

> Given `g`, `g^a`, `g^b`, `g^c` in G1/G2, computing `e(g, g)^{abc}` is hard.

Changing to hash-based derivation or G1-based derivation may alter these security properties. Need to verify:

1. The derived scalar remains secret (not computable from public values)
2. The IBE scheme remains IND-ID-CCA secure
3. No new attacks via the G1→scalar conversion

---

## Files Involved

| File                                                       | Change Needed          | Risk   |
| ---------------------------------------------------------- | ---------------------- | ------ |
| `crates/aptos-dkg/src/pvss/dealt_secret_key.rs`            | Option A: Change type  | High   |
| `crates/aptos-dkg/src/ibe/mod.rs`                          | Option B/C: Accept G1  | Low    |
| `aptos-vm/src/validator_txns/dkg.rs`                       | MPK computation update | Medium |
| `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` | Update test            | Low    |

---

## Recommended Path Forward

1. **Short-term:** Keep current approach (Option C simplified) for testing
   - Document the architectural limitation
   - Mark `mpk_encrypt_decrypt` as BLOCKED

2. **Long-term:** Evaluate Option B
   - Modify IBE to accept G1 element
   - Use pairing-based key derivation
   - Requires security review

3. **Avoid Option A** - too risky for production randomness

---

## Test Status

| Test                            | Status     |
| ------------------------------- | ---------- |
| `randomness::e2e_correctness`   | ✅ PASSING |
| `timelock::mpk_on_chain`        | ✅ PASSING |
| `timelock::mpk_encrypt_decrypt` | 🔲 BLOCKED |

---

## References

- Boneh-Franklin IBE: [CRYPTO 2001](https://crypto.stanford.edu/~dabo/papers/bfibe.pdf)
- Pinkas VUF: `aptos-dkg/src/weighted_vuf/pinkas/mod.rs`
- DKG Transcript: `types/src/dkg/real_dkg/mod.rs`
- IBE Module: `aptos-dkg/src/ibe/mod.rs`
