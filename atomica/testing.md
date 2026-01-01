# Atomica Timelock Testing

## Overview

The `timelock-tests` suite (located in `atomica/timelock-tests`) is a TypeScript-based test suite designed to replace legacy smoketests. It utilizes the `docker-test-harness` to manage ephemeral Move-enabled testnets, allowing for transaction-centric validation of validator operations, DKG cycles, and IBE cryptography.

## How to Run

### Prerequisites
*   Bun runtime
*   Docker

### Commands

Run basic flow tests:
```bash
bun run test:basic
```

Run IBE end-to-end tests:
```bash
bun run test:ibe
```

Run manual rotation tests:
```bash
bun run test:rotation
```

Run framework loading verification:
```bash
bun run test:framework-loading
```

To run all tests:
```bash
bun test
```

---

## Architecture

*   **Language**: TypeScript (running on Bun).
*   **Core SDK**: `docker-test-harness` for managing validator containers.
*   **Transaction**: Aptos SDK for crafting and submitting transactions.
*   **Structure**:
    *   `src/`: Core test logic and helpers.
    *   `test/`: Individual test scenarios.
    *   `helpers/`: Utilities for testnet setup and assertions.

### Key Components

1.  **Testnet Lifecycle**: Automatically spins up 2-4 validator networks with custom genesis.
2.  **Framework Verification & Custom Genesis**:
    *   **Goal**: Test modified Move framework code (e.g., shorter timelock intervals) without waiting for mainnet-like delays.
    *   **Mechanism**: The test harness looks for a compiled framework artifact at `atomica/move-framework-fixtures/head.mrb`.
    *   **Injection**: If found, this artifact is mounted into the Docker container and used during `aptos genesis generate-genesis`. This effectively replaces the default framework baked into the Docker image.
    *   **Build Command**: Use `aptos-framework custom ... --output atomica/move-framework-fixtures/head.mrb` to generate this file.
3.  **Manual Rotation**: Because automatic block production in Docker can be erratic for time-based triggers, a manual rotation trigger is used for deterministic testing.

---

## Framework Development Workflow

### The Correct Loop (Modify -> Rebuild -> Test)

When you need to test changes to the core Move framework (e.g., `timelock_config`, consensus logic), follow this loop. This ensures that your changes are baked into the genesis state, replicating how network upgrades actually work (or how a new chain starts).

1.  **Modify Source**: Edit the Move files in `aptos-framework` (e.g., `aptos-move/framework/aptos-framework/sources/configs/timelock_config.move`).
2.  **Rebuild Artifact**: Run the build script to update the testnet fixture.
    ```bash
    ./atomica/move-framework-fixtures/build-framework.sh
    ```
3.  **Run Test**: Execute your test command. The test runner (`genesis.ts`) automatically detects the updated `head.mrb` and injects it into the new testnet.
    ```bash
    bun run test:rotation
    ```

### ⛔️ Antipattern: Runtime Script Injection

**Do NOT try to change core framework configurations using runtime scripts or transaction payloads (e.g., `set_interval.move`).**

*   **Why?**: Core configurations often require special privileges (like `@aptos_framework` signer) that are difficult or impossible to obtain via standard transaction flows in a production-like environment.
*   **Risk**: Tests might pass by hacking permissions (e.g., `0x1` signer) but fail in reality where those paths don't exist.
*   **Correct Approach**: Use "Custom Genesis" as described above. If parameters need to be different for testing, use `#[test_only]` helpers within the framework itself, but apply them via the genesis configuration or specific governance proposals if testing upgrades.

---

## Troubleshooting & Known Obstacles

### Common Issues

*   **Test Timeout**: Initializing the Docker testnet can take time. If tests fail with timeout, increase the timeout configuration in Bun or the test runner.
*   **Framework Verification Failure**: If the noop contract is not found, the Docker container might be prioritizing its built-in framework over the mounted one. Ensure the `docker-compose.yaml` volume mounts are correct and the `head.mrb` is fresh.
*   **Docker Instability**: Intermittent daemon failures can cause testnet initialization to hang. Restarting Docker usually resolves this.

### Workarounds

*   **Manual Rotation**: Automatic `on_new_block` rotation is unreliable in testnets due to block timing. Use the `trigger_rotation` public function for testing DKG flows.
*   **Pre-compiled Attributes**: Compiling the framework from scratch for every test run is slow. The tests utilize a pre-compiled `head.mrb` (located in `atomica/move-framework-fixtures/`) to speed up execution.

---

## Test Coverage Checklist

*   [x] **Basic DKG Flow**: Key generation, transcript submission, publication.
*   [x] **Share Reveal**: Validators revealing shares for past intervals.
*   [x] **Aggregation**: On-chain aggregation of secret shares.
*   [ ] **Invalid Share Rejection**: Verifying that malformed shares are rejected (TODO).
*   [ ] **Full IBE Roundtrip**: End-to-end encryption and decryption with real keys (TODO).
*   [ ] **Failure Recovery**: Simulating DKG failure and retry (TODO).
