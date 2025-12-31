# Timelock Tests Specification

## Overview

The timelock-tests is a TypeScript-based test suite designed to replace the existing smoketests for Distributed Key Generation (DKG) and timelock functionality in the Atomica project. This suite leverages the docker-test-harness SDK to manage test environments, focusing on transaction-driven validation of validator operations.

## Goals

- **Transaction-Centric Testing**: Ensure all DKG and timelock operations performed by validators are conducted through Aptos transactions, crafted and executed in TypeScript.
- **Comprehensive Coverage**: Cover key scenarios including DKG initialization, key sharing, timelock creation, unlocking, and error handling.
- **Automated Setup/Teardown**: Utilize docker-test-harness for spinning up and tearing down multi-validator testnets, ensuring isolated and repeatable test runs.
- **Integration with Atomica-Web**: Mirror the testing patterns used in the atomica-web project, promoting consistency across the codebase.
- **Reliability and Speed**: Provide faster, more reliable tests compared to smoketests, with better debugging and CI integration.

## Rationale

Current smoketests for DKG and timelock rely on manual or scripted processes that may not fully simulate real-world validator interactions. By shifting to transaction-based testing, we ensure that all operations are validated through the blockchain's transaction layer, mirroring production behavior. This approach eliminates dependencies on external scripts and improves test maintainability.

## Implementation

### Architecture

- **Language**: TypeScript with Bun as the runtime.
- **Dependencies**:
  - docker-test-harness SDK for testnet management.
  - Aptos SDK for transaction crafting and execution.
  - Test framework: Bun's built-in test runner.
- **Structure**:
  - `src/`: Core test logic, transaction helpers, and scenario definitions.
  - `test/`: Individual test files for DKG phases, timelock operations, and integration tests.
  - `helpers/`: Utilities for testnet setup, account management, and assertions.

### Key Components

1. **Testnet Lifecycle**: Use docker-test-harness to initialize multi-validator networks, generate genesis, and handle cleanup.
2. **Transaction Crafting**: Implement functions to create and submit DKG-related transactions (e.g., key submission, timelock proposals).
3. **Scenario Testing**: Define test cases for happy paths, edge cases (e.g., invalid keys, timeouts), and failure recovery.
4. **Assertions**: Validate blockchain state changes, transaction success, and timelock conditions post-execution.

### Comparison to Atomica-Web

Similar to atomica-web, this suite will:

- Use the docker-test-harness for environment management.
- Craft transactions programmatically to interact with smart contracts.
- Focus on end-to-end validation of protocol features.

However, timelock-tests will be more focused on consensus and cryptographic operations, while atomica-web may cover user-facing features.

### Development Plan

1. Setup project structure with package.json, tsconfig, and initial dependencies.
2. Implement basic testnet setup and teardown using docker-test-harness.
3. Develop transaction helpers for DKG and timelock modules.
4. Write initial test cases for DKG initialization and timelock creation.
5. Integrate with CI for automated runs, ensuring tests pass with the atomica-aptos validator image.

This specification outlines the foundation for a robust, transaction-driven test suite that enhances the reliability of DKG and timelock features in Atomica.</content>
<parameter name="filePath">atomica/timelock-tests/spec.md
