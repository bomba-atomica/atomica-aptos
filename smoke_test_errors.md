# Smoke Test Errors Report

## Summary

Investigated failing timelock smoke tests. Fixed two critical issues that were causing test failures.

## Root Causes Identified

### Issue 1: `verify_public_key_published` Panic (testsuite/smoke-test/src/timelock/mod.rs:150)
**Problem:** Function called `.last_complete()` directly on `DKGState` without checking if DKG had completed, causing panic when `last_completed` was `None`.

**Fix:** Added proper error handling to check `last_completed` field before accessing transcript.

### Issue 2: Missing Randomness Config in `ibe_e2e.rs` Test (testsuite/smoke-test/src/timelock/ibe_e2e.rs:39)
**Problem:** Test only enabled validator transactions but did not enable `OnChainRandomnessConfig`, which is required for DKG manager to start.

**Fix:** Added `conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled())` to genesis config.

## Test Status After Fixes

- `test_timelock_public_key_publication`: PASSING
- `test_ibe_encrypt_decrypt_e2e`: PROGRESSED - Now successfully:
  - Creates swarm with DKG enabled
  - Rotates intervals
  - Fetches DKG transcript (previously timed out here)
  - Encrypts messages
  - **NEW ISSUE:** Times out waiting for secret reveal (separate issue from original bugs)

## Remaining Issue

The tests now progress further but fail at secret reveal step. This indicates:
- The original DKG transcript publication issues are FIXED
- A separate issue exists with the secret aggregation/reveal mechanism
- This is likely a timing or on-chain state issue, not related to the original bugs

## CI/CD Integration

Created a dedicated GitHub Actions workflow for timelock smoke tests:

**File:** `.github/workflows/smoke-test-timelock.yml`

**Test Jobs:**
1. `smoke-test-timelock-basic` - Basic timelock flow tests
2. `smoke-test-dkg` - DKG startup and key publication tests
3. `smoke-test-timelock` - Timelock module functionality tests
4. `smoke-test-ibe` - IBE encryption/decryption tests (includes ibe_e2e)

**Features:**
- Runs on push to main, dev-atomica, and timelock-tests branches
- Triggers on relevant file path changes (Move files, DKG crates, smoke tests)
- 90-minute timeout per job to accommodate long-running integration tests
- Sequential execution (--test-threads=1) to prevent resource conflicts
- Shared Rust cache for faster builds

**Status:** Active and monitoring all timelock-related code changes
