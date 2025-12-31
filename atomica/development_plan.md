# Atomica Timelock Development Plan

**Last Updated:** December 31, 2024
**Target:** Production-ready MVP

## Status Overview

The timelock implementation is effectively **feature-complete** regarding the "Happy Path". The core infrastructure for DKG, interval rotation, and IBE encryption/decryption is working. However, critical bugs relating to security and correctness need to be addressed before production deployment.

### Current Achievement Level
*   **95% Complete**: Core timelock + IBE system implemented.
*   **Blockers**: Framework loading verification in Docker, specific security bugs.

---

## Roadmap

### Phase 1: Critical Bug Fixes (Immediate Priority)

*   **Task 1.1: Fix Invalid Share Counting**
    *   *Issue*: Invalid shares are currently skipped but still counted towards the threshold.
    *   *Fix*: Refactor `publish_secret_share` to validate shares before counting them.
*   **Task 1.2: Fix Historical Threshold Storage**
    *   *Issue*: Threshold is calculated using the *current* validator set at reveal time, not the set from the DKG time.
    *   *Fix*: Store `IntervalConfig` (threshold, validator count) when `StartKeyGenEvent` is emitted.
*   **Task 1.3: Fix Topic Mismatch**
    *   *Fix*: Ensure Timelock DKG results use `Topic::TIMELOCK` instead of `Topic::DKG`.

### Phase 2: Security Hardening

*   **Task 2.1: Share Pre-validation**: Add unit tests and Move logic to reject invalid G1 points immediately.
*   **Task 2.2: Interval Validation**: Ensure shares can only be revealed for past intervals.
*   **Task 2.3: Session Cleanup**: Implement cleanup for stale Timelock DKG sessions in `epoch_manager.rs`.

### Phase 3: Code Quality & Testing

*   **Task 3.1**: Remove unused variables in Move contracts.
*   **Task 3.2**: Improve threshold fallback logic (remove hardcoded `1` for production).
*   **Task 4.1**: Add invalid share unit tests.
*   **Task 4.2**: Add threshold edge case tests (exact threshold, threshold - 1).

### Phase 4: TypeScript SDK & IBE

*   **Task 5.1**: Implement fully functional TypeScript IBE module (replace placeholders).
*   **Task 5.2**: Add DKG transcript specific parsing.
*   **Task 5.3**: Update `ibe-e2e.test.ts` to use real encryption/decryption.

### Phase 5: Future Enhancements

*   **DKG Failure Recovery**: Retry mechanism if DKG fails for an interval.
*   **Validator Set Changes**: Robust handling of validator set changes during a DKG session.

---

## Action Items

1.  **Build Custom Docker Image**: Modify `Dockerfile` to remove the built-in `head.mrb` and strictly use the mounted one for accurate framework verification.
2.  **Run Framework Loading Tests**: `bun run test:framework-loading` should pass.
3.  **Fix Critical Bugs**: Start with Task 1.1 (Invalid Share Counting).
4.  **Verify Complete Flow**: Run `bun run test:ibe` and `bun run test:rotation` after fixes.

---

## Known Issues (Code Review Findings)

*   **BUG-001 [Critical]**: Invalid Share Counting Vulnerability.
*   **BUG-002 [High]**: Threshold functionality uses current validator set.
*   **BUG-003 [Medium]**: DKG Topic Mismatch.
*   **BUG-004 [Medium]**: First Share Selection Assumption.
*   **BUG-005 [Low]**: Missing Timelock Session Cleanup.
