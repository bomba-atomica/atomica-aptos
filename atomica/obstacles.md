# Timelock Tests Implementation Obstacles

This file tracks obstacles, issues, and blockers encountered during the implementation of TypeScript-based timelock and DKG tests using docker-test-harness.

## Current Issues

- **Test timeout**: Bun test times out after 5000ms when initializing docker testnet. Need to configure longer timeout or use different test runner.
- **File corruption during edits**: TypeScript files getting corrupted during edit operations, requiring recreation.
- **Docker daemon instability**: Docker daemon stops running intermittently, causing testnet initialization failures.
- **Framework rebuild required**: New manual rotation function in Move code needs framework rebuild to be available.
- **Automatic interval rotation**: Timelock on_new_block is not called in testnet, preventing automatic DKG rotation.
- **CI workflow YAML formatting**: GitHub Actions workflow has indentation issues that need to be fixed.
- **Framework compilation time**: `compileAndPlaceFramework()` takes 10+ minutes to compile entire Rust codebase.

## Framework Compilation Solutions

### Option 1: Pre-compiled Framework (Recommended)

Instead of compiling from scratch each time, use a pre-compiled framework:

- Copy existing `head.mrb` from CI artifacts or local builds
- Download from GitHub releases if available
- Use cached framework builds

### Option 2: Incremental Compilation

- Use `cargo build --release` instead of `cargo run` for faster builds
- Cache Rust dependencies between builds
- Only rebuild when Move code actually changes

### Option 3: CI-Only Compilation

- Only run full compilation in CI environments
- Use pre-built frameworks for local development
- Provide fallback to existing framework files

## Investigation Notes

### Automatic Rotation (on_new_block)

The Move code correctly calls `timelock::on_new_block(vm)` in the block prologue (`block.move`). However, in the docker testnet:

1. **Block frequency**: Blocks may not be produced frequently enough to trigger rotation within test timeouts
2. **Prologue execution**: The VM block prologue might not be executing the timelock logic
3. **Time advancement**: Testnet timestamp may not advance at real-time rates

**Current status**: Automatic rotation is not working in testnet environment. This appears to be a deeper infrastructure issue with block prologue execution or block timing.

**Recommended approach**: Use manual rotation trigger as the primary testing mechanism for DKG flows. Automatic rotation can be addressed separately as an infrastructure improvement.

### Manual Rotation Implementation

✅ **Completed**: Added `trigger_rotation()` public function that:

- Checks if sufficient time has passed since last rotation
- Performs the same rotation logic as `on_new_block`
- Can be called by any account after the scheduled time
- Includes proper error handling for early triggers

**Next steps for automatic rotation**:

- Investigate block production frequency in docker testnet
- Check if VM block prologue is executing timelock functions
- Consider testnet configuration changes for faster block production
- Implement time advancement utilities for testing

**Testing approach**: Use manual rotation for reliable DKG flow testing, with automatic rotation as a future enhancement.

## Resolved Issues

- **Config path resolution**: Fixed docker-testnet/config path in findComposeDir to work from timelock-tests directory.
- **TypeScript API compatibility**: Resolved Aptos SDK import issues by using AptosClient/AptosAccount API instead of newer Aptos class.
- **Test runner timeout**: Switched to Bun runner with individual test scripts to avoid Jest ES module issues.
- **Docker testnet startup**: Verified that docker testnet initializes correctly with 2 validators, genesis generation, and consensus startup.
- **Resource querying**: Aptos client successfully queries blockchain state (timelock resources, timestamps, etc.).
- **Transaction submission**: Fixed by using `client.waitForTransactionWithResult()` instead of `client.waitForTransaction()`, and ensuring proper account funding.
- **IBE crypto stubs**: Implemented placeholder IBE cryptographic operations with proper API structure.
- **Test execution patterns**: Added round-robin and breadth-first test runners for different execution strategies.
- **Framework compilation**: Added `compileAndPlaceFramework()` helper and test with Noop contract to verify fresh .mrb loading.
- **Test infrastructure**: Created complete TypeScript test framework with helpers for transactions, queries, and waiters.

## Notes
