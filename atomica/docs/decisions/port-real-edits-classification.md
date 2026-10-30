# Real edits port classification (1.38.5 fork edits onto 1.49.6)

Feature: port-real-edits-non-timelock. Method: the trunk tree was replaced by `baseline/aptos-node-v1.49.6`, then the fork-only additions were restored. Fork edits are listed by `git diff --name-status facb34a558 <old trunk tip 848c628f2b>`, limited to modified files plus deleted non-`.github` files.

Total listed: 77 files, of which 19 are docker files that the cost report counts separately. The remaining 58 match the 62 real edits of the cost report section d.

| Class | Files |
|---|---|
| ported | 4 |
| superseded (upstream already covers it, or the edit was noise) | 24 |
| obsolete (file removed or moved upstream) | 8 |
| deferred to timelock features | 41 |

Intent of the deferred group: every edit exists to carry IBE, the dual-output DKG (scalar transcript), the `TimelockShare` validator transaction, `ibe_config` Move wiring, or their smoke tests. Features move-port-ibe-modules and natives-port-timelock re-apply them to the 1.49.6 layout (randomness_dkg.rs, chunky_dkg.rs split, new natives crate, validator transaction enum).

Fork-only files carried: `atomica/`, `.github/` (rebuilt workflows and scripts), `.githooks/pre-push`, `rust-toolchain.toml`.

Fork-only files left for the timelock features: `aptos-move/aptos-vm/src/validator_txns/timelock.rs`, `ibe_config.move`, `ibe_golden_vector_fixtures.move`, `ibe.move`, `ibe_native_test.move`, `timelock_workflow_test.move`, natives `ibe.rs` and `ibe_tests.rs` with their docs, `crates/aptos-dkg/src/ibe/`, `crates/aptos-dkg/src/pvss/scalar_elgamal/`, `testsuite/smoke-test/src/ibe/`, `testsuite/smoke-test/src/timelock/`, `testsuite/smoke-test/src/randomness/ibe_mpk_on_chain.rs`, and `crates/aptos-dkg/atomica/golden_vectors/` (same data lives under `atomica/golden_vectors/`).

Removed legacy crates: `move-cli`, `move-ir-compiler` and `move-ir-to-bytecode` are absent from the 1.49.6 tree, which closes the partial criterion left by toolchain-rust-move-update.

| File | Class | Note |
|---|---|---|
| `.cargo/config.toml` | superseded | jobs=10 is machine tuning; upstream 1.49.6 config adopted |
| `.dockerignore` | ported | adds atomica/docker/binaries include |
| `Cargo.lock` | superseded | upstream 1.49.6 lockfile adopted; timelock deps return with the timelock features |
| `Cargo.toml` | ported | release profile (debug=false, strip, thin LTO, 4 codegen units); rust-version 1.88 already upstream |
| `api/types/src/transaction.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/aptos-vm/Cargo.toml` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/aptos-vm/src/system_module_names.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/aptos-vm/src/validator_txns/dkg.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/aptos-vm/src/validator_txns/mod.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/Cargo.toml` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/aptos-framework/doc/block.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-framework/doc/genesis.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-framework/doc/overview.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-framework/doc/reconfiguration_with_dkg.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-framework/sources/block.move` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/aptos-framework/sources/genesis.move` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.move` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.spec.move` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/aptos-stdlib/doc/crypto_algebra.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-stdlib/doc/overview.md` | obsolete | generated framework docs; regenerate locally |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/crypto_algebra.move` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/cached-packages/src/aptos_framework_sdk_builder.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `aptos-move/framework/src/natives/cryptography/algebra/mod.rs` | obsolete | IBE, dual-output DKG or timelock wiring (file moved upstream) |
| `aptos-move/framework/src/natives/mod.rs` | obsolete | IBE, dual-output DKG or timelock wiring (file moved upstream) |
| `aptos-node/Cargo.toml` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `aptos-node/src/lib.rs` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `aptos-node/src/main.rs` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `consensus/src/epoch_manager.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `consensus/src/network.rs` | superseded | rustfmt reflow plus allow(dead_code); upstream file adopted |
| `consensus/src/util/mod.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-crypto/Cargo.toml` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/Cargo.toml` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/benches/pvss.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/benches/weighted_vuf.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/lib.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/das/unweighted_protocol.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/das/weighted_protocol.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/dealt_pub_key_share.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/dealt_secret_key.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/dealt_secret_key_share.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/insecure_field/transcript.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/mod.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/test_utils.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/traits/transcript.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/src/pvss/weighted/generic_weighting.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-dkg/tests/weighted_vuf.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `crates/aptos-genesis/src/config.rs` | superseded | upstream has Layout.randomness_config_override (typed enum); fork string field "V2" was dual-output specific |
| `crates/aptos/src/genesis/mod.rs` | superseded | upstream has Layout.randomness_config_override (typed enum); fork string field "V2" was dual-output specific |
| `docker/builder/README.md` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/build-indexer.sh` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/builder.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/debian-base.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/docker-bake-rust-all.hcl` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/faucet.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/forge.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/tools.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/validator-testing.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/builder/validator.Dockerfile` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/compose/aptos-node/docker-compose-fullnode.yaml` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/compose/aptos-node/docker-compose-src.yaml` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/compose/aptos-node/docker-compose.yaml` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/compose/aptos-node/validator.yaml` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/compose/validator-testnet/validator_node_template.yaml` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/image-helpers.js` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/release-images.mjs` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `docker/wait-images-ci.mjs` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
| `ecosystem/indexer-grpc/indexer-grpc-fullnode/src/convert.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `rust-toolchain.toml` | ported | kept at 1.94.1 plus rustfmt component |
| `scripts/pgo.sh` | ported | llvm-profdata-21 for Rust 1.94 |
| `testsuite/smoke-test/Cargo.toml` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `testsuite/smoke-test/src/lib.rs` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `testsuite/smoke-test/src/randomness/e2e_correctness.rs` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `testsuite/smoke-test/src/randomness/mod.rs` | deferred-timelock | testing feature and smoke-test hooks serve the IBE/timelock smoke tests |
| `types/src/dkg/mod.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `types/src/dkg/real_dkg/mod.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `types/src/validator_txn.rs` | deferred-timelock | IBE, dual-output DKG or timelock wiring |
| `docker/builder/build-node.sh` | superseded | trunk docker already matches 1.49.6; upstream tree adopted |
