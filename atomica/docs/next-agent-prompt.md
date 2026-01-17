# Next Agent Prompt: Complete Phase 1 Smoke Tests

## Current State

**Branch:** `timelock-das-vpss`
**Plan Document:** `atomica/docs/implementation-plan-unified-dkg-ibe.md` (v1.4)

### Completed

| Phase    | Description                                      | Status                                                      |
| -------- | ------------------------------------------------ | ----------------------------------------------------------- |
| 0        | Feasibility - MPK extraction from DKG transcript | ✅ Complete                                                 |
| 1A       | Move module `ibe_config.move`                    | ✅ Complete                                                 |
| 1B       | Rust MPK extraction in `dkg.rs`                  | ✅ Complete                                                 |
| 1C       | IBE Crypto Module (`aptos-dkg/src/ibe/`)         | ✅ Complete                                                 |
| **1D-E** | **Smoke tests**                                  | **🔶 1/2 PASSED**                                           |
| ---      | ---                                              | ---                                                         |
|          |                                                  | `mpk_on_chain`: ✅ PASSED                                   |
|          |                                                  | `mpk_encrypt_decrypt`: 🔲 BLOCKED (IBE-DKG scalar mismatch) |

### Test Status

- IBE unit tests: 16/16 passing
- Move integration tests (`ibe_config`): 9/9 passing
- Smoke test `randomness::e2e_correctness`: passing (no regressions)
- `mpk_on_chain`: ✅ PASSED
- `mpk_encrypt_decrypt`: 🔲 BLOCKED (requires IBE-DKG scalar fix)

---

## Your Task: Fix Phase 1E (IBE-DKG Integration)

**Status:** `mpk_on_chain` is complete. `mpk_encrypt_decrypt` is BLOCKED.

### The Problem

The IBE module expects a `blstrs::Scalar` as the master secret for `derive_decryption_key()`:

```rust
pub fn derive_decryption_key(secret: &Scalar, identity: &[u8]) -> G1Affine
```

But the DKG produces a `DealtSecretKey` which is a G1 projective element, not a scalar:

```rust
pub struct DealtSecretKey {
    h_hat: G1Projective,  // G1 element, NOT a scalar
}
```

### Required Fix

Either:

**Option A:** Modify the DKG to output a scalar secret (not G1 element)

- Change `DealtSecretKey` type from G1 element to Scalar
- This is a significant architectural change to the PVSS/DKG code

**Option B:** Modify the IBE scheme to work with G1 elements

- Change `derive_decryption_key()` to accept G1 element instead of scalar
- Derive key as `dk = H(identity) * secret_as_g1` where `secret_as_g1` is the G1 element

**Option C:** Use a hash-to-scalar conversion

- Hash the G1 element bytes to derive a scalar
- Note: This changes the cryptographic construction and requires analysis

### Files Created

1. ✅ `testsuite/smoke-test/src/timelock/mpk_on_chain.rs` - COMPLETE
2. ✅ `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` - PARTIAL (validates storage, not crypto)
3. ✅ `testsuite/smoke-test/src/timelock/mod.rs` - Updated

### Run Commands

```bash
# Run completed test
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::mpk_on_chain -- --nocapture

# Verify no regressions
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
```

---

## What's Next

Before proceeding to Phase 2, fix the IBE-DKG scalar mismatch (see "Your Task" section above).

Once `mpk_encrypt_decrypt` passes (full encrypt/decrypt roundtrip), proceed to **Phase 2: Timelock Registry** - adding user-facing timelock registration with deadline tracking. See the implementation plan for details.
