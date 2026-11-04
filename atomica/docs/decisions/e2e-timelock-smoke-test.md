# End-to-end timelock run on a live local swarm: decision record

Status: Implemented on branch `upgrade/e2e-timelock-smoke-test`. Date: 2026-10-31. Prepared by an AI agent. No external cryptographer reviewed any of this.

Design sources: [IBE on chunky DKG spec](aptos-upgrade-ibe-chunky-spec.md), [review packet](aptos-upgrade-crypto-review-packet.md) sections 9 and 10, [auction V2 bid wrapper](auction-v2-bid-wrapper.md).

## 1. What ran

Before this feature the second chunky DKG session, the `TimelockDKGResult` vtxn (tag 4) and the `TimelockShare` vtxn (tag 3) had never run on a network of validators. This feature built the missing pieces and ran the flow on a real four validator local swarm (`aptos-forge` `LocalSwarm`, release build of `aptos-node` with `failpoints,smoke-test`).

Test: `testsuite/smoke-test/src/timelock_e2e.rs`, test `timelock_e2e`. Flow:

1. Genesis: validator transactions on, mempool chunky DKG on, encrypted transactions on, 30 minute epochs, timelock DKG off.
2. A governance script calls `timelock_dkg::enable` and `aptos_governance::reconfigure`. The reconfiguration runs both DKG sessions. Epoch 3 starts with a `TimelockEpochConfig` (96 byte mpk, threshold weight 3 of 4, weights `[1,1,1,1]`).
3. A client registers a timelock and three bidders encrypt V2 bids (`aptos_dkg::ibe::ciphertext::encrypt_bid`) to the timelock mpk read from chain state.
4. The timelock is revealed by the node share service. The test reads the key from chain, checks it and decrypts the bids.
5. A second timelock is registered, an epoch change is forced before its deadline, and the status is read.
6. A DKG driven epoch change gives epoch 5 with a fresh key. A third timelock is revealed under it.
7. Encrypted mempool traffic runs before the timelock bids and after the second timelock DKG.

Pieces added by this feature (none existed before):

| Piece | Where |
|---|---|
| Share-submitting service. Reads committed chain state, applies the node rule (`DerivableTimelock::authorize`), derives the node's shares and puts one `TimelockShare` vtxn per timelock into the pool. | `dkg/src/timelock_share_service.rs`, started by `dkg/src/epoch_manager.rs` when the epoch's timelock state is `Ready`, fed a `DbReader` from `aptos-node` |
| Rust mirror of the registry row and table handle | `types/src/timelock.rs` (`TimelockInfoResource`, `TimelockStatusResource`, `timelock_registry_table_handle`) |
| Failpoints `timelock_share::corrupt` and `timelock_share::hold` (test hooks, inert unless the node is built with `failpoints`) | `dkg/src/timelock_share_service.rs`, `aptos_dkg::ibe::double_g1_share` |
| Client side V2 bid tool (`identity`, `ad`, `encrypt`, `verify`, `check-key`, `decrypt`) | `crates/aptos-dkg/examples/timelock_bid.rs` |
| Shell driver against a running network | `atomica/scripts/timelock-smoke-test.sh`, companion test `timelock_testnet_for_script` |

How the service decides (spec section 8, rule 7): a timelock is served only if it is `Pending`, of the current epoch, its committed block time is at or after `deadline_us` and before `reveal_expiry_us`, and its registry identity equals `compute_identity(chain, epoch, id, deadline)`. The node's wall clock is never an input. A share is not put in the pool again once the registry lists the node as a submitter. All transactions are dropped at the epoch change (the task is aborted, the pool guards drop).

## 2. Defects the run found

### Defect 1 (fixed): every reveal with fewer than all validators failed with `key-check-failed`

Symptom (first live run): three honest shares were accepted, the handler logged `Timelock 0: rebuilt key failed the pairing check, expiring (key-check-failed)`, and the timelock expired. The DKG, the share check and the verification keys were all fine.

Cause: `ibe_config::get_reconstruction_inputs` returned `info.validator_weights` as the third value. That vector lists only the validators that submitted. The Rust reconstruction (`reconstruct_ibe_dk_from_g1_shares`) expects the weight of every validator of the epoch, indexed by validator index: it derives each share's player id from the weights before it and checks that the weights sum to the total. With 3 of 4 validators submitting the sum was 3, not 4, so the reconstruction returned `WeightSumMismatch` and the handler read that as a failed key check. The unit and VM tests all had every validator submit (the VM test harness has one validator, so it cannot submit from several), so none could see it.

Fix: `get_reconstruction_inputs` returns `TimelockEpochConfig.weights`. Regression test: Move unit test `ibe_config_tests::reconstruction_inputs_carry_the_weights_of_all_validators` (validators 0 and 2 of three submit, weights must be `[2,1,2]`). `head.mrb` regenerated with `cargo run -p aptos-framework -- update-cached-packages`.

Consequence worth stating: a bug in the handler path turned a recoverable situation into a permanent `Expired(key-check-failed)`, because there is no retry after a failed key check (spec section 7, rule 1). The spec says the branch is a "backstop" that an accepted DKG output makes unreachable. This run shows the branch is reachable through a handler bug. A reviewer should decide whether a failed Rust reconstruction (as opposed to a failed pairing check) should expire the timelock at all, or abort and leave it pending.

### Finding 2 (not a bug, a test design fact): a validator with a larger weight can block a reveal

In epoch 5 the DKG gave weights `[1, 2, 2, 1]`, total 6, threshold 5 (a later run gave `[1, 2, 2, 2]`, total 7, threshold 5). The weights change from run to run. If the corrupted validator holds weight 2 the honest weight is 4, below the threshold, and no reveal is possible while it misbehaves. The 2/3 reconstruction ratio of `OnChainChunkyDKGConfig::default_enabled()` becomes 5 of 6 after rounding. The second reveal in the test runs with the corrupted validator healed for this reason. See section 5.

### Finding 3 (design fact, tested): governance `force_end_epoch` runs no DKG

`aptos_governance::force_end_epoch` and `force_end_epoch_test_only` call `reconfiguration_with_dkg::finish` at once. The epoch after it has no timelock key (`is_ready` false), `register_timelock` aborts with `E_IBE_NOT_READY`, and no auction can open until the next DKG driven epoch change. The pending timelocks of the old epoch expire with `epoch-ended` as the spec says. Use `aptos_governance::reconfigure` to keep the timelock alive across a governance epoch change. The test asserts both behaviors.

### Observation 4 (not fixed, small): duplicate share transactions

The service stops submitting once committed state lists the node as a submitter. Between the commit of its share and the next committed-state read, a later proposer can pull the same transaction again. The registry aborts it with code 7 (`E_SHARE_ALREADY_SUBMITTED`) and the handler discards it. Seen in the logs (code 7 lines), harmless. A corrupted validator's transaction stays in its pool and is proposed again by that validator every time it proposes, which emits one `TimelockShareRejectedEvent` per proposal. The spec says the event is a claim, not proof, and must not drive slashing.

## 3. Assertion results of the live run

Command (from the worktree, with the release binaries in `e2e-target/release`, outside the repo):

```
SMOKE_TEST_PREBUILD_BINARIES=1 SMOKE_TEST_BIN_DIR=<target>/release GIT_LFS_SKIP_SMUDGE=1 \
  <target>/release/deps/smoke_test-<hash> timelock_e2e::timelock_e2e --nocapture --test-threads=1
```

Result: `test result: ok. 1 passed; 0 failed ... finished in 205.61s`. One `E2E-ASSERT` line per assertion:

| Assertion | Result |
|---|---|
| timelock-dkg-completes-in-live-network | PASS: `is_ready=true` at epoch 3 after `timelock_dkg::enable` and one DKG driven epoch change |
| mempool-mpk-differs-from-timelock-mpk | PASS: 96 byte timelock mpk, 194 byte mempool key |
| mempool-encrypted-txns-commit-with-timelock-enabled | PASS: 560 to 640 encrypted transactions committed in 15 seconds |
| timelock-registered-with-epoch-bound-identity | PASS: on-chain identity equals `compute_identity(chain, epoch, id, deadline)` |
| sealed-until-deadline | PASS: 108 to 110 samples before the committed deadline, no stored share, no key, `get_decryption_key` aborts |
| corrupted-share-is-rejected-and-takes-no-weight | PASS: the honest validators were held back, validator 3 sent a corrupted share after the deadline, a `TimelockShareRejectedEvent` names it, stored share weight stayed 0 |
| reveal-completes-after-deadline-with-honest-threshold-shares | PASS: after release, three honest shares reveal the key (threshold weight 3 of 4), validator 3 stayed corrupt |
| revealed-key-verifies-against-timelock-mpk | PASS: `e(dk, g2) = e(H1(id), mpk)` checked by the client with the mpk read from chain |
| bids-decrypt-with-the-revealed-key | PASS: all three client encrypted bids open to the original `price_le8 \|\| salt32` |
| copied-bid-fails-and-others-decrypt | PASS at the cryptographic level: the copied ciphertext fails `verify` and `decrypt_bid` under the copier's associated data (`BadSignature`), the other two bids open |
| bid-ciphertext-wire-format-roundtrip | PASS: 265 bytes |
| epoch-change-before-reveal-expires-timelock-epoch-ended | PASS: status `(pending, none)` before, `(expired, epoch-ended)` after the forced change, key not readable |
| revealed-timelock-stays-revealed-across-epochs | PASS |
| forced-epoch-end-leaves-no-timelock-key-by-design | PASS |
| second-timelock-dkg-completes-in-a-later-epoch | PASS: epoch 5, threshold 5 of 6, weights `[1,2,2,1]` |
| timelock-secrets-of-two-epochs-are-unrelated | PASS: the epoch 5 mpk differs from the epoch 3 mpk and the epoch 3 key fails the pairing check under it |
| reveal-works-under-the-second-epoch-secret | PASS: a third timelock reveals, its key verifies under the epoch 5 mpk and not under the epoch 3 mpk |
| mempool-encrypted-txns-commit-after-second-timelock-dkg | PASS |

The "C-2" case. The packet row C-2 is "a share released for one secret is invalid under another". The task text calls it "two timelock sessions in one epoch". The run covers the reading in the packet in two ways: the epoch 3 key fails under the epoch 5 mpk (two independent timelock sessions in one chain), and the timelock session and the mempool session run in the same reconfiguration with distinct mpk values. The manager level test `two_managers_in_one_epoch_produce_separate_results` already covers two managers in one epoch with a dummy network. A reading of C-2 as "two reveals of the same timelock id" is excluded by the registry (one reveal per timelock, status one way).

Shell driver: `atomica/scripts/timelock-smoke-test.sh` ran against a kept-alive swarm (`timelock_testnet_for_script`, then `FORCE_EPOCH_CHANGE=1`, `aptos` CLI and the `timelock_bid` example). Result: `TIMELOCK SMOKE TEST: PASS` with nine `ASSERT` lines (key ready, epoch bound identity, bids verify, sealed until deadline, reveal, key verifies, bids decrypt, copied bid fails, epoch change expires timelock with status 2 reason 1).

Other swarm tests run on the same binaries, all pass (the timelock DKG was off in them, the second manager idle): `chunky_dkg_correctness`, `chunky_dkg_enable_feature`, `chunky_dkg_with_validator_down`, `validator_restart_after_randomness_dkg_completion`, `randomness_correctness`, `e2e_basic_consumption`, `secret_share_rpc_path`, `test_encryption_key_rotation_and_encrypted_txns`.

Unit level: `aptos-dkg-runtime` with `--features failpoints` 42 passed (six new share service tests, the failpoint tests need the feature), `aptos-types` timelock filter 9 passed, `aptos-dkg` ibe 24 passed and 3 ignored, Move `ibe_config_tests` 44 passed. `clippy` on `aptos-dkg-runtime`, `aptos-types`, `aptos-dkg` and `aptos-node` shows no warning in the touched files (upstream crates have their own warnings, so `-D warnings` fails on them). The `aptos-vm` lib tests could not be built here (`librocksdb-sys` static library missing for that feature set).

## 4. What did not run

- The auction contract was not deployed. The "copied bid" and "refund returns the bids" assertions are demonstrated at the registry and cryptographic level only: `Expired(epoch-ended)` is shown on a live chain, `verify_bid_ct` semantics are shown with the same Rust code the native calls. The contract's `submit_bid` rejection of a copied ciphertext, and `refund_expired_window` returning collateral, were not run live. They are covered by the Move unit tests of the `atomica` repo (`auction_bid_wrapper_tests.move`). The contract depends on the framework by git revision, so deploying it needs a dependency override and a faucet and lock receipt setup that this feature did not build.
- `atomica-crosschain-testing` does not build (its ethereum dependency is gone, see `crosschain-forge-compat.md`). Its integration tests did not run.
- Docker images were not built. No Docker testnet run.
- No indexer support for the vtxns. No governance script outside the smoke test.

## 5. Judgment calls for the cryptographer

1. The 5-of-6 threshold. With stake equal across four validators the rounding gave threshold 5 of total weight 6 in one epoch, and 3 of 4 in another. Liveness of reveals then tolerates the loss of one weight-1 validator only, and none if a weight-2 validator is offline. Confirm whether `reconstruct_threshold_in_stake_ratio = 2/3` is the right input for the timelock (question C3).
2. The key-check branch (Defect 1). Decide whether a Rust reconstruction error should expire a timelock permanently or abort the transaction so that another share can retry.
3. The corrupted share in the test is `2 * share`, a valid point of the right subgroup. It is rejected by the pairing check. A share that fails decoding (not on the curve) is rejected earlier with a discard, not an event. The test covers the pairing check path only.
4. The failpoints are compiled into every build that enables `fail/failpoints` (the smoke test build). A production build without the feature has no code path to corrupt a share.

## 6. Evidence and CI honesty

Local runs, not CI. Commands and counts are recorded in the forgectl feature `e2e-timelock-smoke-test` (checklist evidence and `set-ci`).
