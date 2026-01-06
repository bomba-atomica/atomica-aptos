# Timelock Secret Revelation Debugging - Session Summary

**Date**: 2026-01-05
**Issue**: `test_timelock_secret_revelation` failing - secret never revealed
**Status**: Root cause identified, fix in progress

## Work Completed

### 1. Event Subscription Investigation ✅

**Verified**:
- Validators correctly subscribe to timelock events (aptos-node/src/state_sync.rs:96-103)
- Event handler exists and is configured (dkg/src/epoch_manager.rs:221-234)
- `process_timelock_reveal()` IS implemented (dkg/src/epoch_manager.rs:759-827)

**Findings**:
- All event infrastructure is properly set up
- Documentation claiming revelation was "NOT IMPLEMENTED" was incorrect

### 2. Time Scale Consistency Check ✅

**Verified**:
- Move code uses `timestamp::now_microseconds()` ✅
- Config uses `get_interval_microseconds()` ✅
- Test converts `5 seconds × 1,000,000 = 5,000,000 microseconds` ✅

**Findings**:
- No time scale mismatch - all using microseconds consistently

### 3. Move Debug Print Implementation ✅

**Added debug logging to `timelock.move::on_new_block()`**:

```move
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] interval_micros="));
aptos_std::debug::print(&interval_micros);
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] elapsed="));
aptos_std::debug::print(&elapsed);
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] should_rotate="));
aptos_std::debug::print(&(elapsed > interval_micros));
```

**Key Discovery**:
- ✅ CORRECT: Use `aptos_std::debug::print()` with `std::string::utf8()` wrapper
- ❌ INCORRECT: Using `std::debug::print(&b"...")` prints as hex codes

### 4. Documentation Updates ✅

**Updated `atomica/docs/smoke-test-debugging.md`**:
- Added comprehensive "Adding Move Debug Prints" section
- Documented correct vs incorrect debug print syntax
- Provided examples for common debugging scenarios
- Explained how debug prints appear in validator logs

### 5. Root Cause Analysis ✅

**Debug Log Analysis**:

Examined validator logs and found:

```
[debug] "[TIMELOCK] interval_micros="
[debug] 5000000                    ← Configured correctly
[debug] "[TIMELOCK] elapsed="
[debug] 5273284                    ← Time check working
[debug] "[TIMELOCK] should_rotate="
[debug] true                       ← ✅ Condition passes!
[debug] "[TIMELOCK] Calling perform_rotation"
[debug] "[TIMELOCK] Interval rotated to"
[debug] 1                          ← Advancing: 0→1→2→3...
```

**Event Stream Analysis**:

```bash
grep "Event\[0\] = V2" validator.log | sort -u
```

**Events Being Emitted**:
- ✅ `0x1::dkg::DKGStartEvent`
- ✅ `0x1::reconfiguration::NewEpoch`

**Events NOT Being Emitted** (despite code execution):
- ❌ `0x1::timelock::StartKeyGenEvent`
- ❌ `0x1::timelock::KeyPublishedEvent`
- ❌ `0x1::timelock::RequestRevealEvent` ← **Critical for secret revelation**

**Root Cause**: **Stale bytecode**
- Move source HAS `#[event]` attributes (confirmed in source)
- Bytecode compiled at 18:16 (6:16 PM)  - Test ran at 23:21 (11:21 PM)
- 5 hours of staleness
- Recent git commits added `#[event]` attributes after bytecode was compiled

### 6. Architecture Clarification ✅

**Corrected Understanding** (per user feedback):

**DKG**:
- Manages master key pairs for randomness beacon
- Master public key CAN rotate every epoch (validator set changes)
- Provides master key material for IBE derivation

**Timelock**:
- Does NOT rotate - advances through time intervals
- Uses IBE (Identity-Based Encryption) to derive keys from DKG master
- Interval number serves as IBE "identity"
- Public key storage is optional **convenience** for users

**Key Terminology**:
- ❌ "Timelock rotation" - INCORRECT
- ✅ "Interval advancement" - CORRECT
- ❌ `perform_rotation()` - Misleading function name
- ✅ Should be `advance_interval()`

**Timelock Flow**:
1. **Interval Start** (optional):
   - Pre-publish IBE-derived public key for interval N
   - Users can encrypt data for future revelation
   - Public key derivable by users from master key (storage is convenience)

2. **Interval Advancement**:
   - Time-based trigger when `interval_micros` elapsed
   - Emit `RequestRevealEvent` for OLD interval (ask for decryption key)
   - Increment counter: N → N+1
   - Emit `StartKeyGenEvent` for NEW interval (optional public key signal)

3. **Decryption Key Revelation**:
   - Validators receive `RequestRevealEvent`
   - Each validator submits IBE decryption key share
   - Threshold (2f+1) shares aggregated into full key
   - Emit `SecretRevealedEvent` with revealed key

## Fix Implementation

### Current Status: In Progress

**Step 1: Rebuild Cached Packages** ⏳
```bash
cargo build -p aptos-cached-packages
```

**Purpose**:
- Regenerate Move bytecode with current source code
- Ensure `#[event]` attributes are properly compiled
- Fresh bytecode will enable proper event emission

**Step 2: Re-run Test** (Pending)
```bash
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test_fixed.log
```

**Expected Results**:
- `RequestRevealEvent` should appear in event stream
- Validators should receive and process the event
- Secret revelation should complete within 60 seconds
- Test should PASS ✅

**Step 3: Verify Event Delivery** (Pending)
```bash
LOG_DIR=$(grep "Logs located" /tmp/test_fixed.log | tail -1 | awk '{print $NF}')
grep "RequestRevealEvent" $LOG_DIR/0/log
```

**Expected Output**:
```
[EventSub] Event[0] = V2:0x1::timelock::RequestRevealEvent
[DKG] Successfully parsed RequestRevealEvent for interval 2
[Timelock] Revealing share for interval 2
```

## Files Modified

### Move Source Code
- **aptos-move/framework/aptos-framework/sources/timelock.move**
  - Added comprehensive debug prints to `on_new_block()`
  - Added debug prints to `perform_rotation()`
  - No functional changes - debug only

### Documentation
- **atomica/docs/smoke-test-debugging.md**
  - Added "Adding Move Debug Prints" section
  - Documented correct `aptos_std::debug::print()` syntax
  - Provided debugging examples and workflows

- **timelock_event_emission_bug.md** (New)
  - Comprehensive root cause analysis
  - Architectural clarification
  - Debug workflow documentation
  - Fix implementation steps

- **timelock_debugging_summary.md** (This file)
  - Session summary and status

### Compilation Artifacts (Pending Rebuild)
- **aptos-move/framework/aptos-cached-packages** ⏳
  - Regenerating bytecode with current source

## Key Learnings

### Move Development

1. **Always rebuild cached packages after source changes**
   - Move source changes don't auto-rebuild bytecode
   - Tests use pre-compiled bytecode from cached-packages
   - Stale bytecode = stale behavior

2. **Debug print syntax matters**
   - ✅ `aptos_std::debug::print(&std::string::utf8(b"msg"))`
   - ❌ `std::debug::print(&b"msg")` (prints as hex)

3. **Event emission requires `#[event]` attribute**
   - Move v2 events need the attribute
   - Without it, events won't emit despite code execution

### Debugging Workflow

1. **Verify subscriptions** → Are listeners registered?
2. **Verify handlers** → Is code present to handle events?
3. **Add debug prints** → Is code actually executing?
4. **Check event stream** → Are events being emitted?
5. **Check bytecode age** → Is compiled code current?

### Architectural Understanding

- DKG = Infrastructure (master keys)
- Timelock = Product (time-based revelation)
- IBE = Bridge (derive interval-specific keys from master)
- Timelock doesn't "rotate" - it "advances intervals"
- Public key storage is user convenience, not required

## Next Steps

1. ⏳ **Wait for cached packages rebuild to complete**
2. 🔜 **Re-run test with fresh bytecode**
3. 🔜 **Verify `RequestRevealEvent` emission**
4. 🔜 **Verify validators process the event**
5. 🔜 **Confirm secret revelation works**
6. 🔜 **Test passes** ✅

## Timeline

- **18:16** - Original bytecode compiled (before `#[event]` attributes)
- **23:00** - Investigation started
- **23:05** - Event subscription verified
- **23:10** - Debug prints added
- **23:21** - Test run with debug logs
- **23:25** - Root cause identified (stale bytecode)
- **23:30** - Documentation updated
- **23:32** - Cached packages rebuild started
- **23:35** - Still building... (current)

## Estimated Completion

- Rebuild: ~5-10 minutes remaining
- Test run: ~2 minutes
- Verification: ~1 minute
- **Total**: ~10-15 minutes from now
