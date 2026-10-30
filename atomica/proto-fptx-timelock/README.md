# proto-fptx-timelock

Standalone prototype for Option C (see `../docs/decisions/aptos-upgrade-option-c-spec.md`).
It exercises FPTX weighted batch threshold encryption (`aptos-batch-encryption`, upstream
1.49.6 by tag) for auction bids, outside consensus. It does not build `aptos-node`.

Run: `cd atomica/proto-fptx-timelock && cargo test`

The lock file is the upstream 1.49.6 lock, so `slh-dsa` resolves to a compatible release.
Setup uses `setup_for_testing` (single dealer). Production uses a chunky DKG session and a ceremony.
