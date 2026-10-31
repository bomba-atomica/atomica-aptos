# Decision: AuctionSettled event source is a transaction scan with a version checkpoint

Status: accepted. Feature: sdk-auction-settled-event-source. Date: 2026-10-31.

Atomica repo: fix commit `a15fe1c` (`fix(sdk): read AuctionSettled module events by transaction scan with version checkpoint`), merged to local `main` as `1336928`. Local only, not pushed.

## Problem
`AuctionSettled` is a module event (`#[event]`, `auction.move:188`). `AuctionRegistry` has no `settled_events` handle (`auction.move:178`). The by-handle endpoint the SDK called could only return 404, which the SDK mapped to an empty list. The relayer never saw a settlement. See `sdk-api-client-compat.md`.

## Options
- Indexer GraphQL events table. Rejected: neither `atomica-sdk` nor `atomica-demo` depends on an indexer, and the docker testnet does not run one. It would add infrastructure.
- `/v1/accounts/{addr}/transactions`. Rejected: it lists transactions sent by one account, and the settle sender is not the contract address.
- `GET /v1/transactions?start={version}&limit={n}` (chosen). It is served by the stock 1.49.6 REST API, needs no new node feature, and returns each transaction's `events`, where module events appear.

## What changed (atomica repo)
- `source/atomica-sdk/src/settlement/bridge.ts`: new `scanAuctionSettledEvents(config, {fromVersion, pageSize, maxPages})`. It pages from `fromVersion`, matches events by type `{contractAddress}::auction::AuctionSettled` (leading zeros of the address normalized), and stops at a short page (ledger head) or `maxPages`. It returns `{events (with version), nextVersion}`. `nextVersion` is the checkpoint. `queryAuctionSettledEvents` keeps its signature (plus an optional `ScanOptions`) and now calls the scan. `samePair` is exported.
- 410 is still surfaced as `AptosPrunedReadError` (the URL names the start version). 404/400 return what was found so far. Other statuses throw. The by-handle endpoint is gone.
- `source/atomica-demo/scripts/settlement-relayer.ts`: the loop keeps a version cursor (`START_VERSION`, optional `SETTLEMENT_CURSOR_FILE`) in place of probing window ids. The cursor moves only after every event in the scan was relayed. A failed submission leaves it in place and is retried. A pruned read is rethrown to `main`, which exits non-zero: the relayer does not retry it. The operator must set `START_VERSION` above the pruned range.

## Tests and checks (local, not CI)
- `source/atomica-sdk/tests/settlement-bridge.test.ts` with a mocked `fetch`: finds settlements in transaction events, ignores other events and other package addresses, handles zero-stripped addresses, pagination across pages, `maxPages` stop, resume from `fromVersion`, unchanged checkpoint when nothing is new, 404, 410, 500, single `/v1` path.
- Built `state-proofs/typescript` first (`bun install && bun run build`: its `tsc` emitted `dist`, its second `tsc --project tsconfig.check.json` failed on a missing `@atomica/ethereum-docker-testnet`, which is unrelated). Then `atomica-sdk`: `bun run build`, `bun run lint -- --max-warnings=0`, `bun run format:check` pass. `bun run test`: 10 files, 130 tests passed (was 122).
- `bun scripts/check-docs.js` passes.
- `atomica-demo`: the script is Prettier-clean and type-checks with `tsc --noEmit`. ESLint ignores `scripts/`. `bun run test` in `atomica-demo` fails before running any test because `prepare:deps` leaves a web-components `dist` that does not resolve `./components/SettlementStatus`. It is unrelated to this change and has no settlement-relayer tests.
- Tracked `dist/` files modified by builds were restored before committing.

## Not verified
No live 1.49.6 node ran here. The criterion "the relayer picks up a settled window on a local testnet end to end" is pending. A live run should also confirm that the node returns module events inside `/v1/transactions` (`events[].type` carries the full type tag) as the 1.49.6 API source indicates.

## Limits
A scan from genesis is O(chain length), so a fresh relayer should set `START_VERSION` near the deploy version. The cursor file is the restart checkpoint.
