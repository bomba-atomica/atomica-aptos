# Timelock Tests Implementation Obstacles

This file tracks obstacles, issues, and blockers encountered during the implementation of TypeScript-based timelock and DKG tests using docker-test-harness.

## Current Issues

- **Test timeout**: Bun test times out after 5000ms when initializing docker testnet. Need to configure longer timeout or use different test runner.
- **File corruption during edits**: TypeScript files getting corrupted during edit operations, requiring recreation.
- **Docker daemon instability**: Docker daemon stops running intermittently, causing testnet initialization failures.
- **Automatic interval rotation**: Timelock on_new_block is not called in testnet, preventing automatic DKG rotation.
- **CI workflow YAML formatting**: GitHub Actions workflow has indentation issues that need to be fixed.

## Resolved Issues

- **Config path resolution**: Fixed docker-testnet/config path in findComposeDir to work from timelock-tests directory.
- **TypeScript API compatibility**: Resolved Aptos SDK import issues by using AptosClient/AptosAccount API instead of newer Aptos class.
- **Test runner timeout**: Switched to Bun runner with individual test scripts to avoid Jest ES module issues.
- **Docker testnet startup**: Verified that docker testnet initializes correctly with 2 validators, genesis generation, and consensus startup.
- **Resource querying**: Aptos client successfully queries blockchain state (timelock resources, timestamps, etc.).
- **Transaction submission**: Fixed by using `client.waitForTransactionWithResult()` instead of `client.waitForTransaction()`, and ensuring proper account funding.
- **IBE crypto stubs**: Implemented placeholder IBE cryptographic operations with proper API structure.
- **Test execution patterns**: Added round-robin and breadth-first test runners for different execution strategies.
- **Test infrastructure**: Created complete TypeScript test framework with helpers for transactions, queries, and waiters.

## Notes
