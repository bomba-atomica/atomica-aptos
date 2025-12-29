# GitHub Workflows

This directory contains CI/CD workflows for the Atomica Aptos project.

## Available Workflows

### 1. Build Aptos Binary (`build-aptos-binary.yml`)
Builds the Aptos node and CLI binaries for releases.

**Features:**
- Builds `aptos-node` and `aptos` CLI
- Creates GitHub releases with binaries
- Generates SHA256 checksums
- 90-day artifact retention

**Triggers:**
- Push tags matching `v*.*.*` (e.g., v1.0.0, v1.2.3)
- Push tags matching `release-*` (e.g., release-mainnet)
- Manual workflow dispatch

**Note:** Does NOT build on regular commits or pull requests to avoid unnecessary CI usage.

---

### 2. Build Validator Image (`build-validator-image.yml`)
Builds Docker images for Aptos validators.

**Triggers:**
- Push to main branches
- Pull requests
- Manual workflow dispatch

---

### 3. Timelock & DKG Tests (`test-timelock-dkg.yml`) ⭐ **NEW**
Focused test workflow for timelock and DKG implementations.

**Features:**
- 4 parallel test jobs for fast feedback
- Only runs when relevant files change
- Optimized for CI (disk space cleanup, caching)
- Summary job reports overall status

**Test Jobs:**
1. **Timelock Move Module Tests** - Tests the Move smart contract
2. **DKG Tests** - Tests the Distributed Key Generation crate
3. **Timelock Smoke Tests** - Integration tests
4. **Validator Transaction Tests** - Tests the Rust validator transaction handler

**Triggers:**
- Push to `main`, `dev-atomica`, or `implement-ibe-*` branches
- Changes to:
  - `aptos-move/framework/aptos-framework/sources/timelock*.move`
  - `aptos-move/aptos-vm/src/validator_txns/timelock.rs`
  - `crates/aptos-dkg/**`
  - `dkg/**`
  - `types/src/dkg/**`
  - `testsuite/smoke-test/src/timelock/**`
- Manual workflow dispatch

**Manual Trigger:**
```bash
# Via GitHub UI: Actions → Timelock & DKG Tests → Run workflow
```

**Local Testing:**
To run the same tests locally:

```bash
# Timelock Move module tests
cd aptos-move/framework/aptos-framework
cargo test -p aptos-framework --test move_unit_test -- timelock

# DKG tests
cargo test -p aptos-dkg --lib

# Timelock smoke tests
cargo test -p smoke-test --lib -- timelock

# Validator transaction timelock tests
cargo test -p aptos-vm --lib validator_txns::timelock
```

## CI Optimization

All workflows include:
- **Disk space cleanup** - Removes ~30GB of unnecessary software
- **Rust caching** - Speeds up subsequent builds
- **Parallel execution** - Runs independent tests concurrently
- **Path filters** - Only runs when relevant files change
- **Optimized builds** - `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`

## Adding New Workflows

When creating new workflows:
1. Use `ubuntu-latest` for standard builds (7GB RAM, 2 cores)
2. Include disk space cleanup for large builds
3. Use `Swatinem/rust-cache@v2` for caching
4. Set appropriate timeouts (60-150 minutes)
5. Add path filters to avoid unnecessary runs
6. Use descriptive job names and step names

## Troubleshooting

**Build fails with "No space left on device":**
- Disk cleanup step should handle this
- Consider using `ubuntu-latest-4-cores` (16GB RAM) for paid plan

**Tests timeout:**
- Check the `timeout-minutes` setting
- Consider splitting large test suites into parallel jobs

**Cache not working:**
- Verify `shared-key` is unique per workflow
- Check that `cache-on-failure: true` is set
- Ensure `workspaces: "."` points to root directory
