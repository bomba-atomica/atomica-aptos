# Agent Prompt: Phase 1E - Complete mpk_encrypt_decrypt Smoke Test

## Context

You are working on a Rust codebase implementing Identity-Based Encryption (IBE) for timelock encryption on the Aptos blockchain. The project uses a Dual-Output DKG that produces both G1 shares (for WVUF/randomness) and scalar shares (for IBE).

**Branch:** `timelock-das-vpss`
**Plan Document:** `atomica/docs/implementation-plan-unified-dkg-ibe.md`

## Current State

### Completed
- ✅ Scalar ElGamal PVSS (`crates/aptos-dkg/src/pvss/scalar_elgamal/`)
- ✅ IBE primitives (`crates/aptos-dkg/src/ibe/`)
- ✅ RealDKG integration - scalar transcript is dealt, aggregated, and decrypted
- ✅ `DealtSecretKeyShares.scalar` now contains scalar shares after DKG

### The Problem

The smoke test `testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs` is blocked (marked `#[ignore]`).

**Current flow:**
1. DKG completes, produces `Transcripts` with `scalar: Option<ScalarTrx>`
2. Test decrypts shares via `decrypt_secret_share_from_transcript()` → returns `DealtSecretKeyShares`
3. Test reconstructs secret via `reconstruct_secret_from_shares()` → returns `DealtSecret` (G1)
4. **BLOCKED:** Test has G1 secret but IBE needs scalar secret

**What we have after Phase 2.1:**
- `DealtSecretKeyShares.scalar` is `Option<Vec<(DealtSecretKeyShare, DealtPubKeyShare)>>`
- These are the scalar shares needed for IBE
- The scalar type is `<ScalarTrx as Transcript>::DealtSecretKeyShare`

## Your Task

Implement the missing piece: **reconstruct the scalar secret from scalar shares and derive the IBE decryption key**.

### Option A: Add `reconstruct_scalar_secret_from_shares()`

Add a function to reconstruct the scalar secret from scalar shares using Lagrange interpolation:

```rust
// In types/src/dkg/real_dkg/mod.rs or a new helper module
pub fn reconstruct_scalar_secret_from_shares(
    wconfig: &SSConfig,
    player_share_pairs: Vec<(Player, <ScalarTrx as Transcript>::DealtSecretKeyShare)>,
) -> Scalar {
    // Use Lagrange interpolation to combine shares
    // The scalar PVSS shares are already scalars, just need to interpolate
}
```

### Option B: Add `derive_decryption_key_from_shares()`

Add a function that takes scalar shares directly and derives the decryption key:

```rust
// In crates/aptos-dkg/src/ibe/mod.rs
pub fn derive_decryption_key_from_shares(
    shares: &[(&Player, &Scalar)],  // (player_id, scalar_share) pairs
    weights: &[usize],              // for weighted Lagrange
    identity: &[u8],
) -> G1Affine {
    // Compute dk = H(identity) * sum(lambda_i * share_i)
    // where lambda_i are Lagrange coefficients
}
```

### Files to Modify

1. **`testsuite/smoke-test/src/timelock/mpk_encrypt_decrypt.rs`**
   - Remove `#[ignore]`
   - Extract scalar shares from `DealtSecretKeyShares.scalar`
   - Use new reconstruction function to get scalar secret
   - Derive decryption key and complete decrypt verification

2. **`crates/aptos-dkg/src/ibe/mod.rs`** (if adding `derive_decryption_key_from_shares`)
   - Add weighted Lagrange interpolation for scalar shares
   - Or call into existing interpolation utilities

3. **`types/src/dkg/real_dkg/mod.rs`** (if adding `reconstruct_scalar_secret_from_shares`)
   - Add reconstruction function parallel to existing `reconstruct_secret_from_shares`

### Key Code Locations

**Scalar share type:**
```rust
// types/src/dkg/real_dkg/mod.rs:50
pub type ScalarTrx = scalar_elgamal::WeightedTranscript;

// Scalar shares are Vec of (DealtSecretKeyShare, DealtPubKeyShare) tuples
// Each DealtSecretKeyShare contains a G1 element (h1^share) - NOT the raw scalar
```

**Wait - Important Discovery:**

Looking at `scalar_elgamal/transcript.rs`, the `DealtSecretKeyShare` is actually `DealtSecretKey` which wraps a G1 element (`h_hat: G1Projective`), not a raw scalar!

This is because the scalar ElGamal PVSS encrypts `h1 * f(i)` (scalar times generator), not the raw scalar.

**This means the reconstruction approach differs:**
- For IBE decryption, we need `H(identity)^secret`
- With shares being `h1^share_i`, we can't directly compute `H(identity)^secret`
- We need to use pairing-based share combination

**Alternative approach:**

The decryption key for IBE is `dk = H(id)^s`. With shares `h1^s_i`:
- Each validator computes their contribution: `H(id)^s_i` (they know `s_i` because they decrypted it)
- Contributions are combined with Lagrange coefficients in the exponent

So the flow is:
1. Validator decrypts their scalar share (gets scalar `s_i`)
2. Validator computes `H(id)^s_i` for the target identity
3. Combine contributions: `prod(H(id)^(s_i * lambda_i)) = H(id)^(sum(s_i * lambda_i)) = H(id)^s`

**Check the decrypt_own_share output:**

In `scalar_elgamal/transcript.rs:183-202`:
```rust
fn decrypt_own_share(...) -> (DealtSecretKeyShare, DealtPubKeyShare) {
    // C[i] = h1 * f(i) + ek * r
    // Decrypt: C[i] - dk * C_0 = h1 * f(i)
    let dealt_secret_key_share = self.C[i] - (self.C_0 * *dk);
    // This is h1 * scalar_share, NOT the raw scalar
}
```

So `DealtSecretKeyShare` contains `h1^scalar_share`, not the raw scalar itself.

### Revised Approach

To get the raw scalar shares for IBE:
1. We need to extract the scalar from `h1^s` which requires DLOG (not feasible)
2. OR we change the PVSS to output raw scalars (but then we lose verifiability)
3. OR we use a different reconstruction approach

**Actually, looking more carefully at the existing IBE code:**

`derive_decryption_key(secret: &Scalar, identity: &[u8])` expects a raw `Scalar`.

But our scalar PVSS outputs `G1` elements (`h1^scalar`), not raw scalars.

**The fundamental issue:** The scalar ElGamal PVSS was designed to output `G1` elements (for verifiability via pairings), but IBE needs raw scalars.

### Possible Solutions

1. **Use discrete log (not practical):** Extract scalar from `h1^s` - computationally infeasible

2. **Change PVSS to output scalars:** Modify the scheme to directly output scalars - loses verifiability

3. **Pairing-based decryption key derivation:**
   - Instead of computing `H(id)^s` from scalar `s`
   - Use shares `h1^s_i` and compute via pairings
   - This requires a different IBE construction

4. **Hybrid approach:**
   - Keep `h1^s_i` shares for verification
   - Additionally encrypt raw scalars `s_i` under each validator's key
   - Validators can then compute `H(id)^s_i` directly

### Recommended Path Forward

**Option 4 (Hybrid)** is likely correct. Check if the current implementation already provides raw scalar access.

Look at:
- `transcript.rs` decrypt logic - does it have access to raw scalar before wrapping in G1?
- Can we add a parallel output that gives the raw scalar share?

If the PVSS polynomial evaluation gives us `f(i)` (scalar), we just need to expose it separately.

### Test Commands

```bash
# Run the specific test (currently ignored)
cargo test -p smoke-test --lib timelock::mpk_encrypt_decrypt -- --nocapture --ignored

# Run all timelock tests
cargo test -p smoke-test --lib timelock -- --nocapture

# Run unit tests
cargo test -p aptos-dkg
```

### Success Criteria

1. `mpk_encrypt_decrypt` test passes (remove `#[ignore]`)
2. Test encrypts a message using on-chain MPK
3. Test derives decryption key from scalar shares
4. Test successfully decrypts and verifies message matches plaintext
5. No regressions in existing tests

### Additional Notes

- Read `atomica/docs/implementation-plan-unified-dkg-ibe.md` for full context
- Read `atomica/docs/adr-001-dual-output-dkg.md` for architecture decisions
- The `ibe/tests.rs` file has unit tests showing IBE encrypt/decrypt with known scalars
- Commit messages should follow pattern: `feat(timelock): Description`

## Questions to Investigate

1. Does `decrypt_own_share` have access to raw scalar `f(i)` before multiplying by `h1`?
2. Can we modify `DealtSecretKeyShare` to include both `h1^s` and raw `s`?
3. Is there a pairing-based way to compute `H(id)^s` from `h1^s` shares?
