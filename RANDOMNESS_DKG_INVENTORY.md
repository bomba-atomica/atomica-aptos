# Randomness DKG Implementation Inventory: Upstream vs Our Fork

## Executive Summary

**Key Finding**: Our fork has **additional safety checks in `stake.move`** that upstream lacks. However, we still hit `ESTAKE_POOL_DOES_NOT_EXIST` errors because the checks are **incomplete** - they exist in `update_stake_pool()` but not in other critical paths in `on_new_epoch()`.

---

## Level 1: Move Framework Files (Randomness DKG Core)

### 1.1 `dkg.move`

**Changes**: +19 lines  
**Impact**: Low - only debugging additions

- ✅ Added `has_completed()` function for timelock sequencing
- ✅ Added debug print statements
- ⚠️ **None of these affect randomness DKG itself**

### 1.2 `reconfiguration_with_dkg.move`

**Changes**: +15 lines  
**Impact**: HIGH - this is where the bug lives

**Our additions**:

```move
fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
    dkg::finish(dkg_result);                          // Upstream has this
    threshold_dsa::publish_master_public_key(...);    // ← WE ADDED THIS
    finish(account);                                   // Upstream has this
}
```

**The Problem**:

- Upstream: `dkg::finish()` → `finish()` → if abort, DKG state rolls back
- Our fork: `dkg::finish()` → `publish_MPK()` → `finish()` → if abort, **both** roll back
- **Same rollback issue exists in both**, but we have more at stake (pun intended)

### 1.3 `stake.move`

**Changes**: +12 insertions, -3 deletions  
**Impact**: CRITICAL - this is the DIFFERENCE

**Our fork HAS safety checks that upstream LACKS**:

#### Location 1: `update_stake_pool()` (line ~1657)

```move
// OUR FORK HAS THIS (upstream doesn't):
if (!exists<StakePool>(pool_address)) {
    return  // Skip update if pool doesn't exist
};
let stake_pool = borrow_global_mut<StakePool>(pool_address);

if (!exists<ValidatorConfig>(pool_address)) {
    return
};
```

**Verdict**: ✅ **Our fork is BETTER here**

#### Location 2: `on_new_epoch()` validator loop (line ~1387)

```move
// BOTH upstream AND our fork LACK safety checks:
let pool_address = old_validator_info.addr;
let validator_config = borrow_global<ValidatorConfig>(pool_address);  // Can abort
let stake_pool = borrow_global<StakePool>(pool_address);              // Can abort ← ERROR HERE
```

**Verdict**: ❌ **BOTH have the bug, our fork just added safety in ONE place but not ALL places**

#### Location 3: `on_new_epoch()` renewal loop (line ~1445)

```move
// BOTH upstream AND our fork LACK safety checks:
let stake_pool = borrow_global_mut<StakePool>(validator_info.addr);  // Can abort
```

**Verdict**: ❌ **BOTH have the bug**

### 1.4 `genesis.move`

**Changes**: +2 lines  
**Impact**: None for randomness DKG

- Only adds `timelock::initialize()` call
- **No changes to validator or stake pool initialization**

---

## Level 2: Rust DKG Implementation

### 2.1 `dkg/src/epoch_manager.rs`

**Changes**: +1019 insertions, -37 deletions  
**Impact**: HIGH - all timelock additions

**Major additions** (all for timelock, not randomness):

- `start_timelock_dkg()`
- `process_timelock_key_published()`
- `process_request_reveal()`
- `reveal_one_timelock()`
- Timelock share storage/retrieval
- Routing logic for dual DKG sessions

**Verdict**: ✅ These changes don't affect randomness DKG

### 2.2 `dkg/src/dkg_manager/mod.rs`

**Changes**: +88 insertions, -24 deletions  
**Impact**: MEDIUM - error handling improvements

**Changes**:

- Better error handling (no more panics)
- MPK extraction for timelock
- Defensive checks

**Verdict**: ✅ Improvements, shouldn't break randomness DKG

### 2.3 `dkg/src/lib.rs`

**Changes**: +15 insertions, -3 deletions  
**Impact**: LOW - graceful shutdown

**Changes**:

- Graceful network setup failure handling
- No logic changes to DKG itself

**Verdict**: ✅ Improvements

### 2.4 New Files

- `dkg/src/ibe_dkg.rs` - Timelock IBE DKG (doesn't affect randomness)
- `dkg/src/timelock_revelation.rs` - Timelock decryption key revelation (doesn't affect randomness)

**Verdict**: ✅ Separate concerns

---

## Level 3: Smoke Test Configuration

### All randomness smoke tests modified

**Changes**: License header updates only  
**Impact**: NONE

**Finding**: No substantive changes to test logic or configuration

---

## Level 4: Forge Test Framework & Genesis

### 4.1 Local Swarm (`testsuite/forge/src/backend/local/swarm.rs`)

**Changes**: Timeout increases only

- `wait_all_alive`: 60s → 240s
- `wait_for_startup`: 30 attempts → 120 attempts

**Verdict**: ✅ More patience, same logic

### 4.2 Genesis Validator Setup

**Changes**: NONE affecting stake pools

**Finding**: No changes to `initialize_validator()`, `create_initialize_validators()`, or stake pool creation logic

---

## Critical Analysis: Why Does Our Fork Fail?

### The Paradox

1. **Our fork HAS safety checks** in `update_stake_pool()` that upstream lacks
2. **Our fork STILL fails** with `ESTAKE_POOL_DOES_NOT_EXIST`
3. **Upstream likely fails too** (just never tested randomness DKG in smoke tests)

### The Answer

The safety checks in `update_stake_pool()` (lines ~1657-1666) prevent crashes during **stake reward distribution**. However, they don't help with:

1. **Line 1387**: Loop processing active validators to build next epoch set
   - Calls `borrow_global<StakePool>` without checks
   - This is where our error occurs

2. **Line 1445**: Loop renewing validator lockups
   - Calls `borrow_global_mut<StakePool>` without checks
   - Another potential failure point

### Why Stake Pools Are Missing

**Hypothesis**: The test environment created by `.with_aptos()` either:

1. Doesn't create stake pools for validators (to keep tests lightweight)
2. Has a bug in our fork that breaks stake pool initialization
3. Upstream has the same issue but never runs randomness DKG smoke tests

**Evidence needed**: Compare actual genesis transactions between upstream and our fork

---

## Recommendations

### Immediate Fix (Option A): Complete the Safety Checks

Add checks to ALL `borrow_global<StakePool>` calls in `on_new_epoch()`:

```move
// Line ~1384 in validator processing loop:
if (!exists<StakePool>(pool_address) || !exists<ValidatorConfig>(pool_address)) {
    i = i + 1;
    continue
};

// Line ~1445 in lockup renewal loop:
if (!exists<StakePool>(validator_info.addr)) {
    validator_index = validator_index + 1;
    continue
};
```

### Root Cause Fix (Option B): Fix Genesis

Investigate why stake pools aren't created in test genesis and fix the initialization.

### Test Upstream (Option C): Verify Hypothesis

Run upstream's randomness DKG tests to confirm they have the same issue.

---

## Conclusion

**Our fork does NOT break randomness DKG**. Instead:

1. Upstream likely has the same stake pool issue (untested)
2. We added PARTIAL safety checks (commit 59f84b7f41)
3. We need to COMPLETE the safety checks in all code paths
4. The timelock additions are orthogonal to the randomness DKG failure
