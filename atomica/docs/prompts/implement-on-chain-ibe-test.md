# Implement On-Chain IBE Encryption Test

## Context

The IBE native function `reconstruct_ibe_dk_internal` has been implemented in:

- `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`

The Move wrapper is available in:

- `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move`

Existing smoke tests in `testsuite/smoke-test/src/ibe/mod.rs` test the Rust implementation but not the on-chain native function.

## Test Goal

Implement an on-chain IBE encryption test that:

1. Uses the Move native function `ibe::reconstruct_ibe_dk_internal`
2. Verifies that G1 point aggregation works correctly on-chain
3. Tests a complete encrypt/decrypt flow using on-chain DK

## Existing Infrastructure

### Move Module (`aptos_std::ibe`)

```move
public fun reconstruct_ibe_dk_internal<G1>(
    validator_indices: vector<u64>,
    dk_shares: vector<crypto_algebra::Element<G1>>,
    weights: vector<u64>,
    threshold: u64,
    total_weight: u64,
): crypto_algebra::Element<G1>
```

### Existing Smoke Tests (`testsuite/smoke-test/src/ibe/mod.rs`)

The existing tests use the Rust implementation directly, not the Move native function.

## What This Test Should Do

### Test 1: On-Chain DK Reconstruction

Create a test that:

1. Creates G1 elements representing DK shares
2. Calls `ibe::reconstruct_ibe_dk_internal` on-chain
3. Verifies the reconstructed DK matches expected value

### Test 2: Complete IBE Flow (requires MPK on-chain)

This test requires MPK to be published on-chain first. It should:

1. Get MPK from on-chain config
2. Compute identity for a timelock
3. Encrypt message using IBE (off-chain or via native)
4. Submit DK shares on-chain
5. Reconstruct DK using `ibe::reconstruct_ibe_dk_internal`
6. Decrypt the message

## Implementation Steps

### Step 1: Create Test File

Create `testsuite/smoke-test/src/ibe/on_chain.rs`:

```rust
use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use rand::SeedableRng;

// Test on-chain DK reconstruction using the native function
#[tokio::test]
async fn test_on_chain_dk_reconstruction() {
    // Setup: Create a local blockchain with 4 validators
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let mut info = swarm.aptos_public_info();

    // Test that we can call the ibe module functions
    // This requires the native function to be registered
}
```

### Step 2: Test G1 Point Operations

```rust
// Test basic G1 point operations via crypto_algebra
#[tokio::test]
async fn test_g1_point_operations() {
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let mut info = swarm.aptos_public_info();

    // Get or create test accounts
    let user = info.create_and_fund_user_account(10_000_000_000).await.unwrap();

    // Test: Can we deserialize a G1 point?
    // Test: Can we add two G1 points?
    // Test: Can we multiply G1 by scalar?
}
```

### Step 3: Test DK Reconstruction

```rust
// Test the ibe::reconstruct_ibe_dk_internal native function
#[tokio::test]
async fn test_ibe_reconstruct_dk() {
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let mut info = swarm.aptos_public_info();

    // Create test DK shares (these would come from DKG in real scenario)
    let validator_indices = vector[1, 2, 3];
    let weights = vector[1, 1, 1];
    let threshold = 2;
    let total_weight = 3;

    // Call the native function
    // let dk = ibe::reconstruct_ibe_dk_internal<G1>(
    //     validator_indices,
    //     dk_shares,
    //     weights,
    //     threshold,
    //     total_weight
    // );

    // Verify the result
}
```

## Prerequisites

1. Build `aptos` CLI with new native function:

   ```bash
   cargo build --release -p aptos
   ```

2. Publish updated framework to localnet

3. Ensure feature flag is enabled for BLS12_381 structures

## Test Structure

Add the new test module to `testsuite/smoke-test/src/ibe/mod.rs`:

```rust
pub mod on_chain;
```

And implement the tests in `testsuite/smoke-test/src/ibe/on_chain.rs`.

## Expected Behavior

When complete, the tests should:

1. ✅ Initialize a local blockchain
2. ✅ Create test accounts
3. ✅ Call `ibe::reconstruct_ibe_dk_internal` successfully
4. ✅ Verify the reconstructed DK matches expected value

## How to Run

```bash
# Run IBE tests
cargo test -p smoke-test --lib ibe

# Run specific test
cargo test -p smoke-test --lib ibe::on_chain::test_on_chain_dk_reconstruction -- --nocapture
```

## Success Criteria

1. Test compiles without errors
2. Native function call succeeds
3. DK reconstruction produces correct result
4. Test runs in < 30 seconds

## If Native Function Not Available

If the native function isn't registered yet, the test should:

1. Verify the module can be loaded
2. Verify basic crypto_algebra operations work
3. Print a message indicating the native function needs to be built
4. Return success (skip) rather than fail

Example:

```rust
#[tokio::test]
async fn test_ibe_native_function_available() {
    // This test verifies the infrastructure is ready
    // It will fail if the native function isn't registered
}
```

## Related Files

- Native function: `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`
- Move wrapper: `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move`
- Smoke test lib: `testsuite/smoke-test/src/ibe/mod.rs`
- Timelock tests: `testsuite/smoke-test/src/timelock/`

## Notes

- This test requires the `aptos` CLI to be built with the new native function
- The test may need to be updated once the full DKG-MPK integration is complete
- Consider adding integration with the existing `timelock` smoke tests
