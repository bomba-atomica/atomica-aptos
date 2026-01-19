# ADR-001: Dual-Output DKG for IBE and Randomness

## Status

**Accepted**

## Executive Summary

This ADR defines the dual-output DKG architecture for Atomica:

| Output            | PVSS Scheme            | Use Case        | Key Type       |
| ----------------- | ---------------------- | --------------- | -------------- |
| **G1 shares**     | DAS PVSS               | WVUF/Randomness | `G1Projective` |
| **Scalar shares** | Chunked Lifted ElGamal | IBE/Timelock    | `Scalar`       |

**Key Design Principles:**

1. **Chunked Lifted ElGamal for IBE** - We use Chunked Lifted ElGamal PVSS to produce scalar shares that can be reconstructed into a master scalar for IBE decryption key derivation.

2. **DAS PVSS for Randomness only** - The existing DAS PVSS continues to produce G1 shares exclusively for WVUF/randomness. IBE does NOT use DAS PVSS output.

3. **DKG produces ephemeral IBE keys** - We do NOT use validator BLS keys directly for IBE. The DKG produces fresh ephemeral key material each epoch. This provides forward secrecy and clean key separation.

4. **Same InputSecret, two representations** - Both PVSS schemes share the same underlying scalar secret `a`, producing the same MPK (`g2^a`), but different share formats optimized for their consumers.

---

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

| PVSS Variant       | Output Type                     | Consumer                    |
| ------------------ | ------------------------------- | --------------------------- |
| **das**            | `DealtSecretKey = G1Projective` | WVUF (randomness)           |
| **scalar_elgamal** | `DealtSecretKey = Scalar`       | IBE, scalar-based protocols |

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

### Option D: Dual-Output DKG (Selected)

Extend RealDKG to produce both G1 and Scalar shares in a single round using two PVSS protocols with the same InputSecret.

**Selected:** Meets all constraints. Uses existing primitives. Single DKG round with bundled transcripts.

---

## Decision

Extend RealDKG to produce **two types of key material** in a single DKG round:

1. **G1 shares** (existing DAS PVSS) → for WVUF/randomness (unchanged)
2. **Scalar shares** (new Chunked Lifted ElGamal PVSS) → for IBE

### Architecture

```
                              ┌─────────────────────────────────────────────────────┐
                              │                    RealDKG                          │
                              │                                                     │
                              │  InputSecret (scalar a)                             │
                              │         │                                           │
                              │         ├───────────────┬───────────────────────┐   │
                              │         │               │                       │   │
                              │         ▼               ▼                       │   │
                              │   ┌──────────┐   ┌─────────────────────┐        │   │
                              │   │ DAS PVSS │   │ Chunked Lifted      │        │   │
                              │   │          │   │ ElGamal PVSS        │        │   │
                              │   └────┬─────┘   └──────────┬──────────┘        │   │
                              │        │                    │                   │   │
                              │        ▼                    ▼                   │   │
                              │   G1 shares            Scalar shares            │   │
                              │   (for WVUF)           (for IBE)                │   │
                              │                                                     │
                              │   MPK = g2^a  (same for both)                       │
                              └─────────────────────────────────────────────────────┘
                                       │                    │
                                       ▼                    ▼
                              ┌────────────────┐   ┌────────────────────────┐
                              │ WVUF/Randomness│   │ IBE Decryption Key     │
                              │ (pairing-based)│   │ Derivation             │
                              │                │   │                        │
                              │ e(G1_share, h) │   │ dk = H(identity)^scalar│
                              └────────────────┘   └────────────────────────┘
```

**Important:** The DKG produces fresh ephemeral key material each epoch. Validator BLS signing keys are used only for authenticating DKG messages (Signatures of Knowledge), NOT as the IBE master secret. This provides:

- **Forward secrecy**: Compromising current epoch keys doesn't reveal past decryptions
- **Key separation**: IBE keys are independent from consensus signing keys
- **Clean rotation**: New IBE master key each epoch via DKG

Both transcripts are dealt in a single round and bundled in a single message.

### Chunked Lifted ElGamal Design

The scalar ElGamal PVSS uses **Chunked Lifted ElGamal encryption** to produce scalar outputs:

#### Secret Sharing

The input scalar `s` is shared using Shamir's Secret Sharing to get shares `sh_i` for each validator.

#### Chunking

Each share `sh_i` (32 bytes) is split into **16 chunks of 16 bits**:

- `u_{i,0} ... u_{i,15}`
- Each chunk is in range `[0, 2^16)`

#### Encryption (Lifted ElGamal)

Each chunk `u_{i,j}` is encrypted as:

```
C_{i,j} = g^{u_{i,j}} · pk_i^{r_j}
```

- `r_j` is a random scalar shared across all validators for chunk index `j`
- Ephemeral keys `R_j = g^{r_j}` are included in the transcript (one per chunk index)

#### Decryption

Validator `i` recovers their share by:

1. Computing `pk_i^{r_j} = R_j^{sk_i}` using their secret key
2. Computing `g^{u_{i,j}} = C_{i,j} / pk_i^{r_j}`
3. Solving the discrete log: `u_{i,j} = log_g(g^{u_{i,j}})`

Since chunks are only 16 bits, the discrete log is computed efficiently using **Baby-step Giant-step (BSGS)** with a precomputed lookup table (~650 entries).

#### Reconstruction

Chunks are concatenated and converted back to a scalar `sh_i`.

### Transcript Structure

```rust
/// Top-level DKG output containing both PVSS transcripts
pub struct Transcripts {
    pub main: WTrx,              // DAS PVSS → G1 shares (for WVUF/randomness)
    pub fast: Option<WTrx>,      // DAS PVSS → G1 fast-path (for randomness)
    pub scalar: Option<ScalarTrx>, // Chunked ElGamal → Scalar shares (for IBE)
}

/// Chunked Lifted ElGamal PVSS transcript (scalar_elgamal module)
/// This produces SCALAR shares for IBE - NOT G1 shares like DAS PVSS
pub struct Transcript {
    /// Signatures of Knowledge proving dealer knows the secret
    pub soks: Vec<SoK<G2Projective>>,
    /// Commitment to first randomness coefficient r_0
    pub hat_w: G2Projective,
    /// Polynomial commitments V_i = G2^{f(i)} for verification
    pub V: Vec<G2Projective>,
    /// Ephemeral keys R_j = g^{r_j} for each chunk index j=0..15
    /// (16 elements total, one per chunk - NOT one per validator)
    pub ephemeral_keys: Vec<G1Projective>,
    /// Ciphertexts C_{i,j} for each validator i and chunk j
    /// Outer: validators (n), Inner: chunks (16)
    pub ciphertexts: Vec<Vec<G1Projective>>,
    /// Pre-computed aggregate for threshold reconstruction
    pub encrypted_aggregate: Vec<G1Projective>,
}
```

**Note on naming:** The `scalar` transcript produces `DealtSecretKey { s: Scalar }` which is a raw scalar value. This is fundamentally different from DAS PVSS which produces `DealtSecretKey(G1Projective)`. The scalar output is what IBE needs for `dk = H(identity)^s`.

### Key Properties

1. **Same InputSecret** - Both transcripts share the same underlying scalar `a`
2. **Same MPK** - Both produce identical public key `g2^a`
3. **Battle-tested primitives** - Shamir, ElGamal, BSGS discrete log
4. **Efficient decryption** - 16-bit chunks enable fast BSGS lookup

### Verification Strategy: DLEQ + Linear Pairing Check

The scalar ElGamal PVSS uses a **two-layer verification strategy** to ensure encryption correctness:

#### Layer 1: DLEQ Proofs (Per-Dealer Verification)

For single-dealer transcripts, we use **DLEQ (Discrete Log Equality) proofs** to verify that each dealer used the same randomness `r_j` in both:

- The ephemeral key `R_j = G^{r_j}`
- The ciphertext `C_{i,j} = G·u_{i,j} + PK_i·r_j`

The proof guarantees: `log_G(R_j) = log_{PK_i}(C_{i,j} - G·u_{i,j})`

**Limitation:** DLEQ proofs are per-dealer and cannot be aggregated. After multiple dealers contribute to the same DKG session, the DLEQ proofs are no longer valid for the summed ciphertexts.

#### Layer 2: Linear Multi-Pairing Check (Aggregated Verification)

To verify encryption correctness on **aggregated transcripts**, we need a linear check that works on summed values. The upstream Aptos chunky PVSS uses this approach:

```rust
// Upstream pattern (chunky/transcript.rs:303-313)
let res = E::multi_pairing(
    [weighted_Cs, h],
    [g2, (-weighted_Vs)],
);
if res != Gt::identity() {
    bail!("Expected zero during multi-pairing check");
}
```

**For our scalar ElGamal, the linear check verifies:**

Given the encryption equation: `C_{i,j} = G·u_{i,j} + PK_i·r_j`

And the polynomial commitment: `V_i = G2^{f(i)}`

The pairing equation holds for summed values:

```
e(Σ_i α_i·C_{i,j}, G2) = e(G, Σ_i α_i·V_i) + e(Σ_i α_i·PK_i, R_j)
```

This equation is **linear** and therefore holds when we sum across multiple dealers.

#### Verification Flow

```
DKG Session:
┌─────────────────────────────────────────────────────────────────────────────────┐
│ 1. Dealer broadcasts transcript with DLEQ proofs                                │
│                                                                                  │
│ 2. Receiver verifies PER-DEALER (soks.len() == 1):                              │
│    ✓ SoK verification (dealer knew secret)                                      │
│    ✓ Low-degree test (polynomial degree < t)                                    │
│    ✓ DLEQ proofs (encryption correctness) ← Per-(i,j) pair                      │
│                                                                                  │
│ 3. If valid → AGGREGATE (ciphertexts, V, R all add linearly)                    │
│                                                                                  │
│ 4. Aggregated transcript verification (soks.len() > 1):                         │
│    ✓ SoK verification (all dealers knew their secrets)                          │
│    ✓ Low-degree test (all polynomials valid)                                    │
│    ✓ Linear pairing check (encryption correctness for sum) ← NEW                │
│                                                                                  │
│ 5. On-chain: Final verification (same as step 4)                                │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Why Both Layers Matter

| Property           | DLEQ (Per-Dealer)               | Linear Check (Aggregated)        |
| ------------------ | ------------------------------- | -------------------------------- |
| **Scope**          | Individual ciphertexts          | Summed ciphertexts               |
| **When**           | Before aggregation              | After aggregation                |
| **Granularity**    | Per (validator, chunk)          | Global per-chunk                 |
| **What it proves** | Each dealer encrypted correctly | Overall encryption is consistent |

**Security Implication:** Without the linear pairing check, an attacker who controls a dealer could:

1. Submit a valid-looking transcript (passes DLEQ for their contribution)
2. But the overall aggregated transcript has incorrect ciphertexts
3. Validators would decrypt wrong values, potentially breaking reconstruction

The linear pairing check prevents this by verifying the **aggregate** encryption equation.

#### Implementation Reference

- **Upstream Implementation:** `~/atomica-aptos-upstream-main/crates/aptos-dkg/src/pvss/chunky/transcript.rs:303-313`
- **Our DLEQ Implementation:** `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs:1173-1197`
- **Implementation Plan:** [implementation-plan-unified-dkg-ibe.md](implementation-plan-unified-dkg-ibe.md) (Phase 5.1)

---

## Consequences

### Positive

- WVUF/randomness completely unchanged
- IBE works with standard Boneh-Franklin construction
- Single DKG service, single consensus round
- No novel cryptography
- MPK is shared (same secret) - no additional on-chain state
- Efficient decryption via small chunk size (16 bits)

### Negative

- Increased transcript size (ephemeral keys + ciphertexts)
- Additional dealing/verification computation per DKG round
- New code to maintain (scalar_elgamal PVSS module)

### Neutral

- Existing `main`/`fast` naming preserved for G1 transcripts
- New `scalar` field added to Transcripts struct

---

## Implementation

Integrate as a new phase in [implementation-plan-unified-dkg-ibe.md](./implementation-plan-unified-dkg-ibe.md).

### Summary

| Phase | Description                                                     |
| ----- | --------------------------------------------------------------- |
| 1     | Create `scalar_elgamal` PVSS module with Chunked Lifted ElGamal |
| 2     | Add weighted threshold wrapper                                  |
| 3     | Integrate into RealDKG                                          |
| 4     | Wire up IBE consumer                                            |

### New Files

- `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs`
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs`
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs`
- `crates/aptos-dkg/src/pvss/dealt_secret_key.rs` (scalar module)
- `crates/aptos-dkg/src/pvss/dealt_secret_key_share.rs` (scalar module)

### Modified Files

- `types/src/dkg/real_dkg/mod.rs` - Add ScalarTrx, extend Transcripts

---

## References

- PVSS Library: `crates/aptos-dkg/src/pvss/`
- DAS Protocol: `crates/aptos-dkg/src/pvss/das/`
- Scalar ElGamal PVSS: `crates/aptos-dkg/src/pvss/scalar_elgamal/`
- DealtSecretKey (Scalar): `crates/aptos-dkg/src/pvss/dealt_secret_key.rs`
- IBE Module: `crates/aptos-dkg/src/ibe/mod.rs`
- WVUF: `crates/aptos-dkg/src/weighted_vuf/pinkas/mod.rs`
- RealDKG: `types/src/dkg/real_dkg/mod.rs`
