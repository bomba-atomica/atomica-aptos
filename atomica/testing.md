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
2.  **Framework Verification**: Ensures that the compiled Move framework (`head.mrb`) is correctly loaded by the validators, rather than using the default Docker image framework.
3.  **Manual Rotation**: Because automatic block production in Docker can be erratic for time-based triggers, a manual rotation trigger is used for deterministic testing.

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
