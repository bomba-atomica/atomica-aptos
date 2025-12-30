# Timelock Tests Specification

## Overview

The timelock-tests is a TypeScript-based test suite designed to replace the existing smoketests for Distributed Key Generation (DKG) and timelock functionality in the Atomica project. This suite leverages the docker-test-harness SDK to manage test environments, focusing on transaction-driven validation of validator operations.

## Goals

- **Transaction-Centric Testing**: ✅ COMPLETED - All operations use Aptos transactions via TypeScript
- **Comprehensive Coverage**: 🟡 IN PROGRESS - Basic flows covered, edge cases and error handling pending
- **Automated Setup/Teardown**: ✅ COMPLETED - Docker testnet lifecycle fully automated
- **Framework Compilation Verification**: ✅ COMPLETED - Fresh .mrb loading verified with Noop contract test
- **Manual Rotation Trigger**: ✅ COMPLETED - Public function for testing rotation logic
- **IBE Crypto Implementation**: 🔄 NEXT - Real cryptographic operations vs current placeholders
- **CI Integration**: ✅ COMPLETED - GitHub Actions workflow with Docker setup

## Current Status

### ✅ **Completed Infrastructure**

- Docker testnet startup and management (2-4 validators)
- Transaction submission and blockchain queries
- Test execution patterns (round-robin, breadth-first)
- Framework compilation and verification system
- Manual timelock rotation trigger
- Comprehensive error handling and logging

### 🔧 **Active Development**

- **Phase 1 Critical Fixes**: Invalid share counting, historical threshold storage, DKG topic fixes
- **IBE Cryptographic Operations**: TypeScript implementation of BLS12-381 IBE
- **Enhanced Test Coverage**: Edge cases, failure scenarios, recovery mechanisms

### 🎯 **Next Priorities**

1. **Fix Move Code Bugs**: Invalid share counting, threshold storage (Phase 1)
2. **Implement Real IBE Crypto**: Replace stubs with actual BLS12-381 operations
3. **Complete End-to-End Flow**: DKG → Key Publication → Reveal → IBE Encryption/Decryption
4. **Add Comprehensive Tests**: Error cases, validator changes, failure recovery

## Rationale

Current smoketests for DKG and timelock rely on manual or scripted processes that may not fully simulate real-world validator interactions. By shifting to transaction-based testing, we ensure that all operations are validated through the blockchain's transaction layer, mirroring production behavior. This approach eliminates dependencies on external scripts and improves test maintainability.

**Key Innovation**: Framework compilation verification ensures we're testing against freshly built Move code, not cached artifacts, providing confidence that changes are properly deployed.

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
