# Prior Art: Aptos BIBE and DKG

**See**: `/atomica/code-review/2026-01-06-timelock-cryptography-review.md` for comprehensive analysis.

## Summary

Aptos's timelock encryption uses:

| Component | Curve     | Purpose                                                   |
| --------- | --------- | --------------------------------------------------------- |
| **BIBE**  | BN254     | Batch Identity-Based Encryption for timelock transactions |
| **DKG**   | BLS12-381 | Distributed Key Generation via PVSS                       |

### Key Findings

1. **BN254 for BIBE**: Chosen for smaller ciphertexts (64-byte G2 elements) and faster pairings
2. **BLS for DKG**: Uses 128-bit security for distributed key generation
3. **Ephemeral keys**: BN254 master secret exists only in memory, regenerated each epoch from:
   - BLS private key (persisted in secure storage)
   - DKG transcript (stored on-chain)

### Critical Issues Found

| Issue                                    | Severity    | Status                |
| ---------------------------------------- | ----------- | --------------------- |
| DST inconsistency (Rust vs Move)         | 🔴 Critical | **Blocks deployment** |
| Dead code markers on IBE functions       | 🔴 Critical | **Blocks deployment** |
| Missing input validation                 | 🔴 Critical | **Blocks deployment** |
| Panic-prone error handling (35+ unwraps) | 🔴 Critical | **Blocks deployment** |

### Key Storage

| Key             | Storage                             | Curve     |
| --------------- | ----------------------------------- | --------- |
| BLS Private Key | PersistentSafetyStorage (encrypted) | BLS12-381 |
| DKG Transcript  | On-chain (DKGState)                 | BLS12-381 |
| BN254 Msk Share | **In-memory only**                  | BN254     |

### Recovery After Restart

```
BLS key → DKG transcript (on-chain) → Decrypt share → BN254 (in memory)
```

### Compliance Summary

- **17 requirements checked**: 5 passed (29%)
- **Overall**: 🔴 **DO NOT DEPLOY** - Critical issues must be resolved

## Documents

- **Detailed Review**: `code-review/2026-01-06-timelock-cryptography-review.md`
- **Cross-Language Tests**: `timelock-tests/test/cross-lang-ibe-verification.test.ts`
- **Golden Vectors**: `golden-vectors/verify.ts`
