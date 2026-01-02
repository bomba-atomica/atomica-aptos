# Atomica Timelock Verification Strategy

This document outlines the **component-first** strategy for verifying the Atomica Timelock DKG integration. We prioritize isolating and verifying individual components (Move logic, Rust DKG machinery, Crypto) before attempting full end-to-end flows.

**Core Principle:** Do not debug complex E2E failures until individual components are proven distinctively correct.

---

## Phase 1: Unit & Logic Verification (No Docker)
*Goal: Verify logic and state transitions in isolation. Fast feedback loop.*

### 1.1 Move Framework Tests
**Location:** `aptos-move/framework/aptos-framework/sources/timelock.move`
- [ ] **Rotation Logic**: Verify `timelock::on_new_block` triggers rotation after interval passes. 
- [ ] **State Updates**: Verify `current_interval` increments and `StartKeyGenEvent` is emitted.
- [ ] **Access Control**: Verify only validators can publish keys/shares.
- [ ] **Share Aggregation**: Verify `publish_secret_share` correctly aggregates G1 points (mocked crypto) and emits `SecretRevealedEvent`.

### 1.2 Rust Component Tests
**Location:** `aptos-node`, `dkg`, `consensus`
- [ ] **Event Subscription**: Unit test `create_event_subscription_service` (state_sync) to ensure Timelock events are filtered.
- [ ] **DKG Trigger**: Unit test `EpochManager::on_dkg_start_notification` to verify it parses `StartKeyGenEvent` correctly.
- [ ] **Metadata**: Verify `build_timelock_session_metadata` produces correct session IDs.

### 1.3 Crypto/IBE Tests
**Location:** `timelock-tests/src/test-crypto-only.ts` / Rust Unit Tests
- [ ] **BLS12-381 Math**: Verify off-chain valid share aggregation matches on-chain logic (using `test-crypto-only.ts`).
- [ ] **IBE Primitives**: Verify `encrypt` and `decrypt` functions work with a static set of keys, completely offline.

---

## Phase 2: Isolated System Tests (Docker)
*Goal: Verify specific subsystems in a running environment without full dependency chains.*

### 2.1 Sanity & Configuration
**Tests:** `src/test-sanity.ts`
- [ ] **Flags**: Confirm `validator_txn_enabled` and `randomness_enabled` are active.
- [ ] **Resource Existence**: Confirm `TimelockState` and `ConsensusConfig` exist on-chain.
- [ ] **Validator Set**: Confirm 2+ validators are active.

### 2.2 Passive Timelock Rotation (The "Heartbeat")
**Tests:** `src/test-transitions.ts` (New/Planned)
- [ ] **Goal**: Verify the "clock" ticks without user intervention.
- [ ] **Check**: Poll `current_interval` for 3-4 intervals.
- [ ] **Success**: Intervals increase roughly according to `timelock_config`. Events are emitted.
- [ ] **Note**: Does NOT require DKG to succeed, only the Move scheduling logic.

### 2.3 Manual DKG Triggering
**Tests:** `src/test-manual-rotation.ts`
- [ ] **Goal**: Force specific state transitions to test the DKG machinery on demand.
- [ ] **Action**: Use `force_rotation_for_testing` or `trigger_rotation`.
- [ ] **Verify**: Logs show validators received the event and started a DKG session.

### 2.4 DKG-Only Loop
**Tests:** `src/test-dkg-loop.ts` (New/Planned)
- [ ] **Goal**: Verify ONLY that keys are generated and published.
- [ ] **Check**: Monitor `KeyPublishedEvent`.
- [ ] **Success**: Public keys appear in `TimelockState::public_keys` table.

---

## Phase 3: Integration Tests
*Goal: Connect 2+ components.*

- [ ] **Rotation + DKG**: Verify that automatic rotation actually triggers DKG and results in a published key (combines 2.2 and 2.4).
- [ ] **Share Submission**: Manually inject valid shares (as a validator) to verify on-chain aggregation logic in a real testnet.

---

## Phase 4: End-to-End IBE Flow
*Goal: The user story. Only run this after Phases 1-3 pass.*

**Tests:** `src/test-ibe.ts`
1.  **Wait** for interval `N` public key.
2.  **Encrypt** message for interval `N` (Client Side).
3.  **Wait** for interval `N` secret key (requires Validator DKG + Reveal).
4.  **Decrypt** message (Client Side).
