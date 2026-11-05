# Timelock parameters

The shipped `PublicParameters` of the timelock chunky DKG session. Every validator on a chain other than
the test chain (chain id 4) must load this blob, or the node refuses to start
(`types/src/dkg/timelock_dkg.rs`, `timelock_parameters_source`).

## Files

- `range-proof-prover-key.bin`: the DeKART range proof prover key (compressed arkworks encoding). It is
  the only secret-dependent part of the parameters. It was generated once with the OS random source, and
  its trapdoor was dropped.
- `timelock-public-parameters.sha256`: SHA-256 of the full blob, in `sha256sum` format.
- `timelock-public-parameters.sha3-256`: SHA3-256 of the full blob. This is the value a node puts in
  `TimelockDKGResult.public_parameters_hash` and the value governance pins with
  `0x1::timelock_dkg::set_expected_public_parameters_hash`.

The full blob is not committed. It is about 220 MB, almost all of it the dlog table, and the dlog table is
deterministic. The blob is rebuilt from the key, and the same key always gives the same bytes.

## Parameters

- `ell` = `DEFAULT_ELL_FOR_DEPLOYMENT` (32)
- `max_num_shares` = `TIMELOCK_PP_MAX_NUM_SHARES` (72)
- `max_aggregation` = `TIMELOCK_PP_MAX_AGGREGATION` (20)
- commitment base `G_2` = the standard BLS12-381 G2 generator

The parameters do not depend on the session: the `TimelockDKGSessionId` (chain id and session metadata)
is bound into each dealing, not into the parameters.

## Rebuild and check

```bash
aptos node build-timelock-public-parameters \
    --range-proof-key atomica/timelock-params/range-proof-prover-key.bin \
    --output timelock-public-parameters.bcs
sha256sum -c atomica/timelock-params/timelock-public-parameters.sha256
aptos node validate-timelock-public-parameters timelock-public-parameters.bcs \
    --expected-hash "$(cat atomica/timelock-params/timelock-public-parameters.sha3-256)"
```

The validator image (`atomica/docker/Dockerfile`) runs these steps and ships the blob at
`/opt/aptos/etc/timelock-public-parameters.bcs`. The node config templates point
`consensus.timelock_public_parameters_blob_path` there. The smoke test
`timelock_e2e::timelock_dkg_on_non_test_chain_with_shipped_parameters` rebuilds the blob, checks the
SHA3-256, and runs a timelock DKG with it on chain id 77.

A new key (`--generate-range-proof-key`) changes both hashes. Every validator must then move to the new
blob in the same epoch change, and governance must update the pinned hash.

## Trust

The key came from a single-party setup: the process that made it saw the trapdoor. Whoever knows the
trapdoor can forge range proofs, so a dealer could deal out-of-range chunks that honest validators cannot
decrypt. A multi-party ceremony would remove that trust.
