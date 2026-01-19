# Agent Prompt: Phase 3 - Timelock Registry Implementation

**Context:** Security hardening phases (2.3-2.5) are complete. The DKG now produces IBE-capable scalar shares with proper verification, serialization, and error handling.

**Current Status:**

- `Transcript::verify()` implemented with SoK + LDT verification ✅
- `get_ibe_master_public_key()` returns 96-byte G2 ✅
- `get_scalar_secret_share()` returns serialized scalars ✅
- `decrypt_own_share()` returns `Result` (no more panics) ✅
- All 47 DKG tests pass ✅

**Goal for Phase 3:** Implement on-chain timelock registry for managing timelock deadlines.

## Tasks

### Task 1: Extend `ibe_config.move` with Deadline Registration

**File:** `aptos-move/framework/aptos-framework/sources/ibe_config.move`

**Current state:** Contains `IBEConfig` resource storing MPK. Need to add:

- `TimelockDeadline` struct: `{ id: u64, deadline_us: u64, sender: address }`
- `TimelockRegistry` resource: `Table<u64, TimelockDeadline>` or `vector<TimelockDeadline>`
- Function to register a new timelock with deadline
- Function to update deadline (before reveal)
- Event emission for registration

**Design considerations:**

- Use epoch-based or timestamp-based deadlines
- Consider gas costs for table operations
- Need ability to query all pending timelocks for a user

### Task 2: Add View Functions for Querying Pending Timelocks

**File:** `aptos-move/framework/aptos-framework/sources/ibe_config.move`

Add `#[view]` functions:

- `get_timelock(deadline_id: u64): Option<TimelockDeadline>`
- `get_pending_timelocks_by_sender(sender: address): vector<TimelockDeadline>`
- `is_revealable(deadline_id: u64): bool` - Check if deadline has passed

### Task 3: Implement `register_and_query` Smoke Test

**File:** `testsuite/smoke-test/src/timelock/`

Create `testsuite/smoke-test/src/timelock/register_and_query.rs`:

- Register multiple timelocks with different deadlines
- Query pending timelocks
- Verify view functions work correctly

## Implementation Details

### Suggested Code Structure

```move
// In ibe_config.move

/// Timelock metadata stored on-chain
struct TimelockDeadline has copy, drop, store {
    id: u64,
    deadline_us: u64,
    sender: address,
}

/// Registry of all pending timelocks
struct TimelockRegistry has key {
    timelocks: Table<u64, TimelockDeadline>,
    /// Index by sender for easy querying
    by_sender: Table<address, vector<u64>>,
}

public entry fun register_timelock(
    framework: &signer,
    id: u64,
    deadline_us: u64,
) {
    // Emit event for indexing
}

#[view]
public fun get_timelock(id: u64): Option<TimelockDeadline> {
    // Return timelock if exists
}
```

## Success Criteria

- [ ] `ibe_config.move` has deadline registration functions
- [ ] View functions allow querying pending timelocks
- [ ] `register_and_query` smoke test passes
- [ ] No regressions in existing DKG tests

## Files to Create/Modify

| File                                                           | Action | Description                     |
| -------------------------------------------------------------- | ------ | ------------------------------- |
| `aptos-move/framework/aptos-framework/sources/ibe_config.move` | Modify | Add deadline registration       |
| `testsuite/smoke-test/src/timelock/mod.rs`                     | Modify | Add `register_and_query` module |
| `testsuite/smoke-test/src/timelock/register_and_query.rs`      | Create | Smoke test for registry         |

## Commands to Run After Completion

```bash
# Run smoke tests
cargo test -p smoke-test --lib timelock::register_and_query -- --nocapture

# Verify no regressions
cargo test -p smoke-test --lib randomness::e2e_correctness -- --nocapture
```

## Reference Documentation

- `atomica/docs/adr-001-dual-output-dkg.md` - Architecture overview
- `atomica/docs/implementation-plan-unified-dkg-ibe.md` - Current phase tracking
- Existing `ibe_config.move` for code patterns

## Notes

- Use `timestamp::now_microseconds()` for deadline comparisons
- Consider using `event::emit()` for indexing
- Gas optimization: may want to limit number of pending timelocks per sender
