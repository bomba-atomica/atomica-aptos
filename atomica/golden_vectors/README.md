# Timelock Golden Test Vectors (V2)

Golden vectors for the hardened Boneh-Franklin timelock. Design source: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` sections 4 to 6.

## Files

- `ibe_v2_golden_vectors.json` is the source of truth. It is generated, not hand written.
- `legacy-v1/` holds the old vectors (identity `SHA3-256(timelock_id || deadline_us)`, blstrs shares). They are history only. No test reads them.
- `aptos-move/framework/aptos-framework/sources/ibe_golden_vector_fixtures.move` is generated from the JSON.

## What the vectors cover

- Identity and H1 (v2): chain, epoch, timelock id and deadline bound into `SHA3-256`, then `H1` as RFC 9380 `BLS12381G1_XMD:SHA-256_SSWU_RO_` with DST `ATOMICA_TIMELOCK_IBE_V2_H1`.
- A seeded real chunky DKG dealing (three dealers, weights `[2, 1, 2]`, threshold weight 3). The master public key, verification keys and shares come from `get_dealt_public_key`, `get_public_key_share` and `decrypt_own_share` on the aggregated subtranscript. The JSON records the length and SHA3-256 of that subtranscript.
- `dk_share` derivation, per validator share checks (valid and invalid: swapped, other identity, non-subgroup, off-curve, identity element, truncated, count mismatch, bad verification key).
- Weighted reconstruction from several validator sets, the below-threshold error, a poisoned set, and the final key check (right and wrong).
- CCA bid ciphertexts: one valid bid and each tampered field, other associated data, non-subgroup `U` with a valid signature, a body re-signed under a new one time key, and malformed wires.
- Cross-library vectors: scalars, `H1`, `U`, `dk` and `mpk` that the blstrs code and the arkworks port must both reproduce byte for byte.

No vector depends on `aptos-batch-encryption`, FPTX digests, eval proofs or the `DigestKey` format. Encodings are ZCash compressed (G1 48 bytes, G2 96 bytes).

## Regenerating

```bash
cargo test -p aptos-dkg --lib ibe::golden_vectors::write_golden_vectors -- --ignored --nocapture
```

The run is deterministic (seeded dealing, seeded bids). It rewrites the JSON and the Move fixtures. If Move sources change, regenerate `aptos-move/framework/cached-packages/src/head.mrb` too (`cargo run -p aptos-framework -- update-cached-packages`).

Regenerate only when the scheme changes on purpose. A review must explain every changed byte.

## Checks

- Rust, `cargo test -p aptos-dkg --lib ibe::golden_vectors`: decodes the checked-in JSON and verifies it with the production code (`checked_in_vectors_verify`), checks the cross-library claim against blstrs (`checked_in_cross_library_vectors_match_both_libraries`), and checks that a fresh generation equals the checked-in bytes (`regeneration_reproduces_checked_in_bytes`).
- Move, `ibe_golden_vector_tests.move`: identity and H1 through `crypto_algebra`, pairing checks for every share case, the reconstruction and key check natives, and the bid submission native.

## Consumers

`atomica/source/golden-vectors` (the repo `atomica`) holds only SIWE abstract account vectors. It does not read these files.
