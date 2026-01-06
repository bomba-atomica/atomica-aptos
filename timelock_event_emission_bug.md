# Timelock Secret Revelation Bug - Root Cause Analysis

**Date**: 2026-01-05
**Test**: `test_timelock_secret_revelation`
**Status**: Event emission failure - validators never receive `RequestRevealEvent`

## Architecture Clarification

### DKG (Distributed Key Generation)
- **Purpose**: Manages master key pairs for the randomness beacon
- **Rotation**: Master public key **can rotate** every **epoch** (when validator set changes via reconfiguration)
- **Scope**: Infrastructure providing master key material
- **Output**: Master key pair used as input to IBE key derivation

### Timelock (IBE-Based Time-Lock Encryption)
- **Purpose**: Consensus-based revelation of IBE-derived decryption keys for specific time intervals
- **No Rotation**: Timelock does **NOT rotate** anything - it simply **advances through time intervals**
- **Mechanism**:
  - Uses Identity-Based Encryption (IBE) with the DKG master key as the master secret
  - Each time interval gets a unique IBE-derived decryption key
  - The interval number serves as the IBE "identity"

**Key Terminology**:
- ❌ **"Timelock Rotation"** - INCORRECT terminology, timelock doesn't rotate
- ✅ **"Interval Advancement"** - CORRECT, timelock advances to the next interval
- ❌ `perform_rotation()` - Misleading function name (legacy naming)
- ✅ Should be `advance_interval()` or `start_next_interval()`

**Timelock Interval Flow**:

1. **Interval Start** (optional convenience for users):
   - Validators may pre-compute and publish the IBE-derived **public key** for interval N
   - This allows users to encrypt data destined for interval N **before** that interval arrives
   - Storage of public key is purely a **convenience** - users could derive it themselves from the DKG master public key
   - Public keys are queryable via on-chain API: `timelock::get_public_key(interval)`

2. **Interval Advancement** (time-based trigger):
   - When `interval_micros` time has elapsed since last interval start
   - `on_new_block()` calls `perform_rotation()` (misnamed - should be `advance_interval()`)
   - This function:
     - Emits `RequestRevealEvent` for the **OLD interval** (N-1) - asking validators to reveal decryption key
     - Increments interval counter: `current_interval = N → N+1`
     - Emits `StartKeyGenEvent` for the **NEW interval** (N) - optional signal for public key derivation

3. **Decryption Key Revelation** (consensus protocol):
   - Validators receive `RequestRevealEvent` for interval N
   - Each validator:
     - Retrieves their IBE secret key share for interval N (derived from DKG master secret)
     - Computes their share of the IBE-derived decryption key for interval N
     - Submits share via `TimelockShare` validator transaction
   - Consensus layer:
     - Collects shares from validators
     - When threshold (2f+1 of 3f+1) shares received: aggregates into full decryption key
     - Stores decryption key on-chain
     - Emits `SecretRevealedEvent` with the revealed decryption key
   - Users can now:
     - Query decryption key via API: `timelock::get_secret(interval)`
     - Decrypt any data that was encrypted for interval N

## Bug Discovery Process

### Initial Symptoms
```bash
Test: test_timelock_secret_revelation
Expected: Secret revealed after 60 seconds
Actual: Test times out - secret never revealed
```

### Investigation Steps

**Step 1: Verify Event Subscription**
```rust
// aptos-node/src/state_sync.rs:96-103
let dkg_start_events = event_subscription_service
    .subscribe_to_events(vec![], vec![
        "0x1::dkg::DKGStartEvent".to_string(),
        "0x1::timelock::StartKeyGenEvent".to_string(),
        "0x1::timelock::KeyPublishedEvent".to_string(),
        "0x1::timelock::RequestRevealEvent".to_string(),  // ✅ Subscribed
    ])
```
✅ **Result**: Validators ARE correctly subscribed to timelock events

**Step 2: Verify Event Handler**
```rust
// dkg/src/epoch_manager.rs:221-234
match RequestRevealEvent::try_from(&event) {
    Ok(timelock_reveal) => {
        info!("[DKG] Successfully parsed RequestRevealEvent for interval {}",
              timelock_reveal.interval);
        self.process_timelock_reveal(timelock_reveal);  // ✅ Handler exists
        continue;
    },
```
✅ **Result**: Event processing loop IS configured to handle `RequestRevealEvent`

**Step 3: Verify Revelation Implementation**
```rust
// dkg/src/epoch_manager.rs:759-827
fn process_timelock_reveal(&self, event: RequestRevealEvent) {
    info!("[Timelock] Revealing share for interval {}", event.interval);
    // 1. Retrieve secret share from storage
    // 2. Deserialize the secret key shares
    // 3. Extract G1 point from first share
    // 4. Create and submit TimelockShare transaction
    // ✅ IMPLEMENTATION EXISTS (contrary to documentation)
}
```
✅ **Result**: `process_timelock_reveal()` IS implemented (documentation was outdated)

**Step 4: Add Move Debug Prints**

Added comprehensive debug logging to `timelock.move::on_new_block()`:

```move
public(friend) fun on_new_block(vm: &signer) acquires TimelockState {
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));

    // ... timing checks ...

    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] interval_micros="));
    aptos_std::debug::print(&interval_micros);
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] elapsed="));
    aptos_std::debug::print(&elapsed);
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] should_rotate="));
    aptos_std::debug::print(&(elapsed > interval_micros));

    if (now - state.last_rotation_time > interval_micros) {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Calling perform_rotation"));
        perform_rotation(state);
    }
}
```

**Step 5: Analyze Debug Logs**

Validator logs (`/var/folders/.../T/.tmpfqVh4R/0/log`) revealed:

```
[debug] "[TIMELOCK] on_new_block called"
[debug] "[TIMELOCK] interval_micros="
[debug] 5000000                              ← 5 seconds configured correctly
[debug] "[TIMELOCK] elapsed="
[debug] 5273284                              ← More than 5 seconds elapsed
[debug] "[TIMELOCK] should_rotate="          ← Legacy naming, means "should advance"
[debug] true                                 ← ✅ Time check PASSES
[debug] "[TIMELOCK] Calling perform_rotation" ← Legacy naming, means "advance_interval"
[debug] "[TIMELOCK] Interval rotated to"     ← Legacy naming, means "advanced to"
[debug] 1                                    ← Intervals advancing: 1, 2, 3...
```

✅ **Result**:
- Time-based interval advancement IS working correctly
- `perform_rotation()` (legacy name - should be `advance_interval()`) IS being called
- Interval counter IS incrementing correctly (0 → 1 → 2 → 3...)
- Terminology note: "rotation" in the code means "interval advancement"

**Step 6: Check Event Stream**

Checked all events being emitted:

```bash
grep "Event\[0\] = V2" validator.log | sort -u
```

**Events ACTUALLY emitted**:
- ✅ `0x1::dkg::DKGStartEvent` - Working
- ✅ `0x1::reconfiguration::NewEpoch` - Working

**Events MISSING** (never appear):
- ❌ `0x1::timelock::StartKeyGenEvent` - Never emitted
- ❌ `0x1::timelock::KeyPublishedEvent` - Never emitted
- ❌ `0x1::timelock::RequestRevealEvent` - **Never emitted** ← CRITICAL BUG

## Root Cause

### Why Events Aren't Being Emitted

Even though `perform_rotation()` is called and debug logs confirm execution:

```move
// NOTE: Function name is misleading - this advances to next interval, doesn't "rotate"
fun perform_rotation(state: &mut TimelockState) {
    let now = timestamp::now_microseconds();
    let old_interval = state.current_interval;

    // Emit event asking validators to reveal decryption key for the OLD interval
    event::emit(RequestRevealEvent {
        interval: old_interval,  // e.g., "reveal key for interval 2"
    });

    // Advance to next interval
    state.current_interval = state.current_interval + 1;

    // Emit event for NEW interval (optional - validators can derive public key themselves)
    event::emit(StartKeyGenEvent {
        interval: state.current_interval,  // e.g., "starting interval 3"
        config,
    });
}
```

**The events are NOT appearing in the event stream despite `perform_rotation()` being called.**

### Investigation of Event Definitions

Checked if `#[event]` attribute is present:

```move
// timelock.move:66-84

#[event]                                      // ✅ Present!
struct StartKeyGenEvent has drop, store {
    interval: u64,
    config: TimelockConfig,
}

#[event]                                      // ✅ Present!
struct KeyPublishedEvent has drop, store {
    interval: u64,
    public_key: vector<u8>,
}

#[event]                                      // ✅ Present!
struct RequestRevealEvent has drop, store {
    interval: u64,
}
```

**Confirmed**: All events have the `#[event]` attribute in source code.

### Bytecode Timestamp Analysis

```bash
ls -la aptos-move/framework/aptos-framework/build/AptosFramework/bytecode_modules/timelock.mv
# -rw-r--r--  1 lucas  staff  4797 Jan  5 18:16 timelock.mv
```

**Test runtime**: 23:21 (11:21 PM)
**Bytecode compiled**: 18:16 (6:16 PM)
**Time difference**: 5 hours old!

### Git History Analysis

```bash
git log --oneline --since="2 days ago" -- aptos-move/framework/aptos-framework/sources/timelock.move

3d4b060483 patches move early exit when DK should be revealed
b5af7d97de maybe patches secret reveal
3ef4509ec3 debug(timelock): Add logging to diagnose secret revelation issue
dc79b5b49b fix(timelock): Publish DKG transcript to timelock module on completion
```

**Conclusion**: The `#[event]` attributes were added in recent commits (likely `3ef4509ec3` or earlier), but the compiled bytecode is from BEFORE those changes.

## Root Cause: Stale Bytecode

The test is using **outdated bytecode** compiled before the `#[event]` attributes were added to timelock events.

**Evidence**:
1. Source code HAS `#[event]` attributes ✅
2. Debug logs show functions ARE being called ✅
3. Events DO NOT appear in event stream ❌
4. Bytecode timestamp is 5 hours old ❌
5. Recent git commits show event-related changes ✅

**What's happening**:
- The Move source has been updated with proper `#[event]` attributes
- The cached bytecode in `aptos-cached-packages` is stale
- Tests compile `aptos-node` (Rust) but use pre-compiled Move bytecode
- The old bytecode doesn't properly emit events through the event notification system

## Fix

Rebuild `aptos-cached-packages` to regenerate Move bytecode with current source:

```bash
cargo build -p aptos-cached-packages
```

This will:
1. Recompile all Move framework modules
2. Generate fresh bytecode with proper event emission
3. Package the bytecode into `aptos-cached-packages`
4. Next test run will use updated bytecode

## Testing the Fix

After rebuilding, re-run the test and verify:

```bash
# Run test
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.log

# Extract log directory
LOG_DIR=$(grep "Logs located" /tmp/test.log | tail -1 | awk '{print $NF}')

# Verify RequestRevealEvent is now emitted
grep "RequestRevealEvent" $LOG_DIR/0/log
```

**Expected output**:
```
[EventSub] Event[0] = V2:0x1::timelock::RequestRevealEvent
[DKG] Successfully parsed RequestRevealEvent for interval 2
[Timelock] Revealing share for interval 2
```

## Lessons Learned

### Move Development Best Practices

1. **Always rebuild cached packages after Move source changes**
   ```bash
   # After editing Move files
   cargo build -p aptos-cached-packages
   ```

2. **Check bytecode timestamps when events don't work**
   ```bash
   ls -la aptos-move/framework/*/build/*/bytecode_modules/
   ```

3. **Use debug prints to trace execution**
   ```move
   aptos_std::debug::print(&std::string::utf8(b"Message"));  // ✅ Readable
   std::debug::print(&b"Message");                            // ❌ Prints as hex
   ```

4. **Verify event attributes are present**
   ```move
   #[event]  // Required for Move v2 event emission
   struct MyEvent has drop, store {
       field: u64,
   }
   ```

### Debug Workflow

1. **Check event subscriptions** - Are validators listening?
2. **Check event handlers** - Are handlers implemented?
3. **Add debug prints** - Is code being executed?
4. **Check event stream** - Are events actually emitted?
5. **Check bytecode age** - Is compiled code current?

## Related Issues

- Documentation incorrectly stated `process_timelock_reveal()` was "NOT IMPLEMENTED"
- Function naming: `perform_rotation()` is misleading - should be `advance_interval()`
- Test documentation needed updates on how to debug smoke tests with Move debug prints

## Status

**Current**: Rebuilding `aptos-cached-packages`
**Next**: Re-run test to verify `RequestRevealEvent` emission and secret revelation
