# proto-fptx-timelock

Status: superseded for the product path on 2026-10-30. Kept as a reference only. The product design is now hardened Boneh-Franklin IBE on a separate chunky DKG secret, see `../docs/decisions/aptos-upgrade-ibe-chunky-spec.md`. Nothing in the product path may depend on this crate or on `aptos-batch-encryption`. The tests still run in CI so the reference stays buildable and the review packet's evidence stays reproducible.

Standalone prototype for Option C (see `../docs/decisions/aptos-upgrade-option-c-spec.md`).
It exercises FPTX weighted batch threshold encryption (`aptos-batch-encryption`, upstream
1.49.6 by tag) for auction bids, outside consensus. It does not build `aptos-node`.

Run: `cd atomica/proto-fptx-timelock && cargo test`

The lock file is the upstream 1.49.6 lock, so `slh-dsa` resolves to a compatible release.
Setup uses `setup_for_testing` (single dealer). Production uses a chunky DKG session and a ceremony.
