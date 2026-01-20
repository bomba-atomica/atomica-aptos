# Phase 1 Recovery Plan: On-Chain MPK Storage

**Date:** January 17, 2026
**Branch:** `timelock-das-vpss-recovery`
**Base Commit:** `2ce3bf90a7` (update workflow)
**Problem Commit:** `e4a5d23aca` (feat(ibe): Phase 1 - on-chain MPK storage with ibe_config module)

---

## Problem Statement

Commit `e4a5d23` on `timelock-das-vpss` introduced Phase 1 of the IBE implementation (on-chain MPK storage). This commit caused the `randomness::e2e_correctness` smoke test to fail with a **Swarm liveness check timeout** - validators appear to get stuck during the DKG/reconfiguration phase.

### Current State

| Branch                    | `randomness::e2e_correctness` | Status                              |
| ------------------------- | ----------------------------- | ----------------------------------- |
| `timelock-das-vpss-recovery` | PASSES                     | Clean baseline at `2ce3bf90a7`      |
| `timelock-das-vpss`        | FAILS                        | Broken after `e4a5d23`              |

---

## What Commit e4a5d23 Changed

The commit made 9 file changes totaling ~460 lines:

### Rust Changes

1. **`aptos-move/aptos-vm/Cargo.toml`**
   - Added `aptos-dkg = { workspace = true }` dependency

2. **`aptos-move/aptos-vm/src/validator_txns/dkg.rs`**
   - Added `extract_mpk_from_transcript()` function to deserialize transcript and get dealt public key
   - Modified `process_dkg_result()` to extract MPK and pass it to Move function
   - Changed Move function call to pass 3 arguments instead of 2 (added `mpk`)

### Move Changes

3. **`aptos-move/framework/aptos-framework/sources/ibe_config.move`** (NEW - 213 lines)
   - New module storing `IBEPublicParams { mpk, epoch }`
   - `initialize()` - called in genesis
   - `set_mpk()` - friend function called by reconfiguration
   - View functions: `get_mpk()`, `get_epoch()`, `is_ready()`
   - Unit tests

4. **`aptos-move/framework/aptos-framework/sources/genesis.move`**
   - Added `use aptos_framework::ibe_config`
   - Added `ibe_config::initialize(&aptos_framework_account)` call

5. **`aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.move`**
   - Added `use aptos_framework::ibe_config` and `use std::vector`
   - Changed `finish_with_dkg_result(account, dkg_result)` signature to `finish_with_dkg_result(account, dkg_result, mpk)`
   - Added logic to call `ibe_config::set_mpk()` if mpk is non-empty

6. **`aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.spec.move`**
   - Updated function signature in spec to match new 3-argument version

### Test/Doc Changes

7. **`testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs`** (NEW - 190 lines)
   - New smoke test to verify MPK storage

8. **`testsuite/smoke-test/src/randomness/mod.rs`**
   - Added `mod ibe_mpk_on_chain`

9. **`atomica/docs/phase1_recovery_plan.md`** (NEW - 33 lines)
   - Original recovery plan doc

---

## Recovery Strategy

**Goal:** Reimplement the features from `e4a5d23` incrementally on this recovery branch, testing after each step with `randomness::e2e_correctness` smoke test.

### Phase 1A: Move Module Only (No Rust Changes)

Add the `ibe_config.move` module and wire it into genesis, but **without** the Rust side changes. The MPK will remain empty (epoch 0) but the chain should still work.

**Files to create/modify:**
1. Create `ibe_config.move`
2. Modify `genesis.move` to call `ibe_config::initialize()`
3. Keep `reconfiguration_with_dkg.move` unchanged (no MPK setting yet)

**Test:** Run `randomness::e2e_correctness`
- If PASS: Move module is not the problem
- If FAIL: Something in Move wiring breaks chain

**Command:**
```bash
RUST_MIN_STACK=104857600 cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
```

---

### Phase 1B: Wire reconfiguration_with_dkg (Move Only)

Modify `reconfiguration_with_dkg.move` to accept the MPK parameter and call `set_mpk()`.

**Files to modify:**
1. `reconfiguration_with_dkg.move` - add mpk parameter and set_mpk call
2. `reconfiguration_with_dkg.spec.move` - update signature

**Critical Question:** This changes the function signature that Rust calls. If Rust still passes only 2 args, the call will fail.

**Option A:** Add the MPK parameter but make the Rust call still work somehow (default values?)
**Option B:** Do Phase 1B and 1C together since they're interdependent

**Decision:** Skip to Phase 1C - the Move signature change and Rust caller change must happen together.

---

### Phase 1C: Add Rust-Side MPK Extraction

Add the Rust changes to extract MPK from transcript and pass to Move.

**Files to modify:**
1. `aptos-move/aptos-vm/Cargo.toml` - add `aptos-dkg` dependency
2. `aptos-move/aptos-vm/src/validator_txns/dkg.rs` - add MPK extraction and pass to Move

**This is the likely failure point.** The original code:
```rust
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;
    Ok(transcript.main.get_dealt_public_key().to_bytes().to_vec())
}
```

**Potential Issues:**
1. Transcript deserialization might fail for some transcripts
2. Error handling might cause chain halt
3. The extra Move argument might cause issues

**Test:** Run `randomness::e2e_correctness`

---

### Phase 1D: Add Smoke Test

Only after 1A-1C pass, add the new smoke test.

**Files to create/modify:**
1. Create `testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs`
2. Modify `testsuite/smoke-test/src/randomness/mod.rs`

---

## Execution Checklist

### Preparation
- [x] Fork new branch `timelock-das-vpss-recovery` from last passing commit
- [x] Verify `randomness::e2e_correctness` passes on clean branch
- [x] Document what commit `e4a5d23` changed

### Phase 1A: Move Module
- [ ] Create `ibe_config.move`
- [ ] Modify `genesis.move`
- [ ] Run smoke test
- [ ] Commit if passes

### Phase 1B+1C: Wire Everything Together
- [ ] Modify `reconfiguration_with_dkg.move`
- [ ] Modify `reconfiguration_with_dkg.spec.move`
- [ ] Add `aptos-dkg` dependency to `aptos-vm/Cargo.toml`
- [ ] Modify `dkg.rs` to extract and pass MPK
- [ ] Run smoke test
- [ ] Debug if fails (this is the risky step)
- [ ] Commit if passes

### Phase 1D: Add Tests
- [ ] Create `ibe_mpk_on_chain.rs`
- [ ] Update `mod.rs`
- [ ] Run new test
- [ ] Commit if passes

### Finalization
- [ ] Run full regression suite
- [ ] Rebase onto `timelock-das-vpss` or merge recovery branch

---

## Debugging Strategy

If `randomness::e2e_correctness` fails after Phase 1C:

### 1. Check Validator Logs
```bash
# During test run, look for errors
grep -i "error\|panic\|fail" /tmp/aptos-*/validator-*/aptos.log
```

### 2. Check DKG Processing
```bash
# Look for DKG-related log messages
grep -i "dkg\|transcript\|mpk" /tmp/aptos-*/validator-*/aptos.log
```

### 3. Isolate the Failure

**Test A:** Does MPK extraction work?
```rust
// Add logging in dkg.rs before extract_mpk_from_transcript
info!("[DKG] About to extract MPK from {} byte transcript", transcript_bytes.len());
let mpk_result = extract_mpk_from_transcript(transcript_bytes);
info!("[DKG] MPK extraction result: {:?}", mpk_result.as_ref().map(|v| v.len()));
```

**Test B:** Does the Move call succeed?
```rust
// Add logging after session.execute_function
info!("[DKG] Move function call completed");
```

### 4. Make MPK Optional

If MPK extraction is the issue, make it optional:
```rust
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Vec<u8> {
    match bcs::from_bytes::<Transcripts>(transcript_bytes) {
        Ok(transcript) => transcript.main.get_dealt_public_key().to_bytes().to_vec(),
        Err(e) => {
            warn!("[DKG] Failed to extract MPK: {:?}. IBE disabled this epoch.", e);
            Vec::new()  // Empty MPK = IBE not ready, but chain continues
        }
    }
}
```

---

## Success Criteria

1. `randomness::e2e_correctness` passes after all changes
2. `ibe_mpk_on_chain` smoke test passes (MPK is stored on-chain)
3. No chain liveness issues (validators don't get stuck)
4. Ready to merge back into `timelock-das-vpss`

---

## Notes

- The original commit tried to do everything at once, making it hard to identify which change broke the test
- This recovery plan applies changes incrementally with testing after each phase
- The interdependency between Move signature change and Rust caller means 1B and 1C must be done together
- Error handling in consensus code is critical - never panic, always log and continue
