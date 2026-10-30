# Decision: atomica-sdk compatibility with the 1.49.6 node API

Status: accepted. Feature: sdk-api-client-compat. Date: 2026-10-30.

Analysis target: `source/atomica-sdk` in the atomica repo at base commit `a95d0cd` (main).
Fix branch: `fix/sdk-api-client-compat` at `1c858aefdd59fab9b2cc51a3af9599feca646a18` (local only, not pushed).
Node reference: atomica-aptos trunk `dev-atomica` (aptos-node-v1.49.6 plus fork files) versus the 1.38 tree at `d006a94d3f`.

## Findings

### 1. 410 on pruned reads
- 1.49.6 returns 410 with `AptosErrorCode::VersionPruned` for pruned versions, blocks, transaction ranges and events (`api/src/response.rs:676`, `:691`, `:713`; `api/src/events.rs:172`; `api/src/state.rs:44`; `api/src/blocks.rs:35`).
- Pruned events reuse `VersionPruned` on purpose, so the error code does not distinguish events from versions (`api/src/response.rs:683-689`).
- The SDK has one raw REST read: `queryAuctionSettledEvents` in `src/settlement/bridge.ts`. It treated 410 as a generic error. Fix: it now throws a typed, non-retryable `AptosPrunedReadError`.
- All other SDK reads go through `@aptos-labs/ts-sdk` (`view`, `getLedgerInfo`, `getAccountModules`, `waitForTransaction`). They surface 410 as the library's own API error. None of them reads a historic version, so none is expected to hit it.

### 2. Fat types removed
- The SDK has no use of fat transaction or event types. `grep` for `ValidatorTransaction`, `TimelockShare`, `timelock_share` over `src/` and `tests/` finds nothing. The only timelock use is the Move view functions `timelock_config::get_mpk` and `is_revealed` (`src/aptos/payloads.ts:505-545`) and the identity hash (`src/ibe/identity.ts`). Both are Move-level, not REST types.
- Confirmed unused, so nothing to remove.

### 3. TimelockShare API tag (another agent's change)
- No SDK impact. The SDK never deserializes validator-transaction payloads. Revisit only if an SDK client starts reading `/transactions` with validator-transaction variants.

### 4. Defects found that are older than 1.49 (fixed or recorded)
- Fixed: `queryAuctionSettledEvents` appended `/v1/...` to `aptosRpcUrl`, but the relayer default and the test config already end in `/v1` (`atomica-demo/scripts/settlement-relayer.ts:43`). That produced `/v1/v1/...`. The base URL is now normalized.
- Fixed: three SDK test files imported paths from before the move into `atomica-sdk` (`../../src/lib/...`). Two now resolve (`register-lock-payload`, `ethereum/storage-key`) and run.
- Open, not fixed: `AuctionSettled` is a module event (`#[event]`, `auction.move:188`, emitted with `event::emit` at `:785`). `AuctionRegistry` has no `settled_events` handle (`auction.move:178`). The by-handle REST endpoint the SDK calls can therefore only return 404, which the SDK maps to an empty list. The relayer will never see a settlement. Module events are not served by 1.49.6 REST. The fix needs the indexer (events table) or a transaction scan, which is a design change. File a follow-up feature.
- Open, not fixed: `tests/sell-flow-state.test.ts` tests the `useSellFlow` React hook, which lives in `atomica-web-components`. It cannot resolve here. Move it there.

## Verification (local, not CI)
In `source/atomica-sdk` of the fix worktree: `bun install`, `bun run build`, `bun run lint -- --max-warnings=0`, `bun run format:check`, `bun run test` (vitest browser mode, chromium), and `bun scripts/check-docs.js` from the repo root.
Result: build, lint, format and docs check pass. Tests: 122 passed, 10 files passed, 1 file failed to import (`sell-flow-state`, above).
No 1.49.6 node could run here (libdw missing), so the criterion "tests pass against a 1.49.6 node" is met only for node-free unit tests. No SDK test talks to a live node.
