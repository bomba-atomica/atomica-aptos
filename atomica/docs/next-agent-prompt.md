# Agent Prompt: Phase 1E - Complete mpk_encrypt_decrypt Smoke Test

## Context

You are working on a Rust codebase implementing Identity-Based Encryption (IBE) for timelock encryption on the Aptos blockchain. The project uses a Dual-Output DKG that produces both G1 shares (for WVUF/randomness) and scalar shares (for IBE).

**Branch:** `timelock-das-vpss`
**Plan Document:** `atomica/docs/implementation-plan-unified-dkg-ibe.md` (v2.3)
**Status:** Phase 1E - IN PROGRESS

## Current State

### Completed (Phases 0-2.2)

| Phase | Description                         | Status      |
| ----- | ----------------------------------- | ----------- |
| 0     | Feasibility Test                    | ✅ COMPLETE |
| 1A-1D | IBE Primitives + MPK Storage        | ✅ COMPLETE |
| 2     | Scalar ElGamal PVSS                 | ✅ COMPLETE |
| 2.0.5 | Unit Tests for Scalar ElGamal       | ✅ COMPLETE |
| 2.1   | Integration into RealDKG + DKGTrait | ✅ COMPLETE |
| 2.2   | Aggregation Bug Fix                 | ✅ COMPLETE |

### What We Have

After DKG completes:

- `DealtSecretKeyShares.scalar` contains `Vec<(DealtSecretKeyShare, DealtPubKeyShare)>` for each validator
- `DealtSecretKeyShare` is `scalar_elgamal::DealtSecretKeyShare = h1^share` (G1 element)
- `DealtPubKeyShare` is `scalar_elgamal::DealtPubKeyShare = G2^share` (G2 element)
- The scalar PVSS uses chunked ElGamal encryption with 16 chunks of 16 bits each
- BSGS discrete log is used for decryption with expanded range for aggregated transcripts

### Test Status

- IBE unit tests: ✅ PASSING
- Scalar ElGamal unit tests: ✅ ALL 15 PASSING
- `randomness::e2e_correctness`: ✅ PASSING
- `mpk_on_chain`: ✅ PASSING
- `mpk_encrypt_decrypt`: 🔲 BLOCKED (needs raw scalar access)

### The Problem

The smoke test `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` is blocked (marked `#[ignore]`).

**Goal:** Encrypt a message using on-chain MPK, then decrypt using shares reconstructed from DKG.

**Current flow:**

1. DKG produces scalar shares via `decrypt_own_share()` → returns `DealtSecretKeyShare` (G1 element: h1^share)
2. IBE `derive_decryption_key(secret: &Scalar, identity)` expects raw `Scalar`
3. **MISMATCH:** We have G1 elements, but need raw scalars

**Flow needed:**

1. DKG produces scalar shares (done)
2. Extract raw scalar from decryption (needs fix)
3. Reconstruct scalar secret from shares
4. Derive IBE decryption key from scalar secret
5. Decrypt ciphertext and verify matches plaintext

## Your Task

Complete the `mpk_encrypt_decrypt` smoke test. There are two sub-tasks:

### Task 1: Enable Raw Scalar Access from Decryption

The current `decrypt_own_share()` returns `DealtSecretKeyShare` which wraps `h1^share`. But IBE needs the raw scalar to compute `H(identity)^secret`.

**Solution:** Modify the decryption flow to expose raw scalars.

**Approach:** In `scalar_elgamal/transcript.rs`, the `decrypt_own_share` function:

1. Decrypts each chunk via BSGS discrete log → gets raw `u16` values
2. Reconstructs scalar via `chunks_to_scalar(&chunks)` → gets raw `Scalar`
3. Wraps in `DealtSecretKeyShare` which contains `h1 * scalar`

**Modify to also return the raw scalar:**

```rust
fn decrypt_own_share(...)
    -> (DealtSecretKeyShare, DealtPubKeyShare, Scalar)  // Add raw Scalar
```

Also update `WeightedTranscript::decrypt_own_share` in `weighted_protocol.rs` to pass through the raw scalar.

### Task 2: Implement Scalar Secret Reconstruction

Add a function to reconstruct the master scalar secret from shares using Lagrange interpolation.

**Location:** `crates/aptos-dkg/src/ibe/mod.rs` or `types/src/dkg/real_dkg/mod.rs`

```rust
pub fn reconstruct_scalar_secret(
    shares: &[(Player, Scalar)],  // (player_id, scalar_share) pairs
    threshold: usize,
    weights: Option<&[usize]>,
) -> Scalar {
    // Lagrange interpolation over scalar field
    // If weights provided, use weighted Lagrange
}
```

### Task 3: Complete mpk_encrypt_decrypt Smoke Test

Modify `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs`:

1. Remove `#[ignore]` attribute
2. After DKG completes, extract scalar shares from `DealtSecretKeyShares.scalar`
3. For each validator, call `decrypt_own_share_scalar()` (or modified method) to get raw scalars
4. Collect `(player_id, scalar_share)` pairs
5. Reconstruct master scalar using Lagrange interpolation
6. Derive decryption key: `dk = derive_decryption_key(&master_scalar, &identity)`
7. Decrypt the ciphertext using `ibe::decrypt(&ciphertext, &dk)`
8. Verify decrypted plaintext matches original message

### Key Code Locations

**Scalar PVSS module:**

- `crates/aptos-dkg/src/pvss/scalar_elgamal/mod.rs` - module exports
- `crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs` - core PVSS logic (decrypt_own_share at line ~800)
- `crates/aptos-dkg/src/pvss/scalar_elgamal/weighted_protocol.rs` - weighted wrapper

**IBE module:**

- `crates/aptos-dkg/src/ibe/mod.rs` - IBE primitives (encrypt, decrypt, derive_decryption_key)
- `crates/aptos-dkg/src/ibe/ciphertext.rs` - ciphertext structure

**DKG integration:**

- `types/src/dkg/real_dkg/mod.rs` - RealDKG with scalar transcript
- `types/src/dkg/mod.rs` - DKGTrait definitions

**Existing smoke tests:**

- `testsuite/smoke-test/src/timelock/mpk_on_chain.rs` - MPK storage test
- `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` - BLOCKED test to fix
- `testsuite/smoke-test/src/randomness/e2e_correctness.rs` - DKG correctness test

**Reference for IBE encrypt/decrypt:**

- `crates/aptos-dkg/src/ibe/tests.rs` - unit tests showing encrypt/decrypt with known scalars

### Reference: IBE API

From `crates/aptos-dkg/src/ibe/mod.rs`:

```rust
// Encrypt uses H(id) and master public key
pub fn encrypt<M: Serialize>(
    mpk: &IBEMasterPublicKey,
    identity: &[u8],
    message: &M,
    rng: &mut impl RngCore,
) -> Result<IBECiphertext> { ... }

// Decrypt uses decryption key derived from secret
pub fn decrypt(
    ciphertext: &IBECiphertext,
    decryption_key: &IBEDecryptionKey,
) -> Result<Vec<u8>> { ... }

// Derive key from secret scalar
pub fn derive_decryption_key(
    secret: &Scalar,
    identity: &[u8],
) -> IBEDecryptionKey { ... }
```

## Implementation Steps

1. **Examine current `decrypt_own_share` implementation**
   - Location: `scalar_elgamal/transcript.rs` around line 800
   - The raw scalar is already computed via `chunks_to_scalar(&recovered_chunks)`
   - Currently only used to create `DealtSecretKeyShare = h1 * scalar`
   - Modify to also return this raw `Scalar`

2. **Modify return type of `decrypt_own_share`**
   - Update `Transcript` trait implementation in `transcript.rs`
   - Update `WeightedTranscript` wrapper in `weighted_protocol.rs`
   - Update any call sites that use this function

3. **Implement Lagrange interpolation for scalars**
   - Check if existing polynomial utilities can be reused
   - Support weighted threshold (validators with different stakes)
   - Location: likely in `crates/aptos-dkg/src/ibe/mod.rs`

4. **Complete smoke test logic**
   - Wire up the pieces in `mpk_encrypt_decrypt.rs`
   - Test encrypt → decrypt roundtrip with DKG-derived shares

5. **Run tests to verify**

   ```bash
   # Run the specific test (currently ignored)
   cargo test -p smoke-test --lib timelock::mpk_encrypt_decrypt -- --nocapture --ignored

   # Run all timelock tests
   cargo test -p smoke-test --lib timelock -- --nocapture

   # Run scalar ElGamal unit tests
   cargo test -p aptos-dkg --lib scalar_elgamal

   # Verify no regressions
   cargo test -p smoke-test --lib randomness::e2e_correctness
   ```

## Success Criteria

1. `mpk_encrypt_decrypt` smoke test passes (no `#[ignore]`)
2. Test encrypts a message using on-chain MPK
3. Test reconstructs scalar secret from shares
4. Test derives decryption key and decrypts successfully
5. Decrypted message matches original plaintext
6. All scalar ElGamal tests still pass
7. No regressions in `randomness::e2e_correctness`

## Additional Notes

- Read `atomica/docs/implementation-plan-unified-dkg-ibe.md` for full project context
- Read `atomica/docs/adr-001-dual-output-dkg.md` for architecture decisions
- Commit messages should follow pattern: `feat(timelock): Description`
- The codebase uses BLS12-381 curves (blstrs crate)
- Tests may take 4+ minutes to run, use `--test-threads=1` for smoke tests

## Questions to Investigate

1. Does `decrypt_own_share` already compute the raw scalar before wrapping in G1? (Yes, via `chunks_to_scalar`)
2. Are there existing Lagrange interpolation utilities in the codebase?
3. Does the weighted protocol need special handling for Lagrange coefficients?
4. What is the expected format for `IBEMasterPublicKey` serialization?
