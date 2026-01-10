# Atomica Timelock Testing Guide

## Prerequisites

- **Bun runtime** (for TypeScript tests)
- **Docker** (for testnet containers)

---

## Test Commands

```bash
cd atomica/timelock-tests

# Install dependencies
bun install

# Run all tests
bun test

# Specific suites
bun run test:basic              # Core DKG flow
bun run test:rotation           # Manual rotation trigger
bun run test:ibe                # IBE encryption (WIP)
bun run test:framework-loading  # Custom framework verification
```

---

## Smoke Tests

Smoke tests provide integration testing for the Aptos validator environment with timelock functionality.

### Running Smoke Tests

```bash
# Run timelock smoke tests with output
cargo test -p smoke-test --lib timelock -- --nocapture --test-threads=1
```

**Flags explained:**

- `--nocapture` — Shows stdout logs during test execution
- `--test-threads=1` — Runs tests sequentially to avoid port conflicts

### Accessing Validator Logs

Smoke tests create ephemeral validator nodes with logs stored in temporary directories:

```bash
# Validator logs are written to:
<temp_path>/0/log          # Validator 0
<temp_path>/1/log          # Validator 1
<temp_path>/2/log          # Validator 2
<temp_path>/3/log          # Validator 3
```

**To read logs during/after test execution:**

```bash
# Find the temp directory from test output, then tail logs:
tail -f /tmp/.tmp<random>/0/log

# Or monitor all validators:
tail -f /tmp/.tmp<random>/*/log
```

> [!TIP]
> The temp path is printed in the test stdout. Look for lines indicating where the swarm is initialized.

---

## Framework Development Workflow

When modifying Move framework code, follow this loop:

```bash
# 1. Modify Move sources
vim aptos-move/framework/aptos-framework/sources/timelock.move

# 2. Rebuild framework artifact
./atomica/docker-test-harness/build-framework.sh

# 3. Run tests (uses custom head.mrb)
cd atomica/timelock-tests && bun run test:rotation
```

> [!WARNING]
> Do NOT create runtime scripts to change framework configs.
> Use custom genesis injection via `head.mrb` instead.

# Framework Unit Testing

To verify the Move framework changes independently:

```bash
# 1. Compile the framework source
aptos-framework release

# 2. Run Move unit tests
aptos move test --package-dir aptos-move/framework/aptos-framework
```

---

## Test Architecture

```
timelock-tests/
├── src/
│   ├── index.ts              # Exports
│   ├── queries.ts            # Blockchain queries
│   ├── transactions.ts       # Transaction builders
│   ├── waiters.ts            # Polling utilities
│   └── ibe-crypto.ts         # IBE cryptography
└── test/
    ├── basic-flow.test.ts    # Core DKG tests
    ├── ibe-e2e.test.ts       # IBE roundtrip
    └── manual-rotation.test.ts
```

**Key Components:**

- `docker-test-harness/` — Manages 4-validator Docker testnets
- `move-framework-fixtures/head.mrb` — Compiled framework for custom genesis

---

## Known Issues and Fixes

### DKG Transcript Deserialization Errors (FIXED)

**Symptom**: Validators log errors like:

- `[DKG] adding peer transcript failed with trx deserialization error: ULEB128 encoding was not minimal in size`
- `[DKG] adding peer transcript failed with trx deserialization error: unexpected end of input`

**Root Cause**: Both randomness DKG (RealDKG) and timelock DKG (IbeDKG) were running concurrently. Since they use different transcript formats, validators receiving transcripts from the "wrong" session would fail to deserialize them.

**Fix Applied**: Modified `timelock.move` to ensure timelock IBE DKG only starts AFTER randomness DKG completes:

- Added `dkg::incomplete_session()` check in `on_new_block()`
- Removed premature `StartKeyGenEvent` emission from `initialize()`
- Added unit test `test_mpk_dkg_flag_tracking()` to verify the sequencing flag

**Test Added**: `testsuite/smoke-test/src/timelock/test_dkg_sequencing.rs` verifies no deserialization errors occur.

**Commit**: See `aptos-move/framework/aptos-framework/sources/timelock.move:223-236`

---

## Troubleshooting

### Test Timeout

Increase timeout in `bun.config.ts` or add `--timeout 300000` flag.

### Framework Not Loading

Check that `head.mrb` exists and Docker volumes are correctly mounted.

### Docker Instability

Restart Docker daemon: `docker restart`

### Analyzing Validator Logs for DKG Issues

```bash
# Check for deserialization errors (should be none after fix)
grep "deserialization error" /tmp/.tmp<random>/*/log

# Check DKG session starts
grep "DKGManager started" /tmp/.tmp<random>/0/log

# Check transcript sizes (should be consistent per session)
grep "transcript_bytes_len" /tmp/.tmp<random>/0/log
```

---

## Test Coverage

### TypeScript Tests (Bun)

| Test                            | Status                                    |
| ------------------------------- | ----------------------------------------- |
| ✅ DKG transcript publication   | Passing (Basic)                           |
| ✅ Share reveal and aggregation | Passing (Basic)                           |
| ✅ Manual rotation trigger      | Passing                                   |
| ✅ Custom framework loading     | Passing                                   |
| ✅ IBE crypto vectors           | **Passing** (Real Crypto Fixed)           |
| ⚠️ IBE E2E Flow                 | **Timeout** (Validator DKG publication)   |
| ✅ Docker Faucet                | **Passing** (Fixed: Using SDK CoinClient) |
| ❌ invalid share rejection      | Not implemented                           |

### Smoke Tests (Rust)

| Test               | Status                                           |
| ------------------ | ------------------------------------------------ |
| ✅ DKG Sequencing  | **Passing** (Timelock waits for randomness)      |
| ✅ MPK Publication | **Passing** (MPK published after sequential DKG) |
| ✅ DKG Transcript  | **Passing** (No deserialization errors)          |
| ⚠️ Full E2E Flow   | **In Progress** (Some tests timeout)             |

Run all timelock smoke tests:

```bash
cargo test -p smoke-test --lib timelock -- --nocapture --test-threads=1
```

Run specific test:

```bash
cargo test -p smoke-test --lib timelock::test_dkg_sequencing -- --nocapture
```

See [development-and-verification.md](./development-and-verification.md) for roadmap.
