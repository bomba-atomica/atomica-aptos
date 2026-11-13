# Golden Test Vectors

This folder holds two vector sets: the timelock IBE vectors and the Aptos outcome proof vectors.

## Timelock Golden Test Vectors (V2)

Golden vectors for the hardened Boneh-Franklin timelock. Design source: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` sections 4 to 6.

### Files

- `ibe_v2_golden_vectors.json` is the source of truth. It is generated, not hand written.
- `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move` is generated from the JSON.

### What the vectors cover

- Identity and H1 (v2): chain, epoch, timelock id and deadline bound into `SHA3-256`, then `H1` as RFC 9380 `BLS12381G1_XMD:SHA-256_SSWU_RO_` with DST `ATOMICA_TIMELOCK_IBE_V2_H1`.
- A seeded real chunky DKG dealing (three dealers, weights `[2, 1, 2]`, threshold weight 3). The master public key, verification keys and shares come from `get_dealt_public_key`, `get_public_key_share` and `decrypt_own_share` on the aggregated subtranscript. The JSON records the length and SHA3-256 of that subtranscript.
- `dk_share` derivation, per validator share checks (valid and invalid: swapped, other identity, non-subgroup, off-curve, identity element, truncated, count mismatch, bad verification key).
- Weighted reconstruction from several validator sets, the below-threshold error, a poisoned set, and the final key check (right and wrong).
- CCA bid ciphertexts: one valid bid and each tampered field, other associated data, non-subgroup `U` with a valid signature, a body re-signed under a new one time key, and malformed wires.
- Price bids (`price_bids`, since version 3.1.0): three bidders, each with a bid whose plaintext is its 8-byte little-endian price, and one bid whose plaintext is not 8 bytes. They come from their own seed, so the earlier bid cases keep their bytes. The product auction tests read the bidders, prices, ciphertexts and associated data (Move accessors `price_bid_bidders`, `price_bid_prices`, `price_bid_cts`, `price_bid_ads`, `bad_length_bid_ct` and `bad_length_bid_ad`), open each bid with `ibe::decrypt_bid` and `dk()`, and clear a window from the prices.
- Cross-library vectors: scalars, `H1`, `U`, `dk` and `mpk` that the blstrs code and the arkworks port must both reproduce byte for byte.

No vector depends on `aptos-batch-encryption`, FPTX digests, eval proofs or the `DigestKey` format. Encodings are ZCash compressed (G1 48 bytes, G2 96 bytes).

### Regenerating

```bash
cargo test -p aptos-dkg --lib ibe::golden_vectors::write_golden_vectors -- --ignored --nocapture
```

The run is deterministic (seeded dealing, seeded bids). It rewrites the JSON and the Move fixtures. If Move sources change, regenerate `aptos-move/framework/cached-packages/src/head.mrb` too (`cargo run -p aptos-framework -- update-cached-packages`).

Regenerate only when the scheme changes on purpose. A review must explain every changed byte.

### Checks

- Rust, `cargo test -p aptos-dkg --lib ibe::golden_vectors`: decodes the checked-in JSON and verifies it with the production code (`checked_in_vectors_verify`), checks the cross-library claim against blstrs (`checked_in_cross_library_vectors_match_both_libraries`), and checks that a fresh generation equals the checked-in bytes (`regeneration_reproduces_checked_in_bytes`).
- Move, `ibe_golden_vector_tests.move`: identity and H1 through `crypto_algebra`, pairing checks for every share case, the reconstruction and key check natives, the bid submission native, and `ibe::decrypt_bid` on every bid case and price bid.

### Consumers

`atomica/source/golden-vectors` (the repo `atomica`) holds only SIWE abstract account vectors. It does not read these files.

## Aptos Outcome Proof Vectors

`aptos_outcome_proof_vectors.json` is the source of truth for the Ethereum verifier of Aptos outcome proofs (the Aptos light client in the product repo). It is generated, not hand written. Every value is hex BCS of the production aptos-types struct.

### What the vectors cover

- A genesis epoch-ending LedgerInfo (epoch 0, version 0) and its waypoint.
- An epoch change from a 4-validator set with voting power `[1, 2, 3, 4]` (quorum 7) to a new 4-validator set with voting power `[5, 3, 2, 1]` (quorum 8), signed by exactly the old quorum.
- The same epoch 2 LedgerInfo signed by exactly the quorum (accept), below quorum (reject), with one flipped signature bit (reject) and by the previous epoch's set (reject).
- A transaction accumulator with 1500 leaves and the proof for leaf 1337, which commits the AuctionOutcome transaction.
- That transaction's five events, and the event accumulator proof for event 2: type `0xcafe::auction::AuctionOutcome` (the dev address of `atomica`).
- Intermediate values for Solidity: each CryptoHasher salt and seed, the signing message (LedgerInfo seed followed by BCS of the LedgerInfo), the RFC 9380 hash to G2 under `BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_`, and every key, aggregate key and signature in EIP-2537 uncompressed encoding.

<!-- SCOUT(#221) DOC: stale sentence. The product fixes this layout now, and the computed-deadline change keeps the `eth_lock_deadline` field, so these vectors stay valid. -->
The AuctionOutcome payload layout is not fixed in the product repo yet. The vectors pin this default: `struct AuctionOutcome { window_id: u64, pair_hash: vector<u8>, eth_lock_deadline: u64, dispositions_root: vector<u8>, leaf_count: u64 }`. Regenerate the vectors if the Move struct differs.

### Regenerating

```bash
cargo test -p aptos-types --lib atomica_outcome_proof_vectors_test::write_atomica_outcome_proof_vectors -- --ignored
```

The run is deterministic: seeded validator keys, synthetic ledger data and deterministic BLS signatures.

### Checks

`cargo test -p aptos-types --lib atomica_outcome_proof_vectors` decodes the checked-in JSON and verifies it with production aptos-types code: the epoch change from the waypoint, every accept and reject LedgerInfo, the signing intermediates (including the pairing check the EVM makes), the transaction and event inclusion proofs, and the hasher seeds. It also checks that a fresh generation equals the checked-in bytes.

### Serving live proofs

Any Atomica node serves the same structures for the live chain:

- `GET /v1/atomica/event_proof/{version}/{event_index}`
- `GET /v1/atomica/epoch_change_proof?start_epoch=0`
- `GET /v1/atomica/genesis_waypoint`

### Consumers

The product repo copies this file to `source/evm-contracts/test/fixtures/aptos/aptos_outcome_proof_vectors.json`.
