# MPK Publication Test Failure - Root Cause Analysis

## Issue

The test `cargo t -p smoke-test test_mpk_published` fails after 60s with "Master Public Key not found for interval 1"

## Root Cause **CONFIRMED**

**Our fork added timelock MPK publishing code to `finish_with_dkg_result` that doesn't exist in upstream Aptos.**

### The Bug

In `aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.move`:

```move
fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>) {
    dkg::finish(dkg_result);                              // Step 1: Save DKG state
    threshold_dsa::publish_master_public_key(...);        // Step 2: Save MPK (OUR ADDITION)
    finish(account);                                       // Step 3: ABORTS → Rolls back steps 1 & 2!
}
```

**The comment in our code (lines 79-81) claims**: "In test environments without stake pools, finish() may abort. However, the DKG state and MPK have already been saved above so timelock functionality will still work"

**THIS IS FALSE** - Move transactions are atomic. When `finish()` aborts, it rolls back **everything**, including `dkg::finish()` and the MPK publication.

### Why finish() Aborts

```
finish()
  → reconfiguration::reconfigure()  (reconfiguration.move:63)
    → stake::on_new_epoch()  (reconfiguration.move:134)
      → borrow_global<StakePool>(pool_address)  (stake.move:1387)
        → ABORTS with error 65550 (ESTAKE_POOL_DOES_NOT_EXIST)
```

### Error Code Breakdown

- **Error**: `65550` (`0x1000e`)
- **Category**: `0x1` = `INVALID_ARGUMENT`
- **Code**: `0xe` = `14` = `ESTAKE_POOL_DOES_NOT_EXIST` (stake.move:74)

### What Happens

1. ✅ Randomness DKG completes successfully
2. ✅ DKG transcript is aggregated
3. ✅ `finish_with_dkg_result` is called
4. ❌ **Transaction ABORTS**, rolling back all state changes
5. ❌ `dkg::has_completed()` stays `false` (state was rolled back!)
6. ❌ Timelock DKG never starts (blocked waiting for randomness DKG)
7. ❌ No MPK is published
8. ❌ Test fails

## Test Results

| Test                                   | Result      | Notes                                                 |
| -------------------------------------- | ----------- | ----------------------------------------------------- |
| `epoch_transition_without_dkg`         | ✅ **PASS** | Epochs advance fine when randomness disabled          |
| Randomness DKG (w/ randomness enabled) | ❌ **FAIL** | DKG completes but state rollback prevents persistence |
| `test_mpk_published`                   | ❌ **FAIL** | Same root cause - MPK never saved                     |

## Evidence from Logs

```
[DKG] Finished DKG session - transcript published     ← Rust thinks DKG succeeded
[TIMELOCK] About to call finish_with_dkg_result
[TIMELOCK] finish_with_dkg_result execution result: false   ← Move function FAILED
WARN Execution error InternalError {
  error: "status UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION...
         ABORTED { code: 65550, location: 0x1::stake }"     ← The abort
}
dkg_manager_state=Finished { proposed: true }         ← Rust side still thinks it worked
```

**But the on-chain DKG state was rolled back**, so `dkg::has_completed()` returns `false`!

## Upstream Comparison

**Upstream aptos-core does NOT have timelock/threshold_dsa** - it's purely our addition.

Upstream's `finish_with_dkg_result` only calls:

```move
dkg::finish(dkg_result);
finish(account);
```

We added the MPK publishing logic but didn't account for transaction atomicity.

## Why Are Stake Pools Missing?

**Investigation needed**: The test uses `.with_aptos()` which should initialize validators with stake pools via `genesis.move`.

**Working theory**: Something in our fork or test setup broke stake pool initialization, OR the test environment intentionally skips stake pools for simplicity.

Key finding: `epoch_transition_without_dkg` test **passes**, meaning epoch transitions work fine when randomness is disabled. The stake pool issue only manifests when `finish_with_dkg_result` is called.

## Solutions

### Option 1: Fix Stake Pool Initialization ⭐ (Recommended)

- Investigate why stake pools aren't created in test genesis
- Compare our fork's genesis setup with upstream
- Ensure validators have proper `StakePool` resources

### Option 2: Make stake::on_new_epoch() Graceful

Add a safety check in `stake.move:1387`:

```move
if (!stake_pool_exists(pool_address)) {
    // Skip validators without stake pools (test environments)
    continue
};
let stake_pool = borrow_global<StakePool>(pool_address);
```

### Option 3: Split finish_with_dkg_result (Complex)

Restructure to save DKG state in a separate transaction before reconfiguration:

- Transaction 1: Save DKG result + MPK (can't roll back)
- Transaction 2: Attempt reconfiguration (can fail safely)

However, this breaks the atomic epoch transition model.

## Next Steps

1. ✅ Confirmed upstream doesn't have timelock (so their tests don't hit this)
2. ✅ Identified transaction rollback as root cause
3. ⏭️ Check our fork's changes to `genesis.move` and validator initialization
4. ⏭️ Test if adding stake pools to test genesis fixes the issue
5. ⏭️ Consider Option 2 (graceful degradation) as quickest fix
