# Cross-chain testing crate vs aptos-forge 1.49.6

Date: 2026-10-30
Feature: crosschain-forge-compat (answers open question 5 of the upgrade cost report)
Question: does `atomica-crosschain-testing` still build against the upstream forge crate?

## Answer

The forge-facing code is compatible with `aptos-forge` at 1.49.6. No forge API that the crate calls changed between the pre-1.49.6 trunk and `baseline/aptos-node-v1.49.6`. The crate still does not build as a whole, for reasons that predate the upgrade and do not involve forge.

## Evidence

Source diff of `testsuite/forge` (trunk vs baseline `b4359185dd`). Every symbol the crate uses is unchanged:

| Symbol used | File in the crate | Forge definition at 1.49.6 | Changed? |
| --- | --- | --- | --- |
| `LocalFactory::from_workspace(Option<String>)` | `src/env.rs:35` | `backend/local/mod.rs:102` | no |
| `Factory::versions()` | `src/env.rs:36` | `interface/factory.rs:12` | no |
| `LocalFactory::new_swarm_with_version(..10 args..)` | `src/env.rs:48-60` | `backend/local/mod.rs:157` | no (signature identical) |
| `ActiveNodesGuard::grab(usize, Arc<Mutex<usize>>)` | `src/env.rs:46` | `backend/local/swarm.rs:728` | no |
| `LocalSwarm::validators()`, `Node::rest_api_endpoint()` | `src/tests/cross_chain_workflow.rs:19,23` | `backend/local/swarm.rs:479`, `interface/node.rs:47` | no |

Real check: `cargo check -p atomica-forge-check --tests` in a detached worktree of the baseline. The scratch member crate held the crate's `env.rs`, `tests/mod.rs`, `tests/cross_chain_workflow.rs` and `tests/timelock_e2e_smoke.rs`. Only the ethereum lines were removed, because that dependency is gone (see below). It ran against the real `aptos-forge`, `aptos-sdk`, `aptos-config`, `aptos-infallible` and `aptos-cached-packages` at 1.49.6.

- `env.rs` compiled with zero errors: the factory, swarm and guard calls are source compatible.
- Three errors remained in the test files, none caused by 1.49.6: `LocalSwarm` has no `root_account()` method, `NodeExt` was not imported for `rest_client()`, and `tests/mod.rs` declared a missing `auction_workflow` module.
- After the fix in atomica commit `99205681e715836181f2e7f9dcbee33b349f2f38` (branch `fix/crosschain-forge-compat`, not pushed), the same check finished with exit 0.
- `libdw` is missing on this host, and `aptos-forge` pulls `aptos-node` and `dw-sys` through the `aptos` crate. The check ran with a stub `libdw.pc`. `cargo check` does not link, so the stub only unblocks the build script. Nothing was linked or run.

## Local swarm NUMA and Sapling changes (1.44.7)

- NUMA: `LocalFactory` gained `cpu_affinities`, `mem_binds`, `concurrency_level` and `auto_restart`. They default to empty, `1` and `false`, and are set only through the new `with_*` builders. The crate uses none of them, so its behavior is unchanged (`backend/local/mod.rs`, `backend/local/swarm.rs`).
- Sapling: `get_worktree_revision` in `backend/local/cargo.rs` falls back to the revision `workspace` when git is absent. Git checkouts behave as before.
- Behavior changes that do reach the crate: validators now run with `blockstm_v2_enabled = true`. The `aptos-node` build that `from_workspace` triggers now uses `--features=failpoints,smoke-test` (the `indexer` feature is gone). `FORGE_BUILD_WITHOUT_INDEXER` is replaced by the compile-time `LOCAL_SWARM_NODE_FEATURES`. The `Swarm` trait gained `ensure_no_pfn_restart` and `get_pfn_rest_clients`, which matters only to code that implements `Swarm`.

## Required changes outside forge (not done here)

These block the crate from building today, on any forge version:

1. `Cargo.toml` depends on `ethereum-docker-testnet` at `../docker-testnet/ethereum-testnet/rust-sdk`. That path no longer exists. `docker-testnet/rust-sdk` is now the `atomica-docker-testnet` crate with `DockerTestnet`, which has no `EthereumTestnet`, `get_beacon_header`, `get_sync_committee`, `get_beacon_node_url` or `teardown`. `src/env.rs` and `cross_chain_workflow.rs` need a rewrite onto the new harness. This is a product decision, so it is left open.
2. `src/lib.rs` declares only `pub mod env;`, so the `tests` module is never compiled and the tests never run. Add `#[cfg(test)] mod tests;` once item 1 is fixed.
3. `Cargo.toml` tracks `branch = "dev-atomica"` of this repo. It picks up 1.49.6 only after the upgrade merges to `dev-atomica`. Pin a rev after the merge.
4. No CI workflow builds this crate.

## Fix applied

Atomica commit `99205681e715836181f2e7f9dcbee33b349f2f38` on local branch `fix/crosschain-forge-compat` (base `main` at `4fee0bf`): use `env.aptos.chain_info().root_account()`, import `NodeExt` and `Swarm`, and drop the missing `auction_workflow` module declaration.
