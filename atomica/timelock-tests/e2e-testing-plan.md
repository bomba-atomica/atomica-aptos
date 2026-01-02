# Timelock E2E Testing Plan

**Purpose**: Comprehensive testing strategy for verifying the timelock feature in production-like environments.

---

## Production Emulation Strategy

Our E2E tests emulate production behavior through the `docker-test-harness`:

### Core Mechanisms

| Mechanism | How It Works | Why It Matters |
|-----------|--------------|----------------|
| **Custom Genesis Injection** | Mount a custom `head.mrb` to `/framework.mrb` in Docker containers | Test framework changes (e.g., 0.1s intervals) without governance proposals |
| **Multi-Validator Networks** | Run 2-4 validator nodes with real consensus | Test DKG coordination across distributed validators |
| **Production-Like Funding** | Accounts funded via validator transfers, not minting | Ensures test code works identically on mainnet |
| **Deterministic Rotation** | `force_rotation_for_testing()` bypasses time checks | Enables reliable testing without waiting for block timestamps |

### Framework Development Loop

```
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│  1. MODIFY       │ -> │  2. REBUILD      │ -> │  3. TEST         │
│  Move sources    │    │  build-framework │    │  bun run test:*  │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

> [!IMPORTANT]
> **Antipattern**: Do NOT use runtime scripts to change core configs. This bypasses production governance flows and creates false positives.

---

## Implementation Status

### Legend
- ✅ **Implemented** — Test exists and has working assertions
- 🔶 **Stubbed** — Test file exists but contains TODO/placeholder logic
- ❌ **Not Implemented** — No test exists

---

## Test Categories

### 1. Initialization & Genesis Tests

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `timelock_genesis_init` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | Verifies `isTimelockInitialized()` returns true |
| `config_at_genesis` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | Queries `getCurrentInterval()` after genesis |
| `custom_framework_loading` | ✅ | [framework-compilation.test.ts](file:///Users/lucas/code/rust/aptos/atomica/docker-test-harness/test/framework-compilation.test.ts) | Builds framework with noop.move, verifies on-chain |

### 2. Rotation Tests

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `manual_rotation_trigger` | ✅ | [test-manual-rotation.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/src/test-manual-rotation.ts) | Uses `triggerRotation()`, verifies interval increment |
| `forced_rotation` | ✅ | [test-manual-rotation.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/src/test-manual-rotation.ts) | Uses `forceRotationForTesting()` to bypass time check |
| `rotation_triggers_dkg` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | Waits for transcript after rotation |
| `rotation_triggers_reveal` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | Waits for secret aggregation after next rotation |

### 3. DKG Flow Tests

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `transcript_publication` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | `waitForPublicKeyPublication()` + assertions |
| `threshold_share_collection` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | `waitForSecretAggregation(interval, 3)` |
| `secret_aggregation` | ✅ | [basic-flow.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/basic-flow.test.ts) | Verifies `decryption_key` bytes returned |

### 4. IBE Cryptography Tests

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `ibe_encrypt_decrypt_e2e` | 🔶 | [ibe-e2e.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/ibe-e2e.test.ts) | Test structure in place, crypto uses **placeholders** (TODO comments) |
| `identity_derivation` | ❌ | — | Not implemented |
| `transcript_deserialization` | 🔶 | [ibe-e2e.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/ibe-e2e.test.ts) | Placeholder `new Uint8Array(96)` instead of real parsing |
| `decryption_key_deserialization` | 🔶 | [ibe-e2e.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/ibe-e2e.test.ts) | Placeholder `new Uint8Array(48)` instead of real parsing |

### 5. Security & Edge Cases

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `invalid_share_rejection` | ❌ | — | Not implemented |
| `future_interval_reveal_blocked` | ❌ | — | Not implemented |
| `duplicate_share_ignored` | ❌ | — | Not implemented |
| `threshold_not_met` | ❌ | — | Not implemented |

### 6. Failure Recovery Tests

| Test Case | Status | File | Notes |
|-----------|--------|------|-------|
| `dkg_failure_retry` | 🔶 | [dkg-failure-recovery.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/dkg-failure-recovery.test.ts) | **Stub only** — `expect(true).toBe(true)`, TODO comments for validator kill logic |
| `validator_dropout_recovery` | 🔶 | [validator-changes.test.ts](file:///Users/lucas/code/rust/aptos/atomica/timelock-tests/test/validator-changes.test.ts) | **Stub only** — `expect(true).toBe(true)`, TODO comments for dynamic validator changes |

---

## Infrastructure Tests (docker-test-harness)

These tests verify the harness itself, not timelock logic:

| Test | Status | File | Notes |
|------|--------|------|-------|
| Faucet mechanism | ✅ | [faucet.test.ts](file:///Users/lucas/code/rust/aptos/atomica/docker-test-harness/test/faucet.test.ts) | Verifies production-like funding via validator transfers |
| Validator connectivity | ✅ | [validator-connectivity.test.ts](file:///Users/lucas/code/rust/aptos/atomica/docker-test-harness/test/validator-connectivity.test.ts) | Checks all validators respond with correct chain ID |
| Block production | ✅ | [block-production.test.ts](file:///Users/lucas/code/rust/aptos/atomica/docker-test-harness/test/block-production.test.ts) | Waits for 5 blocks, verifies height increment |
| Framework compilation | ✅ | [framework-compilation.test.ts](file:///Users/lucas/code/rust/aptos/atomica/docker-test-harness/test/framework-compilation.test.ts) | Meta-test for custom genesis injection |

---

## Summary

| Category | Implemented | Stubbed | Missing |
|----------|-------------|---------|---------|
| Initialization & Genesis | 3 | 0 | 0 |
| Rotation | 4 | 0 | 0 |
| DKG Flow | 3 | 0 | 0 |
| IBE Cryptography | 0 | 3 | 1 |
| Security & Edge Cases | 0 | 0 | 4 |
| Failure Recovery | 0 | 2 | 0 |
| **Total** | **10** | **5** | **5** |

### Priority Next Steps

1. **Complete IBE E2E test** — Implement actual BLS12-381 crypto (G1/G2 deserialization, IBE encrypt/decrypt)
2. **Implement security tests** — Critical for production: invalid share rejection, future interval protection
3. **Flesh out failure recovery stubs** — Add validator kill/restart logic to existing test files

---

## Test Infrastructure

### Helper Classes

```typescript
// Transactions - submit on-chain actions
TimelockTransactions
  ├── setIntervalForTesting(intervalUs)      // Configure test interval
  ├── forceRotationForTesting()              // Bypass time check
  └── triggerRotation()                      // Production-like rotation

// Queries - read blockchain state
TimelockQueries
  ├── getCurrentInterval()                   // Current timelock interval
  ├── getTimelockState()                     // Full state for debugging
  ├── verifyPublicKeyPublished(interval)     // Check transcript exists
  └── verifySecretAggregated(interval, threshold)  // Check DK exists

// Waiters - poll until condition or timeout
TimelockWaiters
  ├── waitForIntervalRotation(target, timeout)
  ├── waitForPublicKeyPublication(interval, timeout)
  └── waitForSecretAggregation(interval, threshold, timeout)
```

### Test Commands

```bash
bun run test:basic      # Basic DKG flow
bun run test:ibe        # IBE encryption/decryption
bun run test:rotation   # Manual rotation trigger
bun run test:framework-loading  # Custom genesis verification
bun test                # All tests
```

---

## Execution Guidelines

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `ATOMICA_DEBUG_TESTNET` | Enable verbose logging | `false` |
| `IMAGE_TAG` | Docker image version | `latest` |
| `VALIDATOR_IMAGE_REPO` | Docker registry | `ghcr.io/bomba-atomica/atomica-aptos/validator` |

### Timeouts

| Operation | Recommended Timeout |
|-----------|---------------------|
| Testnet initialization | 5 minutes |
| Interval rotation | 2 minutes |
| Public key publication | 1 minute |
| Secret aggregation | 1 minute |

### Debugging Failures

1. **Check Docker logs**: `docker compose logs validator-0`
2. **Enable debug mode**: `ATOMICA_DEBUG_TESTNET=1 bun run test:*`
3. **Verify framework**: Ensure `head.mrb` is fresh after Move changes
4. **Manual cleanup**: `docker compose down -v --remove-orphans`
