# Aptos Docker Testnet - TypeScript SDK

Production-like Aptos testnet for local development and testing. Runs 1-7 validators with full consensus, block production, and **production-like account funding**.

## Quick Start

```typescript
import { DockerTestnet } from "@atomica/docker-testnet";
import { AptosAccount } from "aptos";

// 1. Start testnet with 4 validators
const testnet = await DockerTestnet.new(4);

// 2. Bootstrap validators with unlocked funds (ONE TIME)
// This simulates validators earning staking rewards
await testnet.bootstrapValidators();

// 3. Use production-like faucet to fund new accounts
const newAccount = new AptosAccount();
await testnet.faucet(newAccount.address(), 100_000_000n); // 1 APT

// 4. Test your code...

// 5. Clean up
await testnet.teardown();
```

## Key Features

### ✅ Production-Like Account Funding

**No magic accounts, no minting privileges.** This testnet emulates production mainnet:

- **Validators have unlocked funds** (simulating staking rewards).
- New accounts are funded via **validator transfers** (not minting).
- The **Root Account** (0xA550C18) is used **only** for initial bootstrap.
- All runtime operations use standard account-to-account transfers.

**Why This Matters:** Your test code behaves identically on mainnet!

### ✅ Robust Lifecycle Management

- **Automatic Cleanup**: Signal handlers intercept `SIGINT` (Ctrl+C), `SIGTERM`, and uncaught exceptions to ensure Docker containers are cleaned up.
- **Multi-Validator Consensus**: Runs real consensus with 1-7 validators, network connectivity, and peer discovery.
- **Block Production**: Correctly tracks block height (not just ledger versions) for reliable `waitForBlocks` synchronization.

## Installation

```bash
cd source/docker-testnet/typescript-sdk
npm install
npm run build
```

## API Reference

### `DockerTestnet.new(numValidators, customFrameworkPath?)`

Create and start a fresh testnet.

- **numValidators**: Number of validators (1-7). Default/Recommended is 4.
- **customFrameworkPath**: Optional path to custom `head.mrb` framework file for genesis generation.
- **Time**: ~30s to start (longer on first run for image pulls).

**Framework Notes**: If not provided, uses the framework built into the Docker image. Only affects genesis generation - running validators use the framework embedded in the genesis blob.

### `bootstrapValidators(amountPerValidator?)`

**One-time setup** to give validators unlocked funds for faucet operations.

- **amountPerValidator**: Amount in octas (default: 100,000 APT).
- **Note**: Must be called once after starting the testnet.

### `faucet(address, amount?)`

Fund a new account using production-like validator transfers.

- **address**: Recipient address (Hex string or AccountAddress).
- **amount**: Amount in octas (default: 0.1 APT).
- **Mechanism**: Randomly selects a validator to transfer funds. Auto-creates the account if it doesn't exist.

### `waitForBlocks(numBlocks, timeoutSecs?)`

Wait for a specific number of blocks to be produced.

- **numBlocks**: Number of blocks to wait for.
- **timeoutSecs**: Timeout in seconds (default: 30).
- **Note**: This tracks actual **block height**, not ledger versions, ensuring reliable synchronization.

### `getValidatorAccount(index)`

Get a specific validator account with private key access. Useful for direct validator operations.

### `teardown()`

Stop the testnet and clean up all resources.

- **Note**: Always call this in your `afterAll` hook to prevent orphaned containers.

## Running Tests

The SDK includes a comprehensive test suite verifying the faucet, network connectivity, and consensus.

```bash
# Run all tests
npm test

# Run with debug logging (highly recommended for troubleshooting)
ATOMICA_DEBUG_TESTNET=1 npm test

# Run a specific test file
npx bun test test/faucet.test.ts
```

### Signal Handling & Cleanup

The test suite includes signal handlers that guarantee cleanup in almost all scenarios:

- **Success/Failure**: `afterAll` hook runs `teardown()`.
- **Ctrl+C / SIGTERM**: Signal handlers intercept and run `docker compose down`.
- **Uncaught Exceptions**: Handlers catch crashes and cleanup.

**Note:** `kill -9` (SIGKILL) cannot be caught. If used, you must manually clean up.

## Troubleshooting

### Manual Cleanup

If tests hang, crash hard, or `kill -9` was used, you may have orphaned containers.

```bash
cd ../config
docker compose down -v --remove-orphans
```

_Tip: If containers are stubborn, use `docker rm -f $(docker ps -aq --filter name=atomica)`._

### Port Conflicts

Ensure ports `8080-8086` (REST), `6180-6186` (P2P), and `9101-9107` (Metrics) are free.

```bash
lsof -i :8080-8086
```

### Validators Stuck at Genesis

If validators report `Epoch: 0` and `Block Height: 0` for >30s:

1. Check logs: `docker compose logs validator-0`
2. Ensure `ATOMICA_DEBUG_TESTNET=1` shows successful peer connections.
3. Verify your machine has enough resources (Docker CPU/RAM).

### Framework-Related Issues

#### "Could not find framework.mrb" Error

The genesis generation script cannot locate a framework file. Solutions:

1. **Use Built-in Framework**: Ensure your Docker image includes a framework at `/aptos-framework/move/head.mrb`
2. **Provide Custom Framework**: Pass a custom framework path to `DockerTestnet.new()`
3. **Check Framework Binary**: If using `compileAndPlaceFramework()`, ensure `aptos-framework` binary exists at `~/.cargo/bin/aptos-framework`

#### Framework Version Mismatch

If genesis generation succeeds but validators fail to start:

1. Ensure the framework version matches the aptos CLI version in the Docker image
2. Check that custom framework files are compatible with the validator image
3. Verify framework compilation completed successfully (no errors in build logs)

## Genesis and Framework Management

The testnet uses a multi-step process to generate genesis artifacts and manage the Move framework.

### Genesis Generation Process

1. **Framework Compilation**: The Move framework is compiled into a `head.mrb` binary file
2. **Genesis Script Execution**: A shell script (`generate-genesis.sh`) runs inside a Docker container
3. **Validator Configuration**: Each validator gets identity keys, network configuration, and genesis artifacts
4. **Genesis Blob Creation**: The aptos CLI generates `genesis.blob` and `waypoint.txt` files

### Framework File (head.mrb) Handling

The framework file (`head.mrb`) contains the compiled Move bytecode for the Aptos framework. This file is critical for genesis generation.

#### Default Framework Location

- **Built-in**: The Docker image includes a pre-compiled framework at `/aptos-framework/move/head.mrb`
- **Override**: Developers can provide a custom framework file for testing modifications

#### Providing Custom Framework Files

For convenience, a default framework may be baked into the Docker image. However, when developing or testing framework modifications:

1. **Compile Custom Framework**:

    ```bash
    # Build the aptos-framework binary
    cd /path/to/aptos-core
    cargo build -p aptos-framework --release

    # The binary will be at ~/.cargo/bin/aptos-framework
    # Generate head.mrb using the binary
    ~/.cargo/bin/aptos-framework release --target head --output ./head.mrb
    ```

2. **Use Custom Framework**: Pass the custom framework path when creating the testnet:

    ```typescript
    const testnet = await DockerTestnet.new(4, "/path/to/custom/head.mrb");
    ```

3. **Framework in Genesis**: Only the framework file used during genesis generation matters. Running validators do not need the framework file - they use the compiled bytecode embedded in the genesis blob.

#### Framework File Locations Checked (in priority order):

1. `/framework.mrb` (Docker volume mount from host)
2. `/aptos-framework/move/head.mrb` (built into Docker image)
3. `/opt/aptos/framework/head.mrb` (alternative image location)
4. `/usr/local/share/aptos/framework/head.mrb` (system location)
5. Repository search for any `.mrb` files

#### Framework Loading Verification

The docker-test-harness includes a "meta test" that verifies custom framework loading functionality:

- **Location**: `atomica/timelock-tests/src/test-framework-compilation.ts`
- **Purpose**: Tests the harness's ability to insert modified `.mrb` files into testnets
- **Method**: Creates a custom framework with a test "noop" contract, loads it via `customFrameworkPath`, and verifies the contract is available on-chain
- **Run**: `cd ../timelock-tests && bun run test:framework`

This test ensures that framework modifications can be tested without rebuilding Docker images.

### Genesis Script (`generate-genesis.sh`)

The genesis generation runs in a Docker container to ensure:

- Consistent aptos CLI version matching the framework
- Isolated environment for key generation
- Proper file permissions and ownership

Key steps in the script:

1. Generate root account keys (for test-only faucet)
2. Generate validator identity keys and configurations
3. Copy framework.mrb to genesis repository
4. Create layout.yaml with network parameters
5. Generate genesis.blob and waypoint.txt
6. Create validator node configurations

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  Docker Network: 172.19.0.0/16                  │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│  validator-0  │  validator-1  │  validator-2  │  validator-3    │
│  172.19.0.10  │  172.19.0.11  │  172.19.0.12  │  172.19.0.13    │
│  Ports: 8080  │  Ports: 8081  │  Ports: 8082  │  Ports: 8083    │
└───────────────┴───────────────┴───────────────┴─────────────────┘
```

### Genesis Data Flow

```
Host Framework File ──(mounted)──> Docker Container
                                       │
                                       ▼
Genesis Script ──(aptos CLI)──> Genesis Repository
                                       │
                                       ▼
Genesis Artifacts ──(volumes)──> Validator Containers
```
