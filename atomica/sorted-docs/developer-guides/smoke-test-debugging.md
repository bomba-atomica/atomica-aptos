# Smoke Test Debugging Guide

## Running Smoke Tests

### Basic Test Execution

```bash
# Run all timelock tests with single thread (prevents resource conflicts)
cargo test -p smoke-test --lib timelock -- --test-threads=1 --nocapture

# Run a specific test
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation -- --test-threads=1 --nocapture

# Run with output saved to file
cargo test -p smoke-test --lib timelock -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test_output.txt
```

### Important Flags

- `--test-threads=1`: Sequential execution (critical for smoke tests that start local validators)
- `--nocapture`: Shows stdout/stderr from tests (includes test framework logs)
- `--lib`: Run library tests only (not integration tests)
- `2>&1 | tee file.txt`: Capture all output to both terminal and file

## Understanding Test Output

### Test stdout vs Validator Logs

**Critical distinction:** The test stdout (what you see in terminal) is NOT the same as validator node logs!

```
Test stdout:
├─ Test framework messages (starting swarm, waiting for conditions)
├─ Test assertions and debug output
└─ "Logs located at /path/to/temp/dir" ← THIS IS THE KEY!

Validator logs:
└─ /path/to/temp/dir/0/log  ← Validator 0's actual runtime logs
└─ /path/to/temp/dir/1/log  ← Validator 1's actual runtime logs
└─ /path/to/temp/dir/N/log  ← Validator N's actual runtime logs
```

### Finding Validator Logs

When a test fails, look for this line:
```
Logs located at /var/folders/.../T/.tmpXXXXXX
```

This directory contains the actual validator node logs where you'll find:
- Event processing logs
- DKG operations
- Timelock revelation attempts
- Error messages from runtime components

## Reading Validator Logs

### Log Directory Structure

```
/tmp/test_logs/
├── 0/           # Validator 0
│   ├── log      # Main log file (this is what you want!)
│   ├── node.yaml
│   └── ...
├── 1/           # Validator 1
│   ├── log
│   └── ...
├── N/           # Validator N (last validator)
└── N+1/         # Fullnode (if present)
```

### Essential Log Patterns

```bash
# Check for timelock event subscriptions
grep -i "subscribe_to_events\|KeyPublished\|RequestReveal" /path/to/logs/0/log

# Check for DKG event processing
grep -E "\[DKG\]|\[Timelock\]" /path/to/logs/0/log

# Check for errors
grep -i "error\|failed\|panic" /path/to/logs/0/log

# Check event notification delivery
grep "EventNotification\|process_event" /path/to/logs/0/log

# Tail logs in real-time (while test is running)
tail -f /path/to/logs/0/log | grep -E "Timelock|DKG"
```

### Key Log Messages to Look For

**Event Subscription (should appear early):**
```
[EventSub] subscribe_to_events called with 0 event_keys, 4 v2_tags:
  ["0x1::dkg::DKGStartEvent", "0x1::timelock::StartKeyGenEvent",
   "0x1::timelock::KeyPublishedEvent", "0x1::timelock::RequestRevealEvent"]
```

**DKG Processing (should appear during test):**
```
[DKG] Successfully parsed DKGStartEvent
[DKG] Processing DKGStart event
[DKG] Deal transcript finished
```

**Timelock Event Processing (what we're debugging):**
```
[Timelock] Processing KeyPublishedEvent for interval N
[Timelock] Storing secret share for interval N
[Timelock] Revealing share for interval N
[DKG] DEBUG: process_timelock_reveal called for interval N
```

**Missing logs indicate where the problem is!**

### Adding Move Debug Prints

When debugging Move contract logic (like `timelock.move` or `dkg.move`), you can add debug prints that will appear in validator logs:

**✅ CORRECT - Use `aptos_std::debug::print()` with `std::string::utf8()`:**

```move
// For string messages
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));

// For numeric values (u64, u128, etc.)
aptos_std::debug::print(&interval_micros);

// For booleans
aptos_std::debug::print(&(elapsed > interval_micros));
```

**❌ INCORRECT - Don't use `std::debug::print()` with bare byte strings:**

```move
// This will print as hex codes (0x5b54494d454c4f434b5d...), not readable strings!
std::debug::print(&b"[TIMELOCK] message");
```

**How debug prints appear in validator logs:**

```bash
# After adding prints to Move code, recompile and run the test
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.txt

# Extract log directory
LOG_DIR=$(grep "Logs located" /tmp/test.txt | tail -1 | awk '{print $NF}')

# Check for your debug prints (they appear in [debug] lines)
grep -i "timelock.*on_new_block\|timelock.*interval" $LOG_DIR/0/log

# Example output:
# [debug] "[TIMELOCK] on_new_block called"
# [debug] "[TIMELOCK] on_new_block: current_interval="
# [debug] 2
# [debug] "[TIMELOCK] on_new_block: now="
# [debug] 1736116545378113
```

**Common debugging scenarios:**

```move
// Check if function is being called
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));

// Check timing and intervals
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] interval_micros="));
aptos_std::debug::print(&interval_micros);
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] elapsed="));
aptos_std::debug::print(&elapsed);

// Check boolean conditions
aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] should_rotate="));
aptos_std::debug::print(&(elapsed > interval_micros));

// Check state existence
if (!exists<TimelockState>(@aptos_framework)) {
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] TimelockState does not exist!"));
    return
};
```

**Important notes:**
- Debug prints must be added **before** compiling the Move code into the genesis/framework packages
- Changes to `aptos-move/framework/*/sources/*.move` require recompiling `aptos-cached-packages`
- Debug prints appear under `[debug]` log level in validator logs
- String messages should be wrapped with `std::string::utf8()` to display as readable text

## Debugging Workflow

### Step 1: Run Test and Capture Logs

```bash
# Start test in background, save output
cargo test -p smoke-test --lib timelock::test_timelock::test_timelock_secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.txt &

# Watch for completion
tail -f /tmp/test.txt
```

### Step 2: Extract Log Directory

```bash
# When test fails, grab the log path
LOG_DIR=$(grep "Logs located at" /tmp/test.txt | tail -1 | awk '{print $NF}')
echo "Validator logs: $LOG_DIR"

# Verify logs exist
ls -la $LOG_DIR/
```

### Step 3: Inspect Validator Logs

```bash
# Check what validators actually saw
for i in 0 1 2 3; do
  echo "=== Validator $i ==="
  grep -i "timelock\|KeyPublished\|RequestReveal" $LOG_DIR/$i/log | head -20
done
```

### Step 4: Compare Test Expectations vs Reality

**Test stdout says:**
```
Waiting for secret to be revealed for interval 1
(timeout after 60 attempts)
```

**Validator logs show:**
```
[EventSub] Subscribed to RequestRevealEvent  ✓
(but no "Processing RequestRevealEvent" messages!)  ✗
```

**Conclusion:** Events are subscribed but not being processed.

## Common Issues and Solutions

### Issue: "Test times out waiting for condition"

**Check:**
1. Is the condition actually happening on-chain?
2. Are validators processing events?
3. Are there errors in validator logs?

**Debug:**
```bash
# Check if events are being delivered
grep "EventNotification\|process_event" $LOG_DIR/0/log

# Check if event handlers are being called
grep "process_timelock\|Revealing share" $LOG_DIR/0/log
```

### Issue: "Events subscribed but not processed"

**Symptoms:**
- Log shows: `subscribe_to_events called with [..., "0x1::timelock::KeyPublishedEvent", ...]`
- But no: `Processing KeyPublishedEvent` messages

**Root cause:** Event subscription is separate from event processing. The event loop must actually read from the subscribed channels.

**Fix location:** Check the `tokio::select!` event loop in the component's main task.

### Issue: "Logs directory cleaned up before inspection"

**Solution:** Add a pause before test cleanup:

```rust
#[tokio::test]
async fn my_test() {
    let swarm = build_swarm().await;

    // ... test logic ...

    // DEBUGGING: Keep logs available
    println!("Logs at: {}", swarm.logs_location());
    std::thread::sleep(std::time::Duration::from_secs(3600)); // 1 hour
}
```

Or copy logs before test ends:
```bash
# In another terminal while test is running
cp -r /var/folders/.../T/.tmpXXXXXX /tmp/saved_logs/
```

## Quick Reference

### Test Execution
```bash
# Single test, save output
cargo test -p smoke-test --lib timelock::TEST_NAME -- --test-threads=1 --nocapture > /tmp/out.txt 2>&1

# All timelock tests
cargo test -p smoke-test --lib timelock -- --test-threads=1 --nocapture

# With specific number of validators (modify test code)
# See testsuite/smoke-test/src/timelock/test_helpers.rs
```

### Log Analysis
```bash
# Get log directory from test output
grep "Logs located at" /tmp/test_output.txt

# Check all validators for specific pattern
for log in /path/to/logs/*/log; do
  echo "=== $log ==="
  grep -i "pattern" "$log"
done

# Count occurrences across all validators
grep -rh "pattern" /path/to/logs/*/log | wc -l

# Extract timeline of events
grep -h "Timelock\|DKG" /path/to/logs/0/log | grep -v DEBUG | sort
```

### Common Grep Patterns
```bash
# Event subscriptions
grep "subscribe_to_events" $LOG_DIR/0/log

# Event processing
grep -E "Processing.*Event|process_timelock|process_event" $LOG_DIR/0/log

# Errors and warnings
grep -E "ERROR|WARN" $LOG_DIR/0/log | grep -i timelock

# Specific event types
grep -E "KeyPublished|RequestReveal|SecretRevealed" $LOG_DIR/0/log

# DKG operations
grep "\[DKG\]" $LOG_DIR/0/log

# Timelock operations
grep "\[Timelock\]" $LOG_DIR/0/log
```

## Example Debugging Session

```bash
# 1. Run test
cargo test -p smoke-test --lib timelock::test_timelock::secret_revelation \
  -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.txt

# 2. Test fails - extract log location
LOG_DIR=$(grep "Logs located" /tmp/test.txt | tail -1 | awk '{print $NF}')

# 3. Check if events are subscribed
grep "subscribe_to_events" $LOG_DIR/0/log
# Output: subscribe_to_events called with ... ["KeyPublishedEvent", "RequestRevealEvent"]
# ✓ Events are subscribed

# 4. Check if events are being processed
grep "Processing.*Event" $LOG_DIR/0/log
# Output: Processing DKGStartEvent (only!)
# ✗ Timelock events not being processed

# 5. Check event loop
grep "Received.*notification" $LOG_DIR/0/log
# Output: Received dkg_start_events notification (only!)
# ✗ No timelock event notifications

# 6. Conclusion: Event loop doesn't listen to timelock event channel
# → Check tokio::select! in epoch_manager.rs start() method
```

## Tips

1. **Always use `--test-threads=1`** for smoke tests (they create real validator processes)
2. **Always check validator logs, not just test stdout** (test stdout only shows test framework messages)
3. **Look for "Logs located at" message** to find actual validator logs
4. **Events subscribed ≠ events processed** (subscription is separate from consumption)
5. **Copy log directories before they're cleaned up** (tests delete temp dirs on exit)
6. **Use grep with context** (`-B 3 -A 3`) to see surrounding lines
7. **Check all validators, not just validator 0** (some issues only appear on specific nodes)
8. **Compare timestamps** between test stdout and validator logs to understand timing

## Resources

- Test helpers: `testsuite/smoke-test/src/timelock/test_helpers.rs`
- Swarm setup: `testsuite/forge/src/backend/local/swarm.rs`
- Event subscriptions: `aptos-node/src/state_sync.rs`
- Event processing: `dkg/src/epoch_manager.rs`
