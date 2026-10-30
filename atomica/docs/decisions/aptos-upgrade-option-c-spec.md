# Option C Spec: FPTX Inside Atomica's Natives

Status: Superseded on 2026-10-30 by the [IBE on chunky DKG spec](aptos-upgrade-ibe-chunky-spec.md). Do not implement from this document.

> Superseded. A user decision changed the design direction. The product path no longer uses FPTX or `aptos-batch-encryption`, and no longer uses the encrypted mempool's ciphers or secret. The timelock keeps atomica's Boneh-Franklin IBE, hardened, with its secret taken from its own chunky DKG session. Read [aptos-upgrade-ibe-chunky-spec.md](aptos-upgrade-ibe-chunky-spec.md). Section 9 of that spec says which decisions below no longer apply (round allocation, batch size, `DigestKey`) and which still apply (epoch bounds, expiry and refund, separate secret from the mempool). The text below is kept unchanged as the record of the earlier decision.

Original status: Proposed. Date: 2026-10-30.

This spec fixes the four open design questions from the [Aptos Upgrade Cost Report](https://github.com/bomba-atomica/atomica/blob/main/docs/code-reviews/aptos-upgrade-cost-report.md) (sections f and g, open questions 1 to 4). It lives in the Atomica docs set at `atomica/docs/` in this fork. The report sits in the `atomica` repo, so it is linked by URL. Related docs here are [ADR-001 dual-output DKG](../adr/adr-001-dual-output-dkg.md), [timelock spec](../product-spec/atomica-timelock-spec.md) and [IBE summary](../technical/ibe-summary.md).

Cryptographer review packet: [aptos-upgrade-crypto-review-packet.md](aptos-upgrade-crypto-review-packet.md). Engineering review recorded required changes (section 6a of the packet). No external cryptographer has signed off, and the human Reviewer record is still open.

Code references are to the `baseline/aptos-node-v1.49.6` branch unless a path says `dev-atomica`.

## 1. Decision summary

| Question | Decision |
|---|---|
| Round allocation | Timelock reveals use their own digest key and their own round counter. One round per auction reveal, assigned on chain, never reused within an epoch. |
| Key separation | Timelock uses a separate chunky DKG secret and a separate trusted setup. It shares nothing with the encrypted mempool. The dual-output DKG is retired. |
| Batch size | `max_batch_size = 256` bids per timelock. A larger auction is split into shards, one timelock and one round per shard. |
| Round budget | `num_rounds = 256` per epoch. |
| Epoch bounds | A timelock is bound to the epoch it was registered in. Its deadline must fall before that epoch's end minus a 15 minute guard. A reveal in a later epoch is not attempted. The timelock expires and bids are refundable. |

## 2. Scope and what stays

Kept from `dev-atomica`: the deadline registry in `ibe_config.move` (`TimelockRegistry`, `TimelockInfo`), the timelock validator transaction in `aptos-move/aptos-vm/src/validator_txns/timelock.rs`, and the Move-facing API names. The Move module is renamed only if the cryptographer review asks for it.

Replaced: the Boneh-Franklin `ibe_encrypt`, the `ibe::reconstruct_ibe_dk` native, `crates/aptos-dkg/src/pvss/scalar_elgamal`, and the dual-output DKG of ADR-001. Their replacements are `aptos-batch-encryption` (`FPTXWeighted`) and chunky PVSS from `aptos-dkg`.

Out of scope: the auction clearing proof, and any change to the encrypted mempool.

## 3. Roles of each piece

- Bidder: encrypts a bid with the epoch's timelock encryption key. Associated data (section 7) binds the ciphertext to one auction.
- Auction contract: calls a native `verify_ct` on every bid, rejects duplicates, enforces the batch cap, and stores the ciphertext.
- Registry: records the timelock, its epoch, its assigned round, its digest, and verified shares.
- Validator node: computes the digest, derives its share, and submits it through the timelock validator transaction.
- Validator transaction handler (Rust): verifies each share before it is stored.
- Anyone holding the digest key file: computes eval proofs and decrypts.

## 4. Round allocation

### Facts from the code

- `DigestKey::new(rng, batch_size, num_rounds)` draws one random `r` per round (`crates/aptos-batch-encryption/src/shared/digest.rs`). Each round has its own randomized powers of tau.
- `FPTXWeighted::digest(digest_key, cts, round)` fails when `round >= num_rounds`.
- The mempool tracks a dense counter, `next_decryption_round` in `decryption.move`. It bumps only when a block produces a key. `on_new_epoch` resets it to 0. The decryption pipeline marks transactions `TrustedSetupExhausted` when the counter reaches `num_rounds` (`consensus/src/pipeline/decryption_pipeline_builder.rs`).
- Production mempool setup is not visible in the tree. The test setup uses 32 by 200.

### Rules

1. Timelock reveals do not draw from the mempool counter. Sharing it would let mempool traffic exhaust reveals, and the reverse. It would also need a consensus change.
2. A new resource `TimelockRoundState { epoch, next_round }` lives in the registry module. `on_new_epoch` resets `next_round` to 0. This mirrors `PerBlockDecryptionKeyV2`.
3. The round is assigned by the block prologue hook `on_new_block`, not by validators. When a timelock's deadline has passed, the hook processes due timelocks in order of `(deadline_us, timelock_id)`. Each gets `assigned_round = next_round`, then `next_round` increases by 1. The result is deterministic and the same on every validator.
4. Assignment happens at reveal start, not at registration. A ciphertext does not depend on the round, only the digest does, so bidders need no round when they encrypt.
5. A round is bound to one digest. The digest is a function of the sorted set of verified ciphertext IDs, and that set is frozen at the deadline (section 5). A reveal retry reuses the same round and recomputes the same digest. The handler rejects any share that does not match the stored digest. No second digest is ever formed for a round under the same secret.
6. Exhaustion: when `next_round >= num_rounds`, the timelock is marked expired with reason `rounds-exhausted`. No key is produced and bids are refundable. The registry emits an event and the node exports a gauge `timelock_rounds_remaining`. An alert fires below 25 percent.
7. Rounds are not returned. A failed reveal keeps its round until the epoch ends.

### Budget

Atomica runs two auctions a day (PRD). Epochs last 2 hours in the docker testnet config. Even with ten markets, sharded four ways, one epoch needs at most 40 rounds. 256 rounds gives a margin of more than six. If a real deployment exceeds this, the setup ceremony is redone with a larger `num_rounds`. Rounds are not made elastic.

## 5. Batch size

- `max_batch_size = 256`. The library requires a power of two (`DigestKeyInitError::BatchSizeMustBePowerOfTwo`), and `IdSet::with_capacity` rounds up to one.
- Basis: the timelock bid docs put real auctions at tens to hundreds of bids. The cap bounds per-reveal cost. Eval proof computation is the most expensive step in the scheme, and it grows faster than linearly in batch size.
- The auction contract enforces the cap at bid submission with a new abort `E_BATCH_FULL`. It does not rely on the node failing later.
- Sharding: an auction that may exceed 256 bids registers `ceil(expected_bids / 256)` timelocks up front. A bid picks a shard by `shard = H(bidder, auction_id) mod shard_count`, so bidders cannot crowd one shard cheaply by choice. The clearing step combines the decrypted shards. Each shard uses one round.
- The set is frozen at the deadline. A bid is accepted only if the block timestamp is strictly before `deadline_us`. After that the batch is final, so the digest is stable.
- Setup size: 256 by 256 randomized G1 points is about 3.1 MB plus the Toeplitz domain, so the digest key file stays small enough to ship with the node.

## 6. Key separation

### Decision

The timelock has its own chunky DKG secret and its own digest key. Neither is shared with the mempool.

| Item | Mempool | Timelock |
|---|---|---|
| Secret | Chunky DKG session A | Chunky DKG session B, separate transcript and separate dealings |
| Digest key | Mempool file, one trusted setup | Own file, own trusted setup, own `tau` and `r` values |
| Counter | `PerBlockDecryptionKeyV2` | `TimelockRoundState` |
| Public parameters on chain | `PerEpochEncryptionKey` | New `TimelockEpochConfig` |
| Share domain | none | one-time key AD tag `atomica/timelock-bid/v1` (section 7) |

Randomness stays on the DAS PVSS path. IBE no longer shares the secret `a` with it. This closes security gap 4 by construction, not by a domain separation argument. ADR-001 is superseded by this spec once the work lands.

### Why not share the mempool secret

A shifted BLS key share has the form `(digest + H(mpk)) * share`. Under one secret, a share released for a mempool digest is a valid share for any timelock digest that equals it. Different digest keys make an accidental match negligible. Whoever knows the trusted setup trapdoors can still craft one. Keeping the secrets apart removes that class of cross-use. It costs one extra DKG session per epoch. That cost is accepted.

### Implementation risk and the stop rule

Chunky DKG at 1.49.6 is wired for one encryption session. The first prototype task is to prove two concurrent sessions can run with distinct session identifiers. If that cannot be done without a large consensus change, work stops and goes to a cryptographer. It does not fall back to a shared secret.

### Trusted setup handling

- Timelock `TimelockEpochConfig` stores `digest_key_hash` (SHA3-256 of the file header and points). The node refuses to start the timelock service if the file hash differs.
- The ceremony is a randomized powers-of-tau contribution (`DigestKey::with_randomized_powers_of_tau`) with at least one contributor outside the core team. Toxic waste handling is recorded in the ceremony log.
- The DigestKey deserialization skips point validation upstream (1.45.4). The node must run a full validation once at startup for the timelock file.

## 7. Security properties and how they are met

### Non-malleability (gap 2)

- Every bid is an FPTX ciphertext: batch-IBE wrapped with a one-time Ed25519 signature and AES-128-GCM.
- Associated data is `BCS(tag = "atomica/timelock-bid/v1", chain_id, epoch, auction_id, shard, bidder, deadline_us)`.
- The bid contract calls `verify_ct(ct, ad)` through a native before storing. A flipped or re-bound ciphertext fails verification. A copy submitted by another bidder fails because the bidder is in the associated data.
- The ciphertext ID is derived from the one-time verification key and the associated data. The contract rejects duplicate IDs.

### Bad-share rejection (gap 1)

1. The handler in `timelock.rs` parses the share as a weighted decryption key share and runs `verify_decryption_key_share` against that validator's verification key and the stored digest. A malformed or wrong share is rejected and not stored.
2. Verification runs in Rust in the validator transaction handler. It does one pairing per virtual player. This must not be a metered Move native.
3. After the threshold of verified shares, the handler reconstructs the key and runs `verify_decryption_key` against the epoch's public key. Only then does it set `is_revealed`.
4. If the final check fails, the handler sets nothing. The timelock stays pending and other shares can still arrive. A single bad validator can no longer lock a reveal for good.
5. A rejected share raises an event naming the validator. Repeated rejects feed the validator performance report.

### Range proofs (gap 3)

Timelock uses chunky PVSS, which carries DeKART range proofs for the encrypted chunks. A dealer that encrypts oversize chunks fails transcript verification. `scalar_elgamal` is deleted, not patched.

### Domain separation (gap 4)

Closed by section 6.

## 8. Epoch bounds

### Facts from the code

- `IBEPublicParams` on `dev-atomica` already stores `epoch`. `TimelockInfo` does not, so a reveal after a reconfiguration would combine shares of the wrong validator set.
- FPTX secrets are per epoch. The mempool counter resets per epoch. `on_new_epoch` applies buffered keys.

### Rules

1. Registration records `epoch` in `TimelockInfo`.
2. Registration aborts with `E_DEADLINE_AFTER_EPOCH_END` unless `deadline_us <= epoch_end_us - 15 minutes`, where `epoch_end_us = last_reconfiguration_time + epoch_interval`. The guard leaves room for share collection. It covers a reconfiguration that is a little early.
3. Bids are encrypted to the registered epoch's encryption key. The associated data holds `epoch`. A bid for a different epoch aborts at submission.
4. The reveal runs only while `current_epoch == info.epoch`. The handler rejects shares carrying another epoch.
5. If `current_epoch != info.epoch` and the timelock is unrevealed, the epoch hook marks it expired with reason `epoch-ended`. Bids are refundable. No key for the old epoch is produced. The new validator set holds no old shares, so this is the only safe outcome.
6. `TimelockRoundState` resets in the same `on_new_epoch` call. Round numbers in different epochs never share a secret, so reuse across epochs is safe.
7. Auctions longer than one epoch are not supported. An auction window must fit inside one epoch with the guard. At a 2 hour epoch the longest bid window is under 105 minutes. A longer product needs a multi-epoch design and its own spec.

### Early reconfiguration

Governance can force a reconfiguration at any time. Treat the guard as best effort. A forced early reconfiguration expires pending timelocks under rule 5. The auction contract must handle `epoch-ended` with a refund path before this ships.

## 9. Test plan for the implementation task

- Prototype in Rust outside consensus: encrypt bids, compute a digest at round `r`, derive and verify shares, reconstruct, decrypt.
- Negative tests: flipped ciphertext bytes fail `verify_ct`. A bid copied under another bidder fails. A share with a wrong value is rejected. A final key with a bad share mixed in is not stored. Two distinct digests assigned one round are rejected. A reveal after an epoch change expires. A round at `num_rounds` expires. A bid at count 257 aborts.
- Chunky DKG: two sessions in one epoch yield different secrets and public keys. An oversize chunk fails verification.
- Costs to measure: eval proofs for 256 bids, share verification for the production validator weights, digest key load time.

## 10. Review checklist

This spec answers the report's open questions as follows.

| Report question | Answer | Section |
|---|---|---|
| 1. Round allocation | Separate digest key, own counter, one round per reveal | 4 |
| 2. Separate secret | Yes, separate chunky DKG session | 6 |
| 3. Batch size | 256, sharded beyond that | 5 |
| 4. Epoch crossing | Not supported. Bound to one epoch, expire and refund otherwise | 8 |

Residual items that need a cryptographer before code ships: the two-session chunky DKG, the AD layout, and the trusted setup ceremony. They are review gates, not open design choices.
