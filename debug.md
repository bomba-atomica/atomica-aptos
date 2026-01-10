# MPK Publication Test Failure - Debug Guide

## Issue
`cargo t -p smoke-test test_mpk_published -- --nocapture --test-threads=1` fails after 60s with "Master Public Key not found for interval 1"

## Root Cause (Most Likely)
The timelock DKG **waits for randomness DKG to complete first** (added in commit 59f84b7f41). If randomness DKG never finishes, timelock DKG never starts, and no MPK is published.

## Critical Gate
```
timelock.move:217-240 - on_new_block()
  └── Checks dkg::has_completed()
      └── If false: timelock DKG never starts
      └── If true: emits StartKeyGenEvent, DKG proceeds
```

## Pipeline Summary
```
Randomness DKG completes
    ↓
dkg::has_completed() → true
    ↓
timelock.move emits StartKeyGenEvent
    ↓
DKG manager produces transcript (dkg_manager/mod.rs:419-445)
    ↓
MPK extracted, serialized to 96 bytes
    ↓
ValidatorTransaction::TimelockDKGResult submitted
    ↓
threshold_dsa.move:68-93 stores MPK
    ↓
Test query succeeds
```

## Key Files to Investigate

| File | Lines | Purpose |
|------|-------|---------|
| `dkg/src/dkg_manager/mod.rs` | 419-445 | MPK extraction from transcript |
| `aptos-move/framework/aptos-framework/sources/timelock.move` | 217-240 | Sequencing gate (waits for randomness DKG) |
| `aptos-move/framework/aptos-framework/sources/dkg.move` | 133-139 | `has_completed()` function |
| `aptos-move/framework/aptos-framework/sources/threshold_dsa.move` | 68-93 | MPK storage |
| `testsuite/smoke-test/src/timelock/test_mpk_publication.rs` | 34-69 | The failing test |

## Debug Checklist
- [ ] Does randomness DKG complete? (look for `"[DKG] Finished DKG session"` in logs)
- [ ] Is `on_new_block()` being called in timelock module?
- [ ] Is `StartKeyGenEvent` emitted? (means gate passed)
- [ ] Does MPK serialization succeed? (check for `"MPK serialization error"`)
- [ ] Is validator authorized? (`stake::is_current_epoch_validator()`)

## Quick Hypothesis
The test environment may not be triggering randomness DKG completion, so `dkg::has_completed()` always returns `false`, blocking the entire timelock pipeline.

## Failure Points (Priority Order)

| Priority | Issue | Location | How to Check |
|----------|-------|----------|--------------|
| 1 | Randomness DKG never completes | `dkg.move:133-139` | Log: `"[DKG] Finished DKG session"` |
| 2 | Timelock DKG never starts | `timelock.move:217-240` | Log: `StartKeyGenEvent` emission |
| 3 | MPK serialization fails silently | `dkg_manager/mod.rs:425` | Log: `"MPK serialization error"` |
| 4 | Validator not authorized | `threshold_dsa.move:74` | `stake::is_current_epoch_validator()` fails |
| 5 | G2 deserialization fails | `threshold_dsa.move:80` | Returns without storing |

## Recent Fix Context (commit 59f84b7f41)
Made timelock DKG wait for randomness DKG to finish first. This prevents concurrent DKG crashes but introduces a dependency—if randomness DKG doesn't complete, timelock DKG never starts.
