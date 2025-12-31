# Timelock Tests Implementation Obstacles

This file tracks obstacles, issues, and blockers encountered during the implementation of TypeScript-based timelock and DKG tests using docker-test-harness.

## Current Issues

- **Transaction submission failure**: AptosClient.submitTransaction fails with ECONNREFUSED, even though queries work. May be due to validator configuration or endpoint issues.
- **Interval rotation not triggering**: Timelock on_new_block logic is not causing interval rotation, possibly because block prologue is not calling timelock functions or timing issues.
- **File corruption during edits**: TypeScript files getting corrupted during edit operations, requiring recreation.
- **IBE cryptographic operations**: Need to implement or provide IBE encrypt/decrypt functions in TypeScript (compute_timelock_identity, ibe_encrypt, ibe_decrypt, deserialize_g1).

## Resolved Issues

- **Config path resolution**: Fixed docker-testnet/config path in findComposeDir to work from timelock-tests directory.
- **TypeScript API compatibility**: Resolved Aptos SDK import issues by using AptosClient/AptosAccount API instead of newer Aptos class.
- **Test runner timeout**: Switched to Bun runner with individual test scripts to avoid Jest ES module issues.
- **Docker testnet startup**: Verified that docker testnet initializes correctly with 2 validators, genesis generation, and consensus startup.
- **Resource querying**: Aptos client successfully queries blockchain state (timelock resources, timestamps, etc.).

## Notes
