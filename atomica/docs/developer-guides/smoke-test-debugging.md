# Smoke Test Debugging Guide

## Key Points for Agents

**Read this section first when debugging smoke test failures.**

1. **Automatic rebuilding**: Smoke tests automatically rebuild `aptos-node` with your current code changes. You never need to manually run `cargo build` before running smoke tests.

2. **Find validator logs**: When a test fails, look for `Logs located at /tmp/.tmpXXXXXX` in the test output. This directory contains the actual validator logs - the test stdout only shows test framework messages.

3. **Log structure**: Validator logs are at `$LOG_DIR/0/log`, `$LOG_DIR/1/log`, etc. The last directory (e.g., `4/` for a 4-validator swarm) is typically a fullnode.

4. **Check for panics**: Run `grep -i "panic" $LOG_DIR/*/log` to find crashes. The panic message includes the file path and line number.

5. **Stale binaries**: If you see panics at line numbers that don't exist in the current source file, it means a stale binary is being used. This can happen if incremental compilation doesn't detect changes. Run `cargo clean -p <crate-name>` for the affected crate, then re-run the test.

6. **DKG/Timelock debugging**: Check `grep -E "\[DKG\]|\[Timelock\]" $LOG_DIR/0/log` for protocol-specific logs.

---

## Running Smoke Tests

```bash
# Run a specific test
cargo test -p smoke-test --lib test_name -- --test-threads=1 --nocapture

# Run all tests in a module
cargo test -p smoke-test --lib module_name -- --test-threads=1 --nocapture

# Save output to file
cargo test -p smoke-test --lib test_name -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.txt
```

**Required flags:**
- `--test-threads=1`: Prevents resource conflicts between tests that spawn validators
- `--nocapture`: Shows test output including the log directory path

**Build configuration (environment variables):**
- `LOCAL_SWARM_NODE_RELEASE=1`: Use release builds (slower to compile, faster to run)
- `FORGE_BUILD_WITHOUT_INDEXER=1`: Exclude indexer feature (faster compilation)

## Debugging Workflow

### 1. Run the test and find the log directory

```bash
cargo test -p smoke-test --lib my_test -- --test-threads=1 --nocapture 2>&1 | tee /tmp/test.txt

# Extract log directory
LOG_DIR=$(grep "Logs located at" /tmp/test.txt | tail -1 | awk '{print $NF}')
echo "Logs: $LOG_DIR"
```

### 2. Check for panics or errors

```bash
# Check all validators for panics
grep -i "panic" $LOG_DIR/*/log

# Check for errors
grep -i "error" $LOG_DIR/0/log | head -30
```

### 3. Inspect specific subsystems

```bash
# DKG events
grep "\[DKG\]" $LOG_DIR/0/log

# Timelock events
grep "\[Timelock\]" $LOG_DIR/0/log

# Event subscriptions
grep "subscribe_to_events" $LOG_DIR/0/log
```

### 4. Compare across validators

```bash
for i in 0 1 2 3; do
  echo "=== Validator $i ==="
  grep -i "pattern" $LOG_DIR/$i/log | head -10
done
```

## Common Issues

### Panic at non-existent line number

**Symptom:** Panic message references a line number that doesn't exist in the current file.

**Cause:** Stale compiled artifacts. Incremental compilation didn't detect your changes.

**Fix:**
```bash
cargo clean -p <crate-name>
# Then re-run the test
```

### Test timeout waiting for epoch/condition

**Symptom:** "Epoch N taking too long to arrive" or similar timeout message.

**Cause:** Validators crashed or stopped making progress.

**Debug:**
```bash
# Check if validators are still running
grep -i "panic\|fatal" $LOG_DIR/*/log

# Check last activity
tail -20 $LOG_DIR/0/log
```

### Connection refused

**Symptom:** "Connection refused (os error 111)" when test tries to connect to validator API.

**Cause:** Validator process crashed.

**Debug:**
```bash
# Find the crash
grep -B5 -A10 "panic" $LOG_DIR/0/log
```

## Adding Debug Output

### In Move code

```move
// String messages
aptos_std::debug::print(&std::string::utf8(b"[DEBUG] message"));

// Values
aptos_std::debug::print(&some_value);
```

Debug prints appear in validator logs as `[debug] "message"`.

**Note:** After modifying Move framework code, you may need to rebuild `aptos-cached-packages`.

### In Rust code

Use standard `info!`, `debug!`, or `error!` macros. They appear in validator logs with timestamps.

## Preserving Logs

Logs are deleted when the test ends. To preserve them:

```bash
# Copy while test is running (in another terminal)
cp -r /tmp/.tmpXXXXXX /tmp/saved_logs/

# Or add a sleep in the test code before cleanup
std::thread::sleep(std::time::Duration::from_secs(3600));
```

## Quick Reference

```bash
# Extract log directory from test output
LOG_DIR=$(grep "Logs located at" /tmp/test.txt | tail -1 | awk '{print $NF}')

# Find panics
grep -i "panic" $LOG_DIR/*/log

# Find errors
grep -E "ERROR|error" $LOG_DIR/0/log

# DKG activity
grep "\[DKG\]" $LOG_DIR/0/log

# Event processing
grep -E "Processing.*Event" $LOG_DIR/0/log

# Last lines from all validators
for i in 0 1 2 3; do echo "=== $i ==="; tail -5 $LOG_DIR/$i/log; done
```
