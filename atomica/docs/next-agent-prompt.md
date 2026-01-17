# Next Agent Prompt: Complete Phase 1 Smoke Tests

## Current State

**Branch:** `timelock-das-vpss`
**Plan Document:** `atomica/docs/implementation-plan-unified-dkg-ibe.md` (v1.4)

### Completed

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Feasibility - MPK extraction from DKG transcript | ✅ Complete |
| 1A | Move module `ibe_config.move` | ✅ Complete |
| 1B | Rust MPK extraction in `dkg.rs` | ✅ Complete |
| 1C | IBE Crypto Module (`aptos-dkg/src/ibe/`) | ✅ Complete |
| **1D-E** | **Smoke tests** | **🔲 YOUR TASK** |

### Test Status

- IBE unit tests: 16/16 passing
- Move integration tests (`ibe_config`): 9/9 passing
- Smoke test (`randomness::e2e_correctness`): passing (no regressions)

---

## Your Task: Phase 1D-E Smoke Tests

Create smoke tests that validate the full DKG → MPK → IBE flow.

### Files to Create

1. **`testsuite/smoke-test/src/timelock/mod.rs`**
   ```rust
   pub mod mpk_on_chain;
   pub mod mpk_encrypt_decrypt;
   ```

2. **`testsuite/smoke-test/src/timelock/mpk_on_chain.rs`**
   - Wait for DKG to complete
   - Query MPK from chain via `ibe_config::get_mpk()` view function
   - Verify MPK is 96 bytes (valid G2 compressed)
   - Verify MPK can be deserialized to valid G2 point
   - Verify `is_ready()` returns true
   - Verify chain liveness

3. **`testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs`**
   - Read MPK from chain
   - Create test identity using `compute_identity(timelock_id, deadline_us)`
   - Encrypt a test message using `ibe_encrypt()`
   - Derive decryption key from validator shares
   - Decrypt and verify plaintext matches
   - Verify wrong identity fails to decrypt

4. **Update `testsuite/smoke-test/src/lib.rs`**
   - Add `pub mod timelock;`

### Reference Implementation

See the implementation plan for detailed test templates:
- Section "Smoke Test 1: `mpk_on_chain`"
- Section "Smoke Test 2: `mpk_encrypt_decrypt`"

Existing smoke test to reference: `testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs`

### IBE Crypto API (Already Implemented)

```rust
use aptos_dkg::ibe::{
    compute_identity,
    hash_to_g1,
    derive_decryption_key,
    verify_decryption_key,
    ibe_encrypt,
    ibe_decrypt,
    Ciphertext,
};
```

### Run Commands

```bash
# After implementation
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::mpk_on_chain -- --nocapture
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib timelock::mpk_encrypt_decrypt -- --nocapture

# Verify no regressions
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
```

---

## After Phase 1: What Comes Next

Once smoke tests pass, proceed to **Phase 2: Timelock Registry** - adding user-facing timelock registration with deadline tracking. See the implementation plan for details.
