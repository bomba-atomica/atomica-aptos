# Debugging Plan for test_timelock_secret_revelation

## Test Workflow

```
test_timelock_secret_revelation:
  1. Create swarm (5-second intervals)
  2. Wait for interval rotation (N → N+1)
  3. Wait for public key published (DKG transcript)
  4. Wait for next rotation (N+1 → N+2) ← Should emit RequestRevealEvent(N+1)
  5. Wait 60 seconds for secret to be revealed ← FAILS HERE
```

## Expected vs Actual

### Expected Chain of Events

**On-Chain (Move):**
1. `perform_rotation()` increments interval N+1 → N+2
2. Emits `RequestRevealEvent { interval: N+1 }`
3. Waits for validators to submit shares

**Off-Chain (Rust - epoch_manager.rs):**
1. Event subscription active for `RequestRevealEvent`
2. Receives `RequestRevealEvent(N+1)` notification
3. Calls `process_timelock_reveal(event)`
4. Retrieves stored share from `persistent_safety_storage`
5. Creates `TimelockShare { interval, author, share }`
6. Submits `ValidatorTransaction::TimelockShare`

**On-Chain (Move - publish_secret_share):**
1. Validates share is valid G1 point
2. Stores in `validator_shares` table
3. When threshold met: aggregates shares
4. Stores in `revealed_secrets[interval]`
5. Emits `SecretRevealedEvent`

### Actual Behavior

**Known from documentation:**
- ❌ Secret is NEVER revealed
- ❌ Test times out after 60 seconds
- ❌ `TimelockRevelationManager` is a stub (does nothing)

## Debug Commands

### 1. Run Test and Save Output

```bash
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/timelock_test.log
```

### 2. Extract Log Directory

```bash
# From test output, find line like:
# "Logs located at /var/folders/.../T/.tmpXXXXXX"

LOG_DIR=$(grep "Logs located" /tmp/timelock_test.log | tail -1 | awk '{print $NF}')
echo "Validator logs: $LOG_DIR"
```

### 3. Check Event Subscriptions

```bash
# Verify validators subscribed to RequestRevealEvent
grep "subscribe_to_events" $LOG_DIR/0/log

# Expected output:
# [EventSub] subscribe_to_events called with ... ["RequestRevealEvent", ...]
```

### 4. Check Event Delivery

```bash
# Check if RequestRevealEvent was delivered to validators
grep -E "RequestReveal|process_timelock_reveal" $LOG_DIR/0/log

# Expected: Should see "Processing RequestRevealEvent for interval X"
# Actual: Probably NO output (event not processed)
```

### 5. Check for Revelation Attempts

```bash
# Check if validators tried to reveal shares
grep -E "TimelockShare|publish_secret_share|Revealing share" $LOG_DIR/0/log

# Expected: Should see share submission attempts
# Actual: Probably NO output (revelation not implemented)
```

### 6. Check Interval Rotations

```bash
# Verify intervals are actually rotating
grep -E "perform_rotation|Interval rotated|current_interval" $LOG_DIR/0/log

# Should see rotation happening
```

### 7. Check All Validators

```bash
# Check all validators (not just validator 0)
for i in 0 1 2; do
  echo "=== Validator $i ==="
  grep -c "RequestReveal" $LOG_DIR/$i/log || echo "No RequestReveal events"
done
```

## Known Issues

### Issue 1: TimelockRevelationManager is a Stub

**Location:** `dkg/src/timelock_revelation.rs`

**Current Code:**
```rust
pub struct TimelockRevelationManager {
    _my_addr: AccountAddress,
}

impl TimelockRevelationManager {
    pub fn new(my_addr: AccountAddress) -> Self {
        warn!("[TIMELOCK] TimelockRevelationManager created but not yet functional");
        warn!("[TIMELOCK] Secret revelation is NOT IMPLEMENTED - tests will fail");
        Self { _my_addr: my_addr }
    }

    pub fn maybe_reveal_for_past_interval(&mut self, _current_interval: u64) {
        // Placeholder - does nothing!
    }
}
```

**What's Missing:**
- No monitoring of `RequestRevealEvent`
- No retrieval of stored shares
- No creation of `TimelockShare` transactions
- No submission to validator transaction pool

### Issue 2: Event Processing Loop

**Location:** `dkg/src/epoch_manager.rs:175-234`

**Check:** Verify that `RequestRevealEvent` handling actually calls revelation logic

```rust
// Should see this pattern:
match RequestRevealEvent::try_from(&event) {
    Ok(timelock_reveal) => {
        info!("[DKG] Successfully parsed RequestRevealEvent for interval {}", ...);
        self.process_timelock_reveal(timelock_reveal);  // ← Does this work?
        continue;
    },
    ...
}
```

### Issue 3: process_timelock_reveal Implementation

**Location:** `dkg/src/epoch_manager.rs:646-713`

**Suspected Issues:**
1. Does it actually create the `TimelockShare`?
2. Does it submit to the validator transaction pool?
3. Does it handle errors properly?

## Verification Steps

### Step 1: Confirm Event Subscription Works

Run this grep on validator logs:
```bash
grep "subscribe_to_events" $LOG_DIR/0/log | grep RequestReveal
```

**Expected:** Should show `RequestRevealEvent` in subscribed events
**If Missing:** Event subscription is broken

### Step 2: Confirm Events Are Emitted

Check Move contract logs (may need to enable debug logging):
```bash
grep "emit.*RequestReveal\|RequestRevealEvent" $LOG_DIR/0/log
```

**Expected:** Should see event emission
**If Missing:** Move contract not emitting events

### Step 3: Confirm Events Are Delivered

```bash
grep "Received.*RequestReveal\|process_timelock_reveal called" $LOG_DIR/0/log
```

**Expected:** Should see event processing
**If Missing:** Event loop not reading from channel

### Step 4: Confirm Share Submission

```bash
grep "TimelockShare\|publish_secret_share" $LOG_DIR/0/log
```

**Expected:** Should see validator transaction submission
**If Missing:** Revelation logic not implemented

## Root Cause Hypothesis

Based on documentation (Issue 4 in smoke_test_errors.md):

**PRIMARY CAUSE:** `TimelockRevelationManager::maybe_reveal_for_past_interval()` is a no-op

**SECONDARY CAUSE:** Even if events are delivered, the revelation manager doesn't:
1. Listen to `RequestRevealEvent`
2. Retrieve stored shares
3. Submit `TimelockShare` transactions

**EVIDENCE:**
- Documentation explicitly states: "Secret revelation is NOT IMPLEMENTED"
- Warning logs say: "TimelockRevelationManager created but not yet functional"
- Code review shows placeholder implementation

## Next Steps

### Immediate: Verify Event Delivery

Run the test and check if `RequestRevealEvent` is even being delivered:

```bash
# Run test
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.log

# Get log dir
LOG_DIR=$(grep "Logs located" /tmp/test.log | tail -1 | awk '{print $NF}')

# Check event delivery
echo "=== Event Subscriptions ==="
grep "subscribe_to_events" $LOG_DIR/0/log

echo "=== Event Processing ==="
grep "RequestReveal" $LOG_DIR/0/log

echo "=== Revelation Attempts ==="
grep "TimelockShare\|Revealing" $LOG_DIR/0/log
```

### If Events Are Delivered

The bug is in the revelation manager - need to implement actual revelation logic.

### If Events Are NOT Delivered

The bug is in:
1. Event emission (Move contract)
2. Event subscription (Rust event loop)
3. Event routing (channel plumbing)

## Files to Inspect

1. **dkg/src/timelock_revelation.rs** - Revelation manager (stub)
2. **dkg/src/epoch_manager.rs:646-713** - process_timelock_reveal()
3. **dkg/src/epoch_manager.rs:175-234** - Event processing loop
4. **aptos-move/framework/aptos-framework/sources/timelock.move:199-203** - perform_rotation() event emission
5. **aptos-move/aptos-vm/src/validator_txns/timelock.rs** - VM transaction processing

## Success Criteria

Test passes when:
1. ✅ Validators subscribe to `RequestRevealEvent`
2. ✅ Move contract emits `RequestRevealEvent` on rotation
3. ✅ Validators receive event notification
4. ✅ Validators retrieve stored shares
5. ✅ Validators submit `TimelockShare` transactions
6. ✅ Move contract aggregates shares
7. ✅ `get_secret(interval)` returns aggregated key
8. ✅ Test sees secret within 60 seconds
