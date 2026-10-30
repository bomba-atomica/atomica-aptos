# DB migration assessment: 1.38.5 node database on a 1.49.6 node

Status: static assessment. No 1.38 database was opened by a 1.49.6 node, because `aptos-node` cannot build on this host (libdw is missing, no sudo). The runtime proof is written down below as a gated test.

This answers open question 6 of the upgrade plan: can a 1.38 node database migrate to 1.49.6 without a resync.

## Verdict: conditional

In-place open is expected to work for the storage layout. It does not work for any database that holds an Atomica `TimelockShare` validator transaction, and it does not work for a database with `enable_storage_sharding: false`. For those, resync.

| Case | Verdict |
|---|---|
| Fresh-genesis testnet (docker testnet, smoke tests) | Exempt. Start from genesis on 1.49.6. |
| Long-lived network, sharded DB, no `TimelockShare` ever committed | In-place, gated on the runtime test below. |
| Long-lived network with a committed `TimelockShare` transaction | Resync (or renumber the variant first, see risk 1). |
| Any node with `enable_storage_sharding: false` | Resync. The single-DB layout is gone in 1.49.6. |

Decision for any long-lived network: do not upgrade in place until the gated runtime test passes on a copy of that network's real data directory. Take a full backup first, because opening with 1.49.6 writes new metadata keys and downgrade is untested.

## Method

Trunk (`dev-atomica`, `aptos-node` 1.38.5, `aptos-node/Cargo.toml:4`) against `baseline/aptos-node-v1.49.6`, under `storage/` plus the persisted types they decode. The script `atomica/scripts/storage_schema_diff.py` reproduces the mechanical part:

```
python3 -I atomica/scripts/storage_schema_diff.py HEAD baseline/aptos-node-v1.49.6
```

Result on this tree: 1 breaking finding (the `ValidatorTransaction` tag), everything else additive. The script exits 1 on a breaking finding only with `--strict`.

## Evidence that open works

1. **Same on-disk layout.** Ledger metadata is `ledger_db/metadata` in both (trunk `ledger_db/mod.rs:522-529`, 1.49.6 `ledger_db/mod.rs:439`). Sub-DB names are unchanged (`ledger_db/mod.rs:64-73`). State KV is `state_kv_db/shard_N` and `state_kv_db/metadata` (trunk `state_kv_db.rs:41,355-377`, 1.49.6 `state_kv_db.rs:65-71,401-415`). State merkle is `state_merkle_db/shard_N` and `state_merkle_db/metadata` (trunk `state_merkle_db.rs:49,676-691`, 1.49.6 `state_merkle_db.rs:41-47,302-315`).
2. **No existing column family is dropped from any sharded DB.** The script shows the CF lists for event, ledger metadata, persisted aux info, transaction, accumulator, aux data, info, write set, state KV (new-key) and state merkle DBs are identical. RocksDB fails to open a DB whose on-disk CFs are missing from the open list, so this matters. Only the legacy single-DB lists (`ledger_db_column_families`, `state_kv_db_column_families`) are removed, and they only apply when sharding is off.
3. **New CFs are created on open.** `storage/rocksdb-options/src/lib.rs:39-40` sets `create_if_missing` and `create_missing_column_families`. The hot state KV list gains `DB_METADATA` and `STALE_STATE_VALUE_INDEX_BY_KEY_HASH` (1.49.6 `db_options.rs`, `hot_state_kv_db_column_families`).
4. **Row codecs are byte-identical.** 20 schema codec files have identical code after stripping comments, including `jellyfish_merkle_node`, `stale_node_index`, `state_value_by_key_hash`, `stale_state_value_index_by_key_hash`, `transaction*`, `event*`, `ledger_info`. The JMT node type changed only in lints (`jellyfish-merkle/src/node_type/mod.rs`, `is_multiple_of`, unused binding).
5. **`DbMetadataKey` is append-only** (`schema/db_metadata/mod.rs:49`). BCS encodes the variant index, so appending keeps old keys readable. Twelve variants are added: three `StaleNodeCleanup*` and nine `Position*`.
6. **`WriteSet` rows decode.** Trunk persists `WriteSet` as the single-variant `ValueWriteSet::V0` (`types/src/write_set.rs:507,550-565`), hotness is not serialized. In 1.49.6 `WriteSetV0` marks `hotness` and `extensions` as `#[serde(skip)]`, so tag 0 plus `WriteSetMut` bytes are the same. The 1.49.6 decoder also carries a fallback for a legacy V1 shape (`schema/write_set/mod.rs:49-75`), which trunk never wrote.
7. **Other persisted enums are append-only** per the script: `Transaction`, `TransactionPayload` (adds `EncryptedPayload`), `TransactionInfo` (adds `V1`), `StateKeyInner` (adds `TradingNative`), `BaseStateOp`, `ContractEvent`, `TransactionAuthenticator`, `AccountAuthenticator`, `FeatureFlag` (104 to 126, deprecation renames keep position).
8. **The new native-position DBs stay off.** `ENABLE_TRADING_NATIVE` is `false` (1.49.6 `trading_native.rs:10`), so `init_native_position` does not run.

## Differences that need no action but change disk contents

- **Removed schemas.** `state_value` and `stale_state_value_index` (the legacy ledger-DB state value tables, `STATE_VALUE_CF_NAME`, `STALE_STATE_VALUE_INDEX_CF_NAME`) are removed. Only a non-sharded DB holds them.
- **Hot state moves.** Trunk keeps hot state KV inside the cold folder as `state_kv_db/hot_shard_N` (`state_kv_db.rs:358`). 1.49.6 uses separate `hot_state_kv_db` and `hot_state_merkle_db` folders and wipes them at startup (`HotStateConfig::delete_on_restart` defaults to `true`, `storage_config.rs:265`, seeded at `aptosdb_internal.rs:156-164`). The old `hot_shard_N` folders become orphans and can be deleted. The hot `HotStateValue` encoding also changed field order, which is moot because the data is wiped.
- **Stale node cleanup.** On first start 1.49.6 begins a background scan of leaked stale nodes and records `StaleNodeCleanupDone` when finished (`leaked_stale_node_cleaner.rs:25-60`, started at `state_store/mod.rs:150`). Expect extra IO on a large DB.
- **One-way open.** After 1.49.6 writes new `DbMetadataKey` rows and the hot DBs, going back to 1.38 is untested. Back up first.

## Risks and the resync cases

1. **`ValidatorTransaction` tag collision (breaking).** Trunk has `DKGResult`, `ObservedJWKUpdate`, `TimelockShare` (`types/src/validator_txn.rs:19`). 1.49.6 has `DKGResult`, `ObservedJWKUpdate`, `ChunkyDKGResult` (1.49.6 `validator_txn.rs:22`). Tag 2 changes meaning. A stored `TimelockShare` in the transaction DB, in the consensus DB block data, or in a backup will decode as a `ChunkyDKGResult` and fail or misparse. The fix is to keep upstream tags and move `TimelockShare` to tag 3 in the merge. This is a code change, so it belongs to the fork-merge work, not to this assessment. After that change, tag 3 is free of collisions only if upstream never adds a fourth variant, so pin that in the merge notes. Databases that already hold a tag-2 `TimelockShare` still need a resync.
2. **Non-sharded databases.** `enable_storage_sharding` defaults to `true` and is forced for testnet and mainnet chain IDs (`config/src/config/storage_config.rs:210,589`). The Atomica docker configs set it (`docker/compose/aptos-node/validator.yaml:26`, `fullnode.yaml:12`). A DB created with `false` has no `state_value_by_key_hash` rows and 1.49.6 has no code path for it (`LedgerDb::new` and `StateKvDb::new` no longer branch on sharding). Resync.
3. **Consensus DB and on-chain state are outside this storage diff.** `consensus/consensus-types/src/common.rs` and `payload.rs` changed heavily (1060 and 275 lines). The consensus DB can be rebuilt from the ledger on a node with intact safety data, but that was not verified. The Move framework, on-chain configs (DKG, features, gas) and genesis are upgraded by governance or a new genesis, which is a separate question from the storage layout.
4. **Not examined.** The internal indexer DB (`storage/indexer/src/db_indexer.rs`, 343 lines changed), the backup and restore tools, and secure storage. Treat an indexer DB as disposable and rebuild it.

## Gated runtime test (to run where libdw is available)

This closes the acceptance criterion that needs an actual open. Steps:

1. Run a 1.38.5 node (trunk build) on a test network, commit blocks, then stop it. Copy the data directory.
2. Build `aptos-node` from `baseline/aptos-node-v1.49.6`, or the merged fork, and start it on the copy with the same `enable_storage_sharding: true` config.
3. Pass criteria: the node opens all DBs, the synced version matches the 1.38 node, `aptos-db-tool` or the API reads an old transaction and an old state value, and the node commits new blocks.
4. Repeat once with a committed `TimelockShare` transaction. Expected result is a decode failure until the tag is renumbered.

## What was not proven

- No 1.38 database was opened by a 1.49.6 node. Everything above is from code comparison and the diff script.
- The script compares enum variant order and schema code text. It does not compare the field layout of nested structs inside each variant, so a field-level change inside a persisted struct is not covered. The write set and hot state structs were checked by hand.
