# Atomica Timelock Development Plan

**Last Updated:** January 1, 2026
**Target:** Production-ready MVP

## Status Overview

We have successfully established a reliable "custom genesis" testing workflow (Modify -> Rebuild -> Test), enabling us to test core framework changes (like 0.1s timelock intervals) without unreliable governance workarounds.

### Key Achievements (Completed)
*   **Infrastructure**: `test:rotation` and `test:framework-loading` verify our ability to modify and inject custom Move frameworks into ephemeral Docker testnets.
*   **Core Bug Fixes**:
    *   Optimized Invalid Share Counting (Removed O(N) redundancy).
    *   Fixed Historical Threshold Storage (Snapshotting at DKG start).
*   **Workflow**: established the *Modify -> Rebuild -> Test* loop as the standard for framework development.
*   **Testing Status**:
    *   ✅ Basic DKG flow & Rotation tests implemented.
    *   🔶 IBE E2E tests stubbed (crypto placeholders in place).
    *   ❌ Security tests pending.

### Current Focus
*   **Immediate Priority**: Implement BLS12-381 crypto helpers in `timelock-tests` to enable real IBE encryption/decryption verifying the `test:ibe` flow.

---

## Roadmap (Revised Critical Path)

### Phase 1: End-to-End Verification (Immediate Priority)

*   **Task 1.1: Verify IBE Roundtrip (`test:ibe`)**
    *   *Goal*: Ensure the TypeScript SDK can successfully encrypt a message and decrypt it using shares aggregated from the validators.
    *   *Dependencies*: `test:rotation` (Verified), `timelock.move` (Verified).
    *   *Status*: **IN PROGRESS** (Implementing `ibe-crypto.ts`).

### Phase 2: Security Hardening (Pre-Production)

*   **Task 2.1: Invalid Share Rejection**
    *   *Issue*: Malformed G1 points must be rejected at the contract level to preventing DOS or calculation errors.
    *   *Action*: Add unit tests + Move logic for point validation.
*   **Task 2.2: Topic Mismatch**
    *   *Fix*: Ensure Timelock DKG results use `Topic::TIMELOCK` specifically.
*   **Task 2.3: Interval Validation**
    *   *Action*: Ensure shares can only be revealed for *past* intervals, preventing attacks on future keys.

### Phase 3: Robustness & Cleanup

*   **Task 3.1: DKG Failure Recovery**
    *   *Feature*: Retry mechanism if DKG fails for a specific interval.
*   **Task 3.2: Session Cleanup**
    *   *Optimization*: Implement cleanup for stale Timelock DKG sessions in `epoch_manager.rs`.
*   **Task 3.3: Threshold Fallback**
    *   *Refactor*: Remove hardcoded `1` threshold fallback used for testing; enforce strict thresholds in production.

---

## Development Workflow

We strictly adhere to the **Modify -> Rebuild -> Test** loop for framework changes:

1.  **Modify** `aptos-framework` sources.
2.  **Rebuild** using `./atomica/move-framework-fixtures/build-framework.sh`.
3.  **Test** using `bun run test:rotation` (or similar), which injects the custom `head.mrb`.

**⛔️ ANTIPATTERN**: Do not create runtime scripts (e.g., `set_interval.move`) to change core configs. This leads to signer permission errors and does not match production governance flows.

---

## Action Items

1.  **Implement `ibe-crypto.ts`**: Add BLS12-381 G1/G2 deserialization and IBE encrypt/decrypt logic.
2.  **Update `test:ibe`**: Connect test to real crypto implementation.
3.  **Implement Invalid Share Tests**: Create specific test cases for malformed inputs.
