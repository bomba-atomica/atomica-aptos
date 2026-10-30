# Decision: atomica-sdk compatibility with the 1.49.6 node API

Status: accepted. Feature: sdk-api-client-compat. Date: 2026-10-30.

Analysis target: `source/atomica-sdk` in the atomica repo at base commit `a95d0cd` (main).
Fix branch: `fix/sdk-api-client-compat` at `e62dd6c9703cb2e1845e6306093d10f3ad5d9592` (local only, not pushed; the first fix commit is `1c858aefdd`).
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
- Fixed: `tests/sell-flow-state.test.ts` tests the `useSellFlow` hook from `atomica-web-components`. It moved to `atomica-web-components/tests/unit/hooks/` with mocks on the SDK package paths. That package's jsdom setup now installs an in-memory `localStorage` when Node's own global is unusable (this also fixed 4 older failures in `NetworkConfigProvider`).
- Follow-up feature: `sdk-auction-settled-event-source` covers the open event-source defect.

## Verification (local, not CI)
Static comparison of the 1.38 and 1.49.6 API trees plus mocked-client tests. No live node ran (libdw missing), so the live-node criterion was NOT verified.
- `source/atomica-sdk`: `bun run build`, `bun run lint -- --max-warnings=0`, `bun run format:check` pass. `bun run test`: 10 files, 122 tests passed.
- `source/atomica-web-components`: `bun run build`, `bun run lint`, `bun run format:check` pass. `bun run test:unit`: 12 files passed, 65 tests passed, 7 skipped.
- `bun scripts/check-docs.js` passes.
- Commit `1c858aefdd` contains no `dist/` files. The earlier hook bypass was undone before that commit existed.
