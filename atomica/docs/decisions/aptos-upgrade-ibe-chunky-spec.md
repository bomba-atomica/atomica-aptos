# IBE on Chunky DKG Spec: Hardened Boneh-Franklin Timelock

Status: Proposed. Date: 2026-10-30. Supersedes the [Option C spec](aptos-upgrade-option-c-spec.md).

This spec records a user decision that changed the timelock design direction. The decision is not reopened here. It replaces Option C (FPTX inside atomica's natives) with the cost report's Option A, hardened, with the secret moved to a chunky DKG. The cost report is the [Aptos Upgrade Cost Report](https://github.com/bomba-atomica/atomica/blob/main/docs/code-reviews/aptos-upgrade-cost-report.md), sections f and g. It sits in the `atomica` repo, so it is linked by URL.

Review packet: [aptos-upgrade-crypto-review-packet.md](aptos-upgrade-crypto-review-packet.md). Section 8 of the packet maps every earlier required change to this design. No external cryptographer has reviewed or signed anything in this spec. Review is still recommended before any release.

Code references are to the `baseline/aptos-node-v1.49.6` branch (tip `b4359185dd`) unless a path is marked `dev-atomica`. The fork code was read at the `dev-atomica` tip on 2026-10-30.

## 1. Decision summary

| Question | Decision |
|---|---|
| Mempool ciphers and secret | Not used. No FPTX, no `aptos-batch-encryption` in the product path, no `DigestKey`. |
| Source of the timelock secret | Its own chunky DKG session. The dealt public key becomes the Boneh-Franklin master public key. Each validator's decrypted chunky share becomes its master secret share. |
| Cipher for bids | Atomica's Boneh-Franklin IBE, ported to arkworks and hardened with an AES-128-GCM and one-time Ed25519 wrapper. |
| Share check | One pairing check per virtual-player share, in Rust, before the share is stored. |
| Key check | The rebuilt key must pass `e(dk, g2) = e(H1(id), mpk)` before a reveal counts. |
| Registry | Atomica's registry and validator transaction stay. They gain epoch binding, a status field, an expiry and refund path, and a DKG-derived threshold. |
| Identity | One identity per timelock, bound to chain, epoch, timelock id and deadline. One key per identity, so no rounds. |
| Epoch rule | A timelock lives in one epoch. A reveal never straddles an epoch change. |
| Retired | `scalar_elgamal`, the dual-output DKG (ADR-001), and the `blstrs` IBE code path once ported. Boneh-Franklin code is kept. |
| No longer applies | `DigestKey`, trusted powers of tau for digests, FPTX rounds, `num_rounds`, `max_batch_size`, sharding, eval proofs. |

## 2. Scope: what stays, what changes, what goes

Kept from `dev-atomica`:

- The deadline registry in `aptos-move/framework/aptos-framework/sources/ibe_config.move` (`TimelockRegistry`, `TimelockInfo`, `register_timelock`, `submit_dk_shares`, `finalize_timelock_reveal`, `on_new_block`).
- The timelock validator transaction (`ValidatorTransaction::TimelockShare` in `types/src/validator_txn.rs` lines 19 to 33, handler in `aptos-move/aptos-vm/src/validator_txns/timelock.rs`).
- The Move API names in `ibe_config.move` and `aptos-stdlib/sources/cryptography/ibe.move`, and the `ibe::reconstruct_ibe_dk` native (`aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`).
- Boneh-Franklin: `hash_to_g1`, `derive_decryption_key`, `ibe_encrypt`, `ibe_decrypt`, `verify_decryption_key` and `reconstruct_ibe_dk_from_g1_shares` in `crates/aptos-dkg/src/ibe/mod.rs` (lines 292, 328, 353, 401, 474, 624). These are ported from `blstrs` to arkworks. The port is checked against golden vectors.

Replaced:

- The key source. Today `reconfiguration_with_dkg.move` line 66 to 73 sets `mpk` from the dual-output DKG. The new source is the timelock chunky DKG session (section 4).
- The bid cipher. `ibe_encrypt` XORs a SHA3 key stream (`ibe/mod.rs` lines 353 to 440) and has no integrity check. The new cipher is the wrapper in section 6.
- Share handling. Today the handler checks only that a share is 48 bytes (`timelock.rs`, the `G1_LENGTH` check) and the registry never checks the rebuilt key. Section 5 fixes both.

Retired: `crates/aptos-dkg/src/pvss/scalar_elgamal`, the dual-output DKG wiring in `types/src/dkg/real_dkg/mod.rs`, and ADR-001. Boneh-Franklin code is not retired.

Out of scope: the auction clearing proof, and any change to the encrypted mempool.

## 3. Roles

- Bidder: encrypts a bid to the timelock identity and the epoch's `mpk`, using the wrapper in section 6.
- Auction contract: builds the associated data on chain, calls the bid verification native, rejects duplicate ciphertext ids, and stores the ciphertext. It exposes a refund path for expired timelocks.
- Registry (`ibe_config`): records the timelock, its epoch, deadline, status, verified shares and the revealed key.
- Validator node: derives its share only for a pending timelock whose deadline has passed in a committed block. It submits the share through the timelock validator transaction.
- Validator transaction handler (Rust): verifies each share before it is stored. It verifies the rebuilt key before the reveal counts.
- Anyone: reads the revealed key from chain and opens every bid under that identity.

## 4. The secret: its own chunky DKG session

### Facts from the baseline code

- Chunky DKG state is one resource, `ChunkyDKGState { last_completed, in_progress }` (`aptos-move/framework/aptos-framework/sources/chunky_dkg.move` lines 33 to 44). `start` is idempotent per `dealer_epoch` (line 64 to 93). `finish` stores the aggregated subtranscript (line 95 to 106).
- The session identifier that every dealer binds into the proof is the whole `ChunkyDKGSessionMetadata` (`dkg/src/chunky/dkg_manager/mod.rs` lines 417 to 430 pass `&session_clone.session_metadata` as `session_id`). The trait takes any serializable value (`crates/aptos-dkg/src/pvss/chunky/weighted_transcript_v2.rs` lines 119 to 137, `SokContext::new(spk, session_id, dealer.id, Self::dst())`, and `HasAggregatableSubtranscript::verify` in `crates/aptos-dkg/src/pvss/traits/transcript.rs` lines 264 to 273).
- Every dealer draws a fresh `ChunkyInputSecret::generate` per deal (`dkg_manager/mod.rs` line 413).
- The session metadata has no field that says what the secret is for. A mempool dealing and a timelock dealing for the same epoch would carry the same session id. A dealing could then be replayed from one session to the other.
- The validator's decryption key for its shares is its consensus key (`consensus/src/epoch_manager.rs` line 1204, `consensus_key.as_ref().into()` into `ChunkyDecryptPrivKey`). The mempool reads `last_completed` and requires `dealer_epoch + 1 == new_epoch` (lines 1192 to 1194).
- `AggregatedSubtranscript` holds `dealer_epoch`, the `ChunkyDKGSubtranscript` and a dealer bitmask (`types/src/dkg/chunky_dkg.rs` lines 284 to 289). Its `derive_encryption_key_bytes(tau_g2)` is FPTX-specific and is not used.
- The chunky `PublicParameters` hold the ElGamal bases, the DeKART range proof prover key, the G2 commitment base, `ell`, `max_aggregation` and the dlog table (`crates/aptos-dkg/src/pvss/chunky/public_parameters.rs` lines 46 to 70). They are loaded from a blob (`types/src/dkg/chunky_dkg.rs` lines 97 to 140).

### Decisions

1. The timelock secret is the output of a second chunky DKG session per epoch. It runs for the same dealers and the same target validator set as the mempool session. It is a separate on-chain session, with separate state.
2. The session id is a new Rust type `TimelockDKGSessionId { tag: "atomica/timelock-chunky-dkg/v1", chain_id, metadata: ChunkyDKGSessionMetadata }`, BCS-serialized by the existing generic `session_id: &A` parameter. No change to `aptos-dkg` is needed. A transcript dealt for the mempool session fails verification under the timelock session id, and the reverse.
3. A new fork-only Move module `timelock_dkg.move` mirrors `chunky_dkg.move` with its own `TimelockDKGState` resource, `start`, `finish` and view functions. The upstream file `chunky_dkg.move` is not edited. `finish_with_timelock_dkg_result` stores the aggregated subtranscript, the derived `mpk`, the epoch, the reconstruct threshold in weight units, and the per-validator weights. It replaces `ibe_config::set_mpk` in `reconfiguration_with_dkg.move`.
4. A new validator transaction variant `TimelockDKGResult(CertifiedChunkyDKGOutput)`, next to `ChunkyDKGResult`, carries the certified transcript. Its handler copies the checks of `process_chunky_dkg_result_inner` (`aptos-move/aptos-vm/src/validator_txns/chunky_dkg.rs` lines 90 to 190): epoch is current, an in-progress timelock session exists, voting power quorum, multi-signature over the aggregated subtranscript. It does not rederive an FPTX key. It rederives `mpk` from `get_dealt_public_key().as_g2()` and compares it to the claimed value.
5. The timelock session has its own `PublicParameters` blob, loaded by a new `TIMELOCK_PUBLIC_PARAMETERS` static that mirrors `PUBLIC_PARAMETERS`. It is generated by its own run of the same parameter procedure with fresh randomness and its own contributors. Its `G_2` commitment base must be the standard BLS12-381 G2 generator. The reason is that Boneh-Franklin uses `g2` as the base in `U = g2^r` and in every pairing check, and `get_public_key_share` returns `g2^s` only when the base is that generator (the same check is in `crates/aptos-batch-encryption/src/schemes/fptx_weighted.rs` setup, the `vk == G2Affine::generator() * share` loop). The node refuses to start the timelock service if the blob hash differs from the hash stored in `TimelockEpochConfig`, or if the blob fails validated deserialization.
6. If the timelock session fails or times out, the epoch change still happens. That epoch has no timelock key. `ibe_config::is_ready()` is false and `register_timelock` aborts with `E_IBE_NOT_READY`. Chain liveness does not depend on the timelock DKG.
7. Stop rule. The first implementation task proves that two sessions in one reconfiguration complete with distinct session ids and give unrelated `mpk` values. If this needs a large change to consensus or the DKG manager, work stops and goes to a cryptographer. It does not fall back to the mempool secret.

### From the DKG output to Boneh-Franklin keys

All of this uses the trait methods of `TranscriptCore` on the chunky `Subtranscript` (`crates/aptos-dkg/src/pvss/chunky/subtranscript.rs` lines 80 to 153).

| Boneh-Franklin item | Source | Reference |
|---|---|---|
| Master public key `mpk` (G2, 96 bytes) | `subtranscript.get_dealt_public_key().as_g2()` (`V0`) | `subtranscript.rs` line 104, `fptx_weighted.rs` line 251 for the same call |
| Verification key of virtual player `p` (G2) | `subtranscript.get_public_key_share(config, &player)` (`Vs[player][j] = g2^{s_p}`) | `subtranscript.rs` lines 93 to 101 |
| Validator's master secret shares (Fr, one per virtual player) | `subtranscript.decrypt_own_share(config, &player, &dk, &pp)` with `dk` the consensus key | `subtranscript.rs` lines 109 to 153 |
| Weights and threshold | The session's `WeightedConfigArkworks` (`reconstruct_threshold_in_weights`, per-validator weights) | `types/src/dkg/chunky_dkg.rs` lines 347 to 395 |

Rules:

- The master secret is never reconstructed. Only shares of `s * H1(id)` are combined.
- After `decrypt_own_share`, the node checks `vk_j == g2 * share_j` for each of its virtual players. This is the same check FPTX setup runs (`fptx_weighted.rs`, the `VKMSKMismatchError` loop). On mismatch the node does not serve the timelock for that epoch. It logs and raises an alert.
- Shares are derived again at every epoch start from the on-chain subtranscript and the consensus key, the way the mempool does it (`epoch_manager.rs` lines 1169 to 1227). No new secret is persisted by the node.
- When the DKG result is accepted, the handler also checks that `V0` equals the Lagrange interpolation in G2 of any threshold set of virtual public key shares. A subtranscript whose `V0` and `Vs` disagree is rejected. Whether upstream transcript verification already implies this is a question for the cryptographer. The check is cheap, so it is required here.
- The Shamir polynomial degree comes from the DKG. The registry threshold is therefore the DKG's `reconstruct_threshold_in_weights`, stored in `TimelockEpochConfig`. It replaces today's fixed two-thirds-plus-one of total weight (`ibe_config.move` constants `DEFAULT_REVEAL_THRESHOLD_NUMERATOR` and `DENOMINATOR`, used in `submit_dk_shares`).

### Separation from the mempool

| Item | Mempool | Timelock |
|---|---|---|
| Secret | Chunky DKG session A | Chunky DKG session B. Own dealings, own aggregation, own session id |
| Session id | `ChunkyDKGSessionMetadata` | `TimelockDKGSessionId` with the tag `atomica/timelock-chunky-dkg/v1` |
| On-chain state | `ChunkyDKGState` | `TimelockDKGState` and `TimelockEpochConfig` |
| Validator transaction | `ChunkyDKGResult` | `TimelockDKGResult` and `TimelockShare` |
| Setup material | `DigestKey` file and chunky `PublicParameters` blob | Timelock `PublicParameters` blob only. No `DigestKey`, no digest ceremony |
| What a released value opens | A mempool decryption key for one digest | A key for one timelock identity |
| Hash domains | `APTOS_BATCH_ENCRYPTION_HASH_G2_ELEMENT` (`symmetric.rs` line 150) | `ATOMICA_TIMELOCK_IBE_V2_H1`, `ATOMICA_TIMELOCK_IBE_V2_ID`, `ATOMICA_TIMELOCK_IBE_V2_KDF` |

Closed by construction: gap 4 of the cost report (one secret serves two protocols). Randomness stays on the DAS PVSS path. IBE no longer reads the secret `a` of the dual-output DKG.

What is shared, and is infrastructure rather than secret material: the aptos-dkg library code, the validator set, and each validator's consensus key as the ElGamal decryption key for its shares (the only per-validator key `ValidatorConsensusInfo` offers at 1.49.6, `types/src/dkg/chunky_dkg.rs` line 366 area). Whether that key reuse across two sessions is acceptable is question A2 in the review packet.

## 5. Boneh-Franklin shares and key checks

### Identity and hash

```text
id  = SHA3-256( "ATOMICA_TIMELOCK_IBE_V2_ID" || chain_id || epoch || timelock_id || deadline_us )
Q   = H1(id)    // RFC 9380 BLS12381G1_XMD:SHA-256_SSWU_RO_, DST "ATOMICA_TIMELOCK_IBE_V2_H1"
```

`H1` is built the way upstream builds its G1 hash (`crates/aptos-batch-encryption/src/shared/symmetric.rs` lines 150 to 170: `MapToCurveBasedHasher<G1Projective, DefaultFieldHasher<Sha256>, WBMap<G1Config>>`), with the new DST. The old identity (`compute_identity(timelock_id, deadline_us)`, `ibe/mod.rs` line 260) did not bind chain or epoch. The new one does. New golden vectors are generated. Old vectors are kept only as history.

### Share derivation

A validator holding Fr shares `s_p` for its virtual players `p` computes `dk_share_p = s_p * Q` in G1 (48 bytes compressed). It does this only under the node rule in section 8, rule 7. The node has no interface that derives a share for an identity it did not read from the registry.

### Share verification (fixes gap 1)

For each submitted virtual-player share, in this order:

1. Decode the 48 bytes with validated deserialization: on the curve, in the prime-order subgroup, and not the identity. Upstream `ark_de` does not do this (`crates/aptos-crypto/src/arkworks/serialization.rs` lines 27 to 38 call `deserialize_with_mode(..., Validate::No)`), so this step uses `Validate::Yes` explicitly. This is the carried part of earlier change B-2.
2. Check the pairing equation `e(Q, vk_p) = e(share, g2)`, as one multi-pairing `e(Q, vk_p) * e(-share, g2) = 1`. This has the same shape as upstream's `verify_shifted_bls` (`key_derivation.rs` lines 118 to 132) without the digest shift.
3. Store the share only if both checks pass.

The handler runs this in Rust inside the validator transaction handler, not as a metered Move native. The weighted check costs one pairing per virtual player, as upstream notes (`fptx_weighted.rs` lines 159 to 181, "TODO could use a multipairing"). The cost is not measured yet. It is a required measurement (section 10). No batched random-combination fast path is allowed in the first version.

A rejected share is not stored and does not take the validator's one slot. The handler drops its VM session, so nothing is stored. A fresh session emits `TimelockShareRejectedEvent` (timelock id, epoch, author) and the transaction is kept. The event is emitted only for a submission that passed the cheap registry checks (deadline passed in committed time, window open, slot free). The author is named in an unsigned transaction, so the event is a claim about a message and not proof of misbehavior: a proposer can name an honest validator. Alerts must not slash on it.

Implementation notes (`node-share-verification`):

- `aptos_dkg::ibe::verify_player_shares` computes `H1(id)` once, decodes every share and key with validation, and runs one multi-pairing per share. No batching, so the verdict is the same on every validator.
- `BonehFranklinShareVerifier` in `aptos-vm` wraps it. `aptos-node` installs it before any block executes. With none installed every share is discarded (code `0x30401`).
- The node derivation rule is `aptos_types::timelock::DerivableTimelock::authorize`: Pending, current epoch, committed block timestamp at or after `deadline_us` and before the reveal expiry, and a registry identity equal to `compute_identity(chain, epoch, id, deadline)`. Shares are derived only from that value. No wall clock is an input.
- Measured cost (release build, one core, local machine, `cargo test -p aptos-dkg --release --lib ibe::tests::measure_share_check_cost -- --ignored --nocapture`): about 2.3 ms per share, including validated decoding of the share and of the key (3.2 ms for a single share, 18 ms for weight 8, 74 ms for weight 32, 285 ms for weight 128 per author). A block that carries one `TimelockShare` per validator of total virtual weight W costs about `2.3 ms * W` of verification. Debug builds are roughly an order of magnitude slower. This is not a CI result.

### Reconstruction and key check (fixes gap 1)

- When verified weight reaches `TimelockEpochConfig.reconstruct_threshold_weight`, the handler combines the stored shares by weighted Lagrange interpolation in G1. This is the existing `reconstruct_ibe_dk_from_g1_shares` logic (`ibe/mod.rs` line 624), ported to arkworks, and kept behind the `ibe::reconstruct_ibe_dk` native.
- The handler then checks `e(dk, g2) = e(Q, mpk)` (the port of `verify_decryption_key`, `ibe/mod.rs` line 474). Today this function is only called from tests.
- `finalize_timelock_reveal` takes the checked key and repeats the same pairing check through a new native `ibe::verify_decryption_key`. This costs two pairings once per reveal. It is defense in depth, so that a handler bug cannot store an unchecked key.
- If the check passes, the status becomes `Revealed`. If it fails, every share was individually valid against the verification keys. The failure then means the `Vs` and `V0` of the DKG output disagree, or a code bug. The handler sets nothing except the status `Expired(key-check-failed)`, emits an event, and the node raises an alert. Bids become refundable. The DKG-result check in section 4 makes this case unreachable for an accepted DKG output. The branch is kept as a backstop.

## 6. Bid cipher: Boneh-Franklin with a CCA wrapper (fixes gap 2)

### Construction

For a bid plaintext `m`, with associated data `AD` built on chain:

```text
AD   = BCS( tag = "atomica/timelock-bid/v2", chain_id, epoch, timelock_id,
            auction_id, bidder, deadline_us )
(sk, vk) = fresh Ed25519 one-time key pair        // ed25519_dalek, as in upstream ciphertext/mod.rs line 14
r    = fresh random scalar
U    = g2^r
K    = e(Q, mpk)^r                                // Q = H1(id)
key  = HKDF-SHA256( K || BCS(U) || id || vk || SHA3-256(AD) )   // 16 bytes, hmac_kdf pattern of symmetric.rs lines 133 to 150
C    = AES-128-GCM( key, nonce, m, aad = AD )     // Aes128Gcm, symmetric.rs lines 29 and 81 to 130
sig  = Ed25519.sign( sk, BCS(U, nonce, C, AD) )
ct   = ( vk, U, nonce, C, sig )
ct_id = H( vk, AD )                                // upstream pattern, ids.rs line 41
```

`sk` is erased after signing. `r` is fresh for every bid, so every bid has its own AES key and a repeated nonce is not a concern.

Decryption, after the reveal, with the revealed key `dk`: `K = e(dk, U)`, derive `key` the same way, then AES-GCM decrypt with `aad = AD` read from the contract's stored bid record. A failed tag means the bid is invalid.

### Submission check

The auction contract calls a new native `ibe::verify_bid_ct(ct, AD)` for every bid. The native checks, in this order:

1. `U` passes validated deserialization (on the curve, prime-order subgroup, not the identity), `vk` and `sig` have the right lengths.
2. The associated data in the ciphertext equals the `AD` the contract built. The contract builds `AD` from the transaction sender, the epoch of the timelock, and the timelock id. A caller cannot supply a different bidder.
3. The Ed25519 signature verifies over `BCS(U, nonce, C, AD)`.

This mirrors `Ciphertext::verify` upstream (`crates/aptos-batch-encryption/src/shared/ciphertext/mod.rs` lines 103 to 125), and adds the group-membership check that upstream lacks (earlier change B-5). The contract rejects a duplicate `ct_id` and rejects any bid at or after `deadline_us`.

### What this is claimed to give, and what is not claimed

- A bid copied verbatim by another account fails the check in step 2 unless the copier rebuilds `AD`. A rebuilt `AD` changes `key` and the AES-GCM tag, so the copy decrypts to nothing.
- A bid whose `C`, `U`, `nonce` or `AD` is changed fails the signature. A copier who re-signs with a new `vk'` changes `key` (it binds `vk`), so the body no longer decrypts.
- A decrypting bid therefore needs `r`, which only the encryptor holds, in the random-oracle model for the KDF. This plaintext-awareness argument is a proof obligation for the cryptographer. It is not asserted as proven here.
- One bad bid fails alone. It cannot abort the reveal of the others, because the reveal opens one key and each bid decrypts by itself.
- The signature gives public verifiability at submission. It does not give identity-per-ciphertext as upstream does, because Atomica releases one key for a whole timelock. This is the reason the full CHK transform of upstream is not used.
- Open question for the cryptographer: whether KEM plus AEAD with the one-time key binding is enough, or whether a Fujisaki-Okamoto re-encryption check at decrypt time is also required.

## 7. Registry: what it keeps and what it adds

### Kept

Timelock id (monotonic, `next_timelock_id`), `deadline_us`, the identity, share bookkeeping (`validator_indices`, `submitted_shares`, `validator_weights`, `submitters`), the decryption key, the registration, reveal and expiry events, and `E_SHARE_ALREADY_SUBMITTED`.

### Added to `TimelockInfo`

- `epoch: u64`. The epoch the timelock was registered in.
- `status`: `Pending`, `Revealed`, or `Expired(reason)`. It replaces the `is_revealed` flag and the event-only expiry. Today `on_new_block` only emits `TimelockExpiredEvent` and changes no state (`ibe_config.move` lines 285 to 311). Today `is_expired` returns true as soon as the deadline passes (`ibe_config.move` lines 357 to 365). It means "deadline passed", so it is renamed `is_deadline_passed`. The name `is_expired` then means the `Expired` status.
- `reveal_expiry_us`. See rule 3 below.
- Expiry reasons: `epoch-ended`, `reveal-window-elapsed`, `key-check-failed`.

### Added resource: `TimelockEpochConfig`

Written once per epoch by `finish_with_timelock_dkg_result`. Fields: `epoch`, `mpk`, `reconstruct_threshold_weight`, `total_weight`, per-validator `weights` from the DKG rounding profile, `public_parameters_hash`, the hash of the aggregated subtranscript, and `verification_keys`: the G2 verification key `g2^{s_p}` of every virtual player in player order (validator `i` owns the `weights[i]` entries after the weights of validators `0..i`). `set_epoch_config` checks that there are `total_weight` keys of 96 bytes each. The handler reads the author's keys through `ibe_config::get_share_verification_keys`, so the share check uses chain state only. The handler reads weights and threshold from here. The vtxn author does not supply them. The current `submit_dk_shares` takes `weight` and `total_weight` as arguments.

### Rules

1. One reveal per timelock. `status` moves one way, from `Pending` to `Revealed` or `Expired`. It never reopens. There is no retry after `Revealed` or `Expired`.
2. A key is unique per identity, so there is no round state, no digest, and no "two keys under one secret" case. Re-releasing the same key is idempotent.
3. `reveal_expiry_us = min(deadline_us + 10 minutes, epoch_end_us)`. After it, a pending timelock becomes `Expired(reveal-window-elapsed)`.
4. Retry rules. A validator submits at most one accepted share set per timelock. A rejected share does not consume the slot, so the validator may send a corrected one until the expiry. The handler tries to finalize in the same transaction that makes verified weight reach the threshold. A failed final check is not retried (section 5).
5. Registration aborts unless `ibe_config::is_ready()` holds for the current epoch, `deadline_us > now`, and `deadline_us <= epoch_end_us - 15 minutes` (rule 2 of section 8).
6. `on_new_block` marks due timelocks `Expired` and emits one event each. It processes at most a bounded number per block in order of `(reveal_expiry_us, timelock_id)`, so a registration flood cannot make one block unbounded. The current loop walks every pending id each block. The bound is an initial value to tune by measurement.
7. Spam. `register_timelock` is a public entry with no fee or cap today (`ibe_config.move` lines 176 to 215). A deposit or cap is a product and economic decision outside this spec. It is listed as an open item for the auction contract owner.
8. Refund path. The registry exposes `get_status(timelock_id)` and the expiry reason. The auction contract must accept a refund claim for any `Expired` timelock before this ships.

## 8. Epoch bounds and the no-straddle rule

### Facts

- `IBEPublicParams` already stores `epoch` (`ibe_config.move` lines 72 to 75). `TimelockInfo` did not, so a reveal after a reconfiguration would combine shares of the wrong validator set.
- The new `mpk` and shares belong to the target validator set of the DKG, which becomes the validator set of `dealer_epoch + 1` (`chunky_dkg.move` line 32).
- The existing chunky handler already rejects a result whose epoch is not the current epoch (`chunky_dkg.rs` lines 115 to 117).

### Rules

1. Registration records `epoch` in `TimelockInfo`.
2. Registration aborts with `E_DEADLINE_AFTER_EPOCH_END` unless `deadline_us <= epoch_end_us - 15 minutes`, where `epoch_end_us = last_reconfiguration_time + epoch_interval`. The 15 minutes also cover the 10 minute reveal window of section 7, rule 3.
3. Bids are encrypted to the registered epoch's `mpk`. `AD` holds `epoch`. A bid whose `AD.epoch` is not the timelock's epoch aborts.
4. The handler accepts a share only while `current_epoch == info.epoch` and `share.epoch == info.epoch`. `TimelockShare` gains an `epoch` field.
5. When `on_new_epoch` runs, every `Pending` timelock of the old epoch becomes `Expired(epoch-ended)`, and its stored shares are dropped. No key for the old epoch is produced after that. The new validator set holds no old shares, so this is the only safe outcome.
6. A reveal never straddles an epoch. A reveal is complete only if it is finalized in a block whose epoch equals `info.epoch`. Shares of two epochs are never combined, because the stored share list belongs to one `TimelockInfo` and one epoch.
7. Node rule for deriving a share: only for a timelock with `status = Pending`, `info.epoch = current_epoch`, and a committed block timestamp at or after `deadline_us`, read from chain state. Not from the local clock, and not from a request.
8. Auctions longer than one epoch are not supported. At a 2 hour epoch the longest bid window is under 105 minutes. A longer product needs a multi-epoch design and its own spec.
9. Governance can force an early reconfiguration. The guard is best effort. A forced reconfiguration expires pending timelocks under rule 5. The auction contract must handle `epoch-ended` with a refund path before this ships.

## 9. What happens to the Option C FPTX decisions

| Option C decision | Status | Why |
|---|---|---|
| `DigestKey`, trusted powers of tau, `digest_key_hash` | No longer applies | No digest is formed. The only setup left is the chunky `PublicParameters` blob |
| Round allocation, `TimelockRoundState`, `num_rounds = 256` | No longer applies | One key per identity. No rounds exist |
| `max_batch_size = 256`, `E_BATCH_FULL`, sharding by `H(bidder, auction_id)` | No longer applies | One released key opens all bids under its identity. Any per-auction bid cap is a product parameter of the auction contract |
| Eval proofs and `FPTXWeighted::digest` | No longer applies | No batch commitment |
| Frozen bid set at the deadline (bids only strictly before `deadline_us`) | Still applies | The key opens every bid under the identity, so the contract must stop accepting bids at the deadline |
| Epoch binding, 15 minute guard, `epoch-ended` expiry, refund | Still applies | Section 8 |
| Separate secret from the mempool | Still applies | Section 4 |
| AD binding of auction and bidder | Still applies | Section 6, with `epoch` and `timelock_id` added and `shard` removed |
| `verify_ct` from `aptos-batch-encryption` | Replaced | `ibe::verify_bid_ct` in section 6 |
| `verify_decryption_key_share`, `verify_decryption_key` from `aptos-batch-encryption` | Replaced | The Boneh-Franklin checks in section 5 |

The prototype `atomica/proto-fptx-timelock/` is kept as a reference. It is not part of the product path. Its README says so.

## 10. Test plan for the implementation tasks

Rust, outside consensus first:

- Round trip: a chunky subtranscript (test dealing) gives `mpk`, `vks` and shares. Encrypt a bid, derive shares, verify each, reconstruct, check the key, decrypt.
- Share negatives: a wrong share fails the pairing check. A non-subgroup G1 point fails validated decoding before the pairing check. A share for another identity fails. A share for another epoch's secret fails.
- Key negatives: a rebuilt key from a bad share set fails `verify_decryption_key`. The handler sets nothing but the expiry status.
- Bid negatives: a flipped byte in each of `vk`, `U`, `nonce`, `C`, `sig` and `AD` fails `verify_bid_ct`. A copy under another bidder, auction, epoch or timelock fails to decrypt. A `U` outside the subgroup is rejected at submission. One bad bid does not stop the others decrypting.
- Chunky: two sessions in one epoch give different `mpk`. A transcript dealt for session A fails verification under session B's id. An oversize chunk fails verification (DeKART range proof).
- Cross-library: the arkworks port gives the same bytes as the `blstrs` code for `H1`, `U`, `dk` and `mpk` on shared vectors, so the old code can be retired against evidence.

Move:

- Registry: one-way status. Registration after the epoch guard aborts. A share with the wrong epoch is rejected. Epoch change expires pending timelocks and drops shares. The reveal window expiry works. `is_deadline_passed` and `is_expired` differ as specified.
- Auction: a bid at or after the deadline aborts. A duplicate `ct_id` aborts. Refund works for every expiry reason.

Measurements required before the design is final:

- Share verification time for the production validator weights (one pairing per virtual player).
- Cost of a second chunky DKG session per epoch, in time and transcript size.
- Time for reconstruction and the final pairing check.

## 11. Open items that need a cryptographer

These are review gates, not open design choices. They are listed in section 9 of the review packet.

- The CCA argument for KEM plus AEAD with the one-time key binding (section 6).
- Two chunky sessions in one reconfiguration with distinct session ids, and the ElGamal decryption key reuse across both (section 4).
- Whether sharing nothing but the consensus key is enough separation, and whether the timelock `PublicParameters` need their own ceremony or may share the mempool blob.
- Whether chunky subtranscript verification already implies that `V0` matches `Vs` (section 4).
- Parameters: reveal window, expiry bound per block, and the 15 minute guard.
