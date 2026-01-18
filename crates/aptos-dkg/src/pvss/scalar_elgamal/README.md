# Scalar ElGamal PVSS Module

## Overview

This module implements a **Publicly Verifiable Secret Sharing (PVSS)** scheme that outputs
**scalar shares** (as opposed to the DAS PVSS which outputs G1 group element shares).

This is part of the **Dual-Output DKG** architecture defined in
[ADR-001: Dual-Output DKG](../../../../atomica/docs/adr-001-dual-output-dkg.md).

## Purpose

The Aptos DKG currently uses DAS PVSS which outputs G1 elements. This works well for
WVUF (randomness) but is incompatible with standard IBE (Identity-Based Encryption)
schemes like Boneh-Franklin, which require scalar secrets.

This module provides a second PVSS transcript type that:
- Uses the **same InputSecret** as DAS (a scalar `a`)
- Outputs **scalar shares** that can be used for IBE
- Produces the **same MPK** as DAS (`g2^a`)

## Architecture

```
                    ┌─────────────────────────────────────┐
                    │           SINGLE DKG ROUND          │
                    │       (Same InputSecret scalar)     │
                    └─────────────────┬───────────────────┘
                                      │
                    ┌─────────────────┴───────────────────┐
                    │                                     │
                    ▼                                     ▼
          ┌─────────────────┐                   ┌─────────────────┐
          │   DAS PVSS      │                   │ Scalar ElGamal  │
          │   (existing)    │                   │   PVSS (new)    │
          │                 │                   │                 │
          │ Output: G1      │                   │ Output: Scalar  │
          │ → WVUF, random  │                   │ → IBE, timelock │
          └─────────────────┘                   └─────────────────┘
```

## Key Types

| Type | Description |
|------|-------------|
| `Transcript` | Unweighted PVSS transcript with ElGamal-encrypted scalar shares |
| `WeightedTranscript` | Weighted wrapper using `GenericWeighting` |
| `DealtSecretKey` | The reconstructed scalar (type alias to `Scalar`) |
| `DealtSecretKeyShare` | A single validator's scalar share |

## Cryptographic Primitives

### Sharing
- Uses **Shamir Secret Sharing** over the scalar field
- Polynomial `f(x)` where `f(0) = secret` (the InputSecret scalar)
- Each validator `i` gets share `f(i)`

### Encryption
- Uses **ElGamal encryption** in G1 to encrypt each share
- Ciphertext: `(C1, C2) = (g1^r, ek^r + h^share)` where:
  - `g1` is the generator
  - `h` is the message base
  - `ek` is the validator's encryption public key
  - `r` is random

### Verification
- Uses **DLEQ (Discrete Log Equality) proofs** to prove ciphertext consistency
- Commitments: `V[i] = g2^f(i)` allow verification without revealing shares

### Decryption
- Validator decrypts their share: `share = (C2 - dk*C1) / h`
- Where `dk` is the validator's decryption private key

## Integration with RealDKG

The `Transcripts` struct in `types/src/dkg/real_dkg/mod.rs` will include:

```rust
pub struct Transcripts {
    pub main: WTrx,              // DAS → G1 (existing, for WVUF)
    pub fast: Option<WTrx>,      // DAS → G1 fast-path (existing)
    pub scalar: ScalarTrx,       // ElGamal → Scalar (new, for IBE)
}
```

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module exports and documentation |
| `transcript.rs` | Core unweighted PVSS protocol implementation |
| `weighted_protocol.rs` | Weighted threshold wrapper |

## Security Properties

1. **Secrecy**: Shares are ElGamal-encrypted; only the designated validator can decrypt
2. **Verifiability**: DLEQ proofs ensure shares are consistent with commitments
3. **Threshold**: Requires `t` shares to reconstruct (configurable threshold)
4. **Binding**: Commitments bind the dealer to specific shares

## Implementation Status

- [ ] `mod.rs` - Module structure
- [ ] `transcript.rs` - Core PVSS protocol
- [ ] `weighted_protocol.rs` - Weighted wrapper
- [ ] Unit tests
- [ ] Integration tests

## References

- [ADR-001: Dual-Output DKG](../../../../atomica/docs/adr-001-dual-output-dkg.md)
- [Implementation Plan](../../../../atomica/docs/implementation-plan-unified-dkg-ibe.md)
- `insecure_field/transcript.rs` - Reference implementation (without encryption)
- `das/weighted_protocol.rs` - Reference for weighted threshold support
- `encryption_elgamal.rs` - ElGamal encryption primitives
