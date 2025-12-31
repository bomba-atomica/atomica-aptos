# Timelock Tests Implementation Obstacles

This file tracks obstacles, issues, and blockers encountered during the implementation of TypeScript-based timelock and DKG tests using docker-test-harness.

## Current Issues

- **Interval rotation not triggering**: Timelock interval rotation doesn't occur automatically in the testnet. Need to investigate why the timelock rotation logic isn't running.
- **File corruption during edits**: TypeScript files getting corrupted during edit operations, requiring recreation.
- **IBE cryptographic operations**: Need to implement or provide IBE encrypt/decrypt functions in TypeScript (compute_timelock_identity, ibe_encrypt, ibe_decrypt, deserialize_g1).

## Resolved Issues

- **Config path resolution**: Fixed docker-testnet/config path in findComposeDir to work from timelock-tests directory.
- **TypeScript API compatibility**: Resolved Aptos SDK import issues by using AptosClient/AptosAccount API instead of newer Aptos class.
- **Test runner timeout**: Switched to Bun runner with individual test scripts to avoid Jest ES module issues.
- **Docker testnet startup**: Verified that docker testnet initializes correctly with 4 validators, genesis generation, and consensus startup.

## Notes
