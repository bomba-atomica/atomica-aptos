# ADR-001: Dual-Output DKG for IBE and Randomness

## Status

**Accepted**

## Context

### Problem

The IBE (Identity-Based Encryption) module and DKG (Distributed Key Generation) have incompatible secret representations:

| Component          | Output Type  | Size     |
| ------------------ | ------------ | -------- |
| **DKG (DAS PVSS)** | G1Projective | 48 bytes |
| **IBE**            | Scalar       | 32 bytes |

The existing DKG uses DAS PVSS which outputs G1 elements for use with WVUF (randomness). Standard Boneh-Franklin IBE requires scalar secrets for key derivation:

```
Current:
  DAS PVSS → DealtSecretKey(G1) → WVUF ✓ (works)
                                 → IBE ✗ (expects Scalar)
```

### PVSS Variants

The aptos-dkg library contains multiple PVSS variants:

| PVSS Variant | Output Type                     | Consumer                     |
| ------------ | ------------------------------- | ---------------------------- |
| **das**      | `DealtSecretKey = G1Projective` | WVUF (randomness)            |
| **chunky**   | `DealtSecretKey = Scalar`       | FPTX/BIBE (batch encryption) |

### Consumer Requirements

| Module                        | Input Type            | Operation                |
| ----------------------------- | --------------------- | ------------------------ |
| IBE `derive_decryption_key()` | `Scalar`              | Scalar mult: `H(id) * s` |
| WVUF `eval()`                 | `DealtSecretKey` (G1) | Pairing: `e(secret, h)`  |

### Constraints

1. **Must not break randomness** - WVUF/randomness is production-critical
2. **No novel cryptography** - Use battle-tested primitives only
3. **Single DKG service** - Reuse existing consensus infrastructure
4. **Bundled messages** - No additional network round-trips

---

## Options Considered

### Option A: Modify DKG to Output Scalar Only

Change DAS to produce scalar instead of G1.

**Rejected:** Would break WVUF which requires G1 for pairing operations. High risk to production randomness.

### Option B: Modify IBE to Accept G1 Element

Create new cryptographic construction for G1-based IBE.

**Rejected:** Requires novel cryptography and security review. Violates constraint #2.

### Option C: Hash G1 to Scalar

Deterministic conversion: `scalar = hash(G1)` then use scalar for IBE.

**Rejected:** Breaks relationship between MPK and derived keys. Would require separate MPK for IBE, complicating on-chain state.

### Option D: Switch to Chunky PVSS

Replace DAS with Chunky PVSS which outputs scalars.

**Rejected:** Chunky lacks weighted threshold support. Would require significant development to add weighted support.

### Option E: Dual-Output DKG (Selected)

Extend RealDKG to produce both G1 and Scalar shares in a single round using two PVSS protocols with the same InputSecret.

**Selected:** Meets all constraints. Uses existing primitives. Single DKG round with bundled transcripts.

---

## Decision

Extend RealDKG to produce **two types of key material** in a single DKG round:

1. **G1 shares** (existing DAS PVSS) → for WVUF/randomness (unchanged)
2. **Scalar shares** (new ElGamal PVSS) → for IBE

### Architecture

```
RealDKG → DAS PVSS      → G1 shares     → WVUF, randomness
        → ElGamal PVSS  → Scalar shares → IBE, scalar-based protocols

(Both dealt in single round, bundled in single message)
```

### Transcript Structure

```rust
pub struct Transcripts {
    pub main: WTrx,              // DAS → G1 (existing)
    pub fast: Option<WTrx>,      // DAS → G1 fast-path (existing)
    pub scalar: ScalarTrx,       // ElGamal → Scalar (new)
}
```

### Key Properties

1. **Same InputSecret** - Both transcripts share the same underlying scalar `a`
2. **Same MPK** - Both produce identical public key `g2^a`
3. **Battle-tested primitives** - Shamir, ElGamal, DLEQ proofs

---

## Consequences

### Positive

- WVUF/randomness completely unchanged
- IBE works with standard Boneh-Franklin construction
- Single DKG service, single consensus round
- No novel cryptography
- MPK is shared (same secret) - no additional on-chain state

### Negative

- Increased transcript size (~32 bytes per validator for scalar shares + DLEQ proofs)
- Additional dealing/verification computation per DKG round
- New code to maintain (scalar_elgamal PVSS module)

### Neutral

- Existing `main`/`fast` naming preserved for G1 transcripts
- New `scalar` field added to Transcripts struct

---

## Implementation

Integrate as a new phase in [implementation-plan-unified-dkg-ibe.md](./implementation-plan-unified-dkg-ibe.md).

### Summary

| Phase | Description |
|-------|-------------|
| 1 | Create `scalar_elgamal` PVSS module |
| 2 | Add weighted threshold wrapper |
| 3 | Integrate into RealDKG |
| 4 | Wire up IBE consumer |

### New Files

- `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs`
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs`

### Modified Files

- `types/src/dkg/real_dkg/mod.rs` - Add ScalarTrx, extend Transcripts

---

## References

- PVSS Library: `crates/aptos-dkg/src/pvss/`
- DAS Protocol: `crates/aptos-dkg/src/pvss/das/`
- ElGamal Encryption: `crates/aptos-dkg/src/pvss/encryption_elgamal.rs`
- IBE Module: `crates/aptos-dkg/src/ibe/mod.rs`
- WVUF: `crates/aptos-dkg/src/weighted_vuf/pinkas/mod.rs`
- RealDKG: `types/src/dkg/real_dkg/mod.rs`
