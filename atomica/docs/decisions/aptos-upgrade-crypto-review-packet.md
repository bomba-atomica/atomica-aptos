# Option C Cryptographer Review Packet

Status: Awaiting human review (revised for the new design, see section 8). Prepared 2026-10-30 by an AI agent for forgectl feature `crypto-review-signoff`.

This packet is evidence for a human cryptographer. It is not a review and it is not a sign-off. No cryptographer has reviewed anything in it. The agent that wrote it checked code and ran tests. It did not do a cryptographic proof. Every conclusion below is a claim to be confirmed or refuted by the reviewer.

The forgectl feature `crypto-review-signoff` is closed on the engineering outcome in section 6a (required changes recorded, no external sign-off). The human Reviewer record in section 7 stays open.

Revision notice (2026-10-30): the design direction changed after this packet was written. Sections 0 to 7 below review the superseded Option C (FPTX) design and are kept as the record. Section 8 maps each earlier required change to the new design. Section 9 holds the open questions for the new design. The spec now under review is the [IBE on chunky DKG spec](aptos-upgrade-ibe-chunky-spec.md). Section 7, the Reviewer record, is unchanged and blank. No external cryptographer has signed anything, and review is still recommended before release.

Superseded spec (sections 0 to 7): [Option C spec](aptos-upgrade-option-c-spec.md). Prototype: `atomica/proto-fptx-timelock/` in this fork. Upstream code: branch `baseline/aptos-node-v1.49.6` (tip `b4359185dd`, "Bump version to 1.49.6"). Paths in section 1 to 4 are relative to the repo root on that branch unless marked `proto:`.

## 0. What to know first

- Curve: `aptos-batch-encryption` uses BLS12-381 (`crates/aptos-batch-encryption/src/group.rs`, lines 3 to 6). BN254 is only a commented-out alternative. G1 and G2 both have cofactor greater than 1, so a missing subgroup check is a real gap, not a formality.
- Scheme: FPTX weighted batch threshold encryption. A bid is an IBE-style ciphertext whose identity is `H(one-time Ed25519 verifying key, associated data)`. A batch of identity values is committed as a KZG-style digest in G1. A decryption key is a threshold "shifted BLS" signature on `digest + H(mpk)`. An eval proof opens the digest at one identity.
- Prototype scope: it exercises the library with `setup_for_testing` (one dealer, seeded RNG). It does not exercise chunky DKG, the digest key file loader, epochs, or the on-chain registry. Section 6 lists what no test covers.
- Test run for this packet: `cd atomica/proto-fptx-timelock && cargo test --offline` gave 11 passed, 0 failed (3.14 s test time, optimized profile). This is a local run, not CI.

Prototype test names (all in `proto:tests/timelock.rs`):

1. `round_trip_encrypt_share_verify_combine_decrypt`
2. `threshold_subset_reveals_and_any_subset_gives_same_key_result`
3. `below_threshold_shares_cannot_reveal`
4. `duplicate_share_does_not_count_twice`
5. `bit_flipped_ciphertext_is_rejected_at_every_byte`
6. `ciphertext_rebound_to_other_bidder_auction_epoch_or_shard_is_rejected`
7. `corrupted_or_misattributed_share_fails_verification_before_combination`
8. `unverified_bad_share_poisons_combination_and_final_check_catches_it`
9. `wrong_key_rejection_separate_secrets_do_not_cross_decrypt`
10. `full_256_bid_batch_reveals_and_257th_is_refused`
11. `round_at_num_rounds_is_refused_and_rounds_bind_digests`

## 1. Question A: separate secret versus shared mempool secret

### Claim

The timelock uses its own chunky DKG session (secret B), its own digest key (own `tau`, own per-round `r`), and its own on-chain parameters. It shares no secret, no digest key, and no counter with the encrypted mempool (spec section 6). The dual-output DKG of ADR-001 is retired.

### Threat model

- Adversary: a bidder or outside party who can submit transactions to the mempool and to the auction, observes all on-chain data, and sees every decryption key the chain publishes. Up to the threshold weight of validators is honest. Less than the threshold is malicious.
- Assets: sealed bids until the deadline. Mempool transactions until their block.
- Goal of the adversary: obtain a decryption key that opens a timelock bid early, using a key or share the network legitimately released for the mempool.
- Out of scope here: anyone who knows a digest key trapdoor (`tau` or the per-round `r` values). That is the ceremony question in section 3.

### Exact references

- Spec: section 6 (table, "Why not share the mempool secret", "Implementation risk and the stop rule"), section 4 rule 1.
- Key shape: a decryption key is `(digest + H(mpk)) * s`, a BLS signature on the shifted digest. See `crates/aptos-batch-encryption/src/shared/key_derivation.rs`, `derive_decryption_key_share` (line 106) and `verify_shifted_bls` (line 118). The hash is `symmetric::hash_g2_element(mpk_g2)`.
- Encryption key: `EncryptionKey { sig_mpk_g2, tau_g2 }`, `shared/encryption_key.rs` line 24.
- Reconstruction: `schemes/fptx_weighted.rs` `reconstruct_decryption_key` (near line 390) calls `BIBEDecryptionKey::reconstruct`.
- Final key check: `EncryptionKey::verify_decryption_key`, `encryption_key.rs` line 28, runs `verify_shifted_bls(sig_mpk_g2, digest, sig_mpk_g2, signature)`.
- Setup in the prototype: `FPTXWeighted::setup_for_testing`, `schemes/fptx_weighted.rs` line 306. Wrapped by `proto:src/lib.rs` `TimelockSetup::for_testing`.
- The stop rule (two concurrent chunky DKG sessions with distinct identifiers) is untested. The prototype has no DKG.

### Tests that exercise it

- `wrong_key_rejection_separate_secrets_do_not_cross_decrypt`: builds two setups from seeds 10 and 11, asserts the encryption keys differ, shows A reveals A's bids, shows B's honest shares over A's ciphertexts do not open them, and shows A's key does not pass `verify_key` under B.
- Related: `round_trip_encrypt_share_verify_combine_decrypt` and `below_threshold_shares_cannot_reveal` cover the scheme itself.

Limits of that test, stated plainly:

- It proves only that two independent seeds give independent keys. That is expected from any correct setup.
- It never demonstrates the attack that separation is meant to stop. There is no test that takes a share released for a "mempool" digest and shows it is (or is not) valid for a timelock digest under a shared secret.
- The cross-decrypt branch is guarded by `if let Ok(rv_b) = rv_b`, so if `Reveal::new` fails the assertion is skipped. A reviewer should treat this as weak evidence.
- The two setups also have different `tau`, so the test cannot isolate "shared secret, different digest key" from "different everything".

### Adversarial analysis

1. Cross-use under a shared secret. A share is a signature on `digest + H(mpk)`. With one `mpk`, any G1 point the network signs for the mempool is a valid share input for the same point as a timelock digest. The two digests are built from different digest keys, so a collision needs the trapdoors. The spec relies on that. The question for the reviewer is whether separation is needed at all, or only a domain tag, and whether the cost (a second DKG per epoch) is justified. The spec chose separation. This packet does not argue against it.
2. Bidder-chosen digests. If any validator ever signs a digest it did not derive itself, the adversary can craft a digest `D'` that commits to a victim's identity. Spec section 4 rule 5 says nodes derive the digest from the frozen set and the handler rejects any other. This is an implementation rule and no prototype test enforces it at the handler level. The nearest test is `corrupted_or_misattributed_share_fails_verification_before_combination` case 3 (share for another digest fails verification).
3. Shared `tau_g2` and `mpk` binding. `EncryptionKey` binds `mpk` and `tau_g2`. Two sessions that reuse one `tau_g2` would share a digest key. The spec says separate setups. Nothing in the code forces it. `TimelockEpochConfig.digest_key_hash` (spec section 6) is the intended guard and is not implemented.
4. Two concurrent chunky DKG sessions. Unknown at 1.49.6. A session identifier collision or reused dealing across sessions could make secrets correlated. This is the stop-rule gate. Nothing has been attempted.
5. Epoch rollover. A new epoch has a new validator set and new secret. Old shares are useless. The spec expires the timelock. Not tested.
6. Rogue-key and dealer bias in DKG. The mpk is the dealt public key of the aggregated subtranscript (`fptx_weighted.rs` line 300 area, `get_dealt_public_key`). Whether chunky PVSS gives an unbiased key under the weighted setting is a property of aptos-dkg, which is outside this packet.

### Questions for the cryptographer

- A1. Is a separate secret required for security, or would a distinct `mpk`-shift or domain tag on a shared secret be sufficient? If shared, what exact tag must be bound into the signed message?
- A2. Under separate secrets and separate digest keys, is there any residual cross-protocol use of a released share or key?
- A3. Is running two chunky DKG sessions in one epoch sound at 1.49.6 (session identifiers, transcript domain separation, dealer reuse)? What is the minimum evidence required before code ships?
- A4. Is the binding of the secret to the digest key adequate (`EncryptionKey` holds `mpk` and `tau_g2`, the key file holds `tau_g2`)? Should `mpk` be bound to the digest key hash in the on-chain config?
- A5. Is `wrong_key_rejection_separate_secrets_do_not_cross_decrypt` sufficient evidence, or what test would you require?

## 2. Question B: DigestKey deserialization skips validation

### Claim (from the spec)

Spec section 6, "Trusted setup handling": "The DigestKey deserialization skips point validation upstream (1.45.4). The node must run a full validation once at startup for the timelock file."

### What the agent found in the real code

The claim is confirmed, and it is broader than the spec says. The version in the spec (1.45.4) was not checked. The behavior is present at 1.49.6.

1. The file reader skips validation on every point.
   - `crates/aptos-batch-encryption/src/shared/digest_key_file.rs`, `read_round`, lines 258 to 273.
   - Line 260: `G1Affine::deserialize_with_mode(file, Compress::No, Validate::No)` for every `tau_powers_g1` entry.
   - Line 265: `G1Projective::deserialize_with_mode(file, Compress::No, Validate::No)` for every `prepared_toeplitz_input` entry.
   - With `Compress::No` and `Validate::No`, arkworks reads x and y directly. The agent expects no on-curve check and no subgroup check. The reviewer should confirm this against the pinned `ark-serialize` and `ark-ec` (the patched fork `aptos-labs/algebra` branch `fix-fft-parallelism-cutoff`).
2. The header is not validated either. `HeaderV1.tau_g2` and `fft_domain` use `ark_de` (`digest_key_file.rs` lines 36 to 40).
3. `ark_de` itself uses `Validate::No`, despite its doc comment.
   - `crates/aptos-crypto/src/arkworks/serialization.rs`, `ark_de`, lines 27 to 38.
   - The doc comment says it "attempts to check its correctness". The body calls `deserialize_with_mode(..., Compress::Yes, Validate::No)`.
   - `ark_de` is the serde deserializer for the network-facing types: `Digest.digest_g1`, `EvalProof`, ciphertext `ct_g2`, `BIBEDecryptionKeyShareValue`, `EncryptionKey`, `WeightedBIBEVerificationKey`, `WeightedBIBEMasterSecretKeyShare`. See `shared/digest.rs` lines 37 and 269, `shared/ciphertext/bibe.rs` lines 45 to 56, `shared/key_derivation.rs` lines 26 to 57, `shared/encryption_key.rs` lines 17 to 50, `schemes/fptx_weighted.rs` lines 48 to 126.
   - Compressed decoding recovers y from x, so a decoded point is on the curve. It is not checked to be in the prime-order subgroup. For BLS12-381 this leaves room for small-subgroup or cofactor components in G1 and G2.
4. No validation runs after load.
   - `types/src/dkg/chunky_dkg.rs`, `DIGEST_KEY` (lines 205 to 231): `read_digest_key`, log the elapsed time, return the key. No point check, no pairing consistency check, no hash check.
   - `dkg/src/lib.rs` lines 94 to 98: loads the key in a thread and publishes a blake3 of the file as a metric. The hash is reported, not compared to an expected value.
   - `read_digest_key_v1` (lines 206 to 223) checks only the file size against the header.
   - `From<(HeaderV1, Vec<Round>)> for DigestKey` (line 152) is commented "Assumes correct shape". It does not check that `prepared_toeplitz_input` matches `tau_powers_g1`.
5. The "validate" command validates almost nothing.
   - `crates/aptos/src/node/mod.rs`, `ValidateDigestKey`, line 1540. At line 1561 it calls `read_digest_key` and prints "DigestKey is valid". That is the same unvalidated loader, so "valid" means only "parses and has the right size".
6. Upstream tests do not cover it. `digest_key_file.rs` tests (line 281 onward) are round-trips of a freshly generated key plus range and out-of-bounds panics. There is no negative test with a corrupted point.
7. `DigestKey::with_randomized_powers_of_tau` (`shared/digest.rs` lines 115 to 160) checks shape only: non-empty, power-of-two batch size, equal row lengths. It does not check points.

The spec's mitigation ("full validation once at startup") is not implemented anywhere. The existing validator command would not provide it.

### Threat model

- The digest key file is operator-supplied, local, and loaded at startup. It is not read from the network.
- Adversary A (file supply chain): swaps or corrupts the file in distribution (the node publishes only a blake3 metric).
- Adversary B (ceremony contributor): contributes a malformed or malicious update. The file is the output of the ceremony.
- Adversary C (network): sends points through `ark_de`-decoded types (ciphertexts, shares, digests). This is a different, related issue (adversarial items 6 and 7 below).
- Asset: confidentiality of bids and liveness of reveals. A wrong `tau_powers_g1` changes digests and eval proofs. A wrong `tau_g2` changes the encryption key and the proof check `verify_pf` (`shared/digest.rs` line 202).

### Exact references

- `digest_key_file.rs` lines 258 to 273 (`read_round`), 152 to 174 (`From`), 206 to 223 (`read_digest_key_v1`).
- `crates/aptos-crypto/src/arkworks/serialization.rs` lines 27 to 38, 52 to 61.
- `types/src/dkg/chunky_dkg.rs` lines 205 to 231.
- `dkg/src/lib.rs` lines 94 to 98.
- `crates/aptos/src/node/mod.rs` lines 1540 to 1575.
- Spec: section 6 "Trusted setup handling", bullet 3.
- Prototype: none. The prototype never reads a key file. `proto:src/lib.rs` builds the key in memory through `setup_for_testing`.

### Tests that exercise it

None in the prototype. Upstream round-trip tests exist (`test_serialize_deserialize`, `test_serialize_deserialize_range`, `test_serialize_deserialize_subrange`, `test_serialize_deserialize_range_oob`, `test_serialize_deserialize_range_oob_2` in `digest_key_file.rs`). None feed a bad point. The agent did not write a new test. The packet recommends one in section 6.

### Adversarial analysis

1. Off-curve or low-order points in the file (adversary A or B). With `Compress::No` and `Validate::No`, a point need not be on the curve. A digest computed by MSM over bad bases produces a digest and eval proofs that do not verify, or that leak structure. Most likely effect is denial of service (reveals fail). A cryptographer should decide whether any confidentiality loss is possible, for example through an invalid-curve style effect on `tau_powers_g1` used in `DigestKey::digest` (`shared/digest.rs` line 192, MSM).
2. Inconsistent `tau_g2` versus `tau_powers_g1`. No pairing check ties them. A pairing check per round such as `e(P_1, g2) = e(P_0, tau_g2)` would detect it. Without it, `verify_pf` can accept proofs against one `tau` while digests use another, or reject everything.
3. Inconsistent `prepared_toeplitz_input`. It is derived data. A mismatch gives wrong eval proofs for honest batches. This breaks decryption of every bid in a batch, which is a reveal-level DoS.
4. Trapdoor knowledge. Validation cannot detect a contributor who kept `tau` or `r`. That is a ceremony property. A trapdoor holder can craft a digest colliding with a chosen identity set. See section 3.
5. Shape checks do not prove the powers are powers. Ratio checks across `P_j` are the standard check (consistency of `tau` powers). None exist.
6. Network-facing `ark_de` (adversary C). Ciphertext group elements (`ct_g2`), shares (G1), and verification keys use compressed `Validate::No`. `Ciphertext::verify` (`shared/ciphertext/mod.rs` line 103) checks associated data, identity binding, and an Ed25519 signature over the BCS bytes. It does not check that `ct_g2` is in G2. A bidder signs with their own one-time key, so a bidder can submit a ciphertext with a bad `ct_g2` that passes `verify_ct`. The result is a bid that fails at `prepare_ct` or `decrypt`. The reviewer must decide whether that can break the batch (for example if one bad element aborts `decrypt_all`) or only that bid. The prototype's `decrypt_all` uses `collect::<Result<Vec<_>>>()`, so one failure aborts the whole batch. That is a prototype property, not a library one, and the real handler must be designed accordingly.
7. Share verification with unchecked points. `verify_decryption_key_share` (`key_derivation.rs` line 139) runs a pairing equation on the share. The share's G1 point is decoded with `Validate::No`. Whether a non-subgroup G1 point can satisfy the equation, or can bias the reconstructed key, is a question for the reviewer. The final `verify_decryption_key` check (`encryption_key.rs` line 28) is the backstop.

### Questions for the cryptographer

- B1. Is full validation at startup, as the spec states, adequate? Specify exactly what it must check: on-curve, prime-order subgroup for every G1 and G2 point, pairing ratio checks for powers of `tau` across each round, consistency of `tau_g2`, consistency of `prepared_toeplitz_input`, shape.
- B2. Must the pairing ratio checks run for every round (256 rounds by 256 points), or can they be batched randomly? What is the acceptable failure probability?
- B3. Should the node compare the file hash to an on-chain `digest_key_hash` and refuse to start on mismatch (spec section 6 says yes)? Is the hash function and preimage (file header and points) correct?
- B4. Is `Validate::No` in `ark_de` acceptable for network-facing types given that decoding is compressed? Which types must be subgroup-checked on receipt (`Ciphertext.ct_g2`, shares, `Digest`, `EvalProof`, verification keys)?
- B5. Should `verify_ct` also check group membership of ciphertext elements, so a malformed bid is rejected at submission rather than at reveal?
- B6. Is a corrupted file more likely to cause denial of service only, or can it cause disclosure? Give the reasoning.
- B7. The spec cites upstream version 1.45.4. Does the reviewer want the behavior confirmed on that version, or is 1.49.6 the only relevant one?

## 3. Question C: FPTX round allocation

### Claim

Timelock reveals use their own digest key and their own round counter, `TimelockRoundState { epoch, next_round }`. Rounds are assigned on chain by the block prologue hook, in order of `(deadline_us, timelock_id)`, one round per shard reveal. A round is bound to one digest. A retry reuses the same round and recomputes the same digest. Rounds are not returned. `num_rounds = 256` per epoch. At `next_round >= num_rounds` the timelock expires and bids are refundable (spec section 4).

### Threat model

- Adversary: bidder or validator who tries to get two different digests signed under one round, to exhaust rounds, to reuse a round across epochs, or to make a round's randomizer `r` leak.
- Assets: confidentiality (no second digest for a round under the same secret) and liveness (every auction can reveal).
- Trust: honest weight at or above the threshold. Hook runs identically on every validator.

### Exact references

- `DigestKey::new` (`shared/digest.rs` line 57): one `tau`, one random `r` per round (`rs`, line 77), powers `r * tau^j` (line 79 to 87). Each round has its own randomized powers.
- `DigestKey::digest` (`shared/digest.rs` line 170): fails when `round >= tau_powers_g1.len()` (line 176). The digest is an MSM over `tau_powers_g1[round]`. The `Digest` stores `round` (lines 36 to 40).
- Eval proofs: `EvalProofsPromise::compute_all` uses `digest.round` (near line 245). Proof check `verify_pf` (line 202) uses only `tau_g2` and the digest point. The per-round `r` cancels in the pairing.
- Share verification does not read `Digest.round`. It uses only `digest_g1` (`key_derivation.rs` line 118 to 132). The round binds the digest only through the randomizer `r`.
- Mempool counter, for contrast: `next_decryption_round` in `decryption.move`, `TrustedSetupExhausted` in `consensus/src/pipeline/decryption_pipeline_builder.rs`. These files were not re-read for this packet. They are cited from the spec.
- Spec: section 4 rules 1 to 7, "Budget"; section 5 (frozen set at deadline); section 8 rule 6.
- Prototype: `proto:src/lib.rs` `Reveal::new` (lines 171 to 187) rejects `round >= Scheme::num_rounds(&setup.dk)`. `NUM_ROUNDS` is declared (line 28) and not used by `Reveal::new`.

### Tests that exercise it

- `round_at_num_rounds_is_refused_and_rounds_bind_digests`: round equal to `num_rounds` is refused. The same batch under round 0 and round 1 gives shares that do not cross-verify. A retry of round 0 recomputes the same digest and shares carry over.
- `corrupted_or_misattributed_share_fails_verification_before_combination` case 3: a share for another round and batch fails.
- `full_256_bid_batch_reveals_and_257th_is_refused`: round budget of 2, one 256-bid reveal at round 0.
- `unverified_bad_share_poisons_combination_and_final_check_catches_it`: a share for a different digest is caught by the final key check.

Limits of the evidence:

- No test shows two different digests under one round being rejected. Spec section 9 lists this negative test. The prototype has no registry state, so it cannot enforce one-digest-per-round. This is an implementation task.
- Both reveals in the test use the same ciphertext set. The statement "retry recomputes the same digest" holds because the MSM is deterministic over a sorted `IdSet`. The ordering of `IdSet::from_slice` was not audited.
- No test covers epoch change, round exhaustion on chain, or the on-chain counter.
- The prototype setup uses a seeded RNG. The statistical properties of `r` are not tested.

### Adversarial analysis

1. Two digests, one round, same secret. Two different batches `S1` and `S2` committed under round `i` give digests `D1 = r_i * P_{S1}(tau) * g1` and `D2 = r_i * P_{S2}(tau) * g1`. Their ratio reveals `P_{S1}(tau)/P_{S2}(tau)` in the exponent, and a decryption key for each reveals the signature `s * (D + H(mpk))`. The role of `r_i` is to blind `tau` structure per round. Whether reusing `r_i` for two sets weakens that blinding, or lets an observer combine two signed digests of the same round to open an identity not in either set, is the central question for the reviewer. The spec forbids it by rule. The rule is enforced by the registry in code that does not exist yet.
2. Linear relation across rounds. Digests from different rounds under one `tau` are `r_i` multiples of related polynomial evaluations. If an adversary controls the set committed in round `i` and sees the key released for round `j`, can they craft a set whose digest under round `i` equals a known multiple of a digest signed in round `j`? That needs knowledge of `r_i/r_j`, which is secret, unless `tau` and `r` are known.
3. Digest linkage via `Digest.round`. Shares and keys ignore the `round` field. Two different rounds could in principle produce the same `digest_g1` only with negligible probability. The reviewer should confirm no code path trusts `round` for authorization.
4. Round exhaustion. A reveal consumes a round even if it fails. Rounds are not returned. An adversary who can create many timelocks (registry spam, shard spam) can burn the 256-round budget in one epoch and force refunds for honest auctions. The spec relies on registration cost and the 25 percent alert. Is the registry spam-resistant? That is a product and economic question that also affects the cryptographic liveness claim.
5. Ordering by `(deadline_us, timelock_id)`. A timelock with an earlier deadline gets an earlier round. An adversary who can choose deadline and id can influence which round a victim shard gets. Round index itself should not matter cryptographically, since each round is an independent randomization. The reviewer should confirm.
6. Assignment at reveal start (rule 4). A ciphertext does not depend on the round. A bid can therefore be included in whichever round it is assigned to, and is fixed only when the batch is frozen. The frozen-set rule (spec section 5, bids only accepted strictly before `deadline_us`) must be enforced on chain with the same timestamp that the hook uses. A race between the last accepted bid and assignment is a design risk.
7. Epoch boundary. The same `(digest key, round)` is reused in the next epoch under a new secret (rule 6 of section 8). If the digest key file is not replaced per epoch, round `i` has the same `r_i` in both epochs. A digest `D` signed in epoch `e` under secret `s` is not a valid input under a different secret `s'`. The reviewer should confirm that reuse of `r_i` across secrets is safe, and whether a fresh digest key per epoch is needed.
8. `tau` shared by all rounds. All rounds share one `tau` and differ by `r_i` only. A single `tau` leak breaks every round. This is a property of `DigestKey::new`, not of Atomica's allocation.
9. Budget arithmetic. The spec's claim of at most 40 rounds per epoch (ten markets, four shards, two auctions a day, 2 hour epochs) was not re-derived. It should be rechecked against the real epoch length and auction count.

### Questions for the cryptographer

- C1. Is it sound to allocate one round per reveal and never reuse a round within an epoch? Is the scheme secure when the same round `i` is committed for two different identity sets under the same secret? State the exact assumption used.
- C2. Is `r_i` reuse across epochs (new secret, same digest key) safe, or must the digest key be regenerated every epoch?
- C3. Is a retry that reuses the same round safe, provided the digest is identical? What if the set is identical but the IdSet ordering differs between validators? Is the digest canonical?
- C4. Is `num_rounds = 256` with `max_batch_size = 256` an acceptable parameter set for the trusted setup size, and does the bound on `max_batch_size` relate to any security degree bound (polynomial degree versus `tau` powers)?
- C5. Does any property depend on the round being dense and starting at 0 (the mempool style) versus any assignment?
- C6. What is the correct, minimal on-chain invariant to enforce "one digest per round per epoch"? Is storing the digest hash enough?
- C7. Do you want the round-exhaustion denial-of-service handled in this design or accepted as an economic risk?

## 4. Cross-cutting findings the agent flags

1. The DigestKey validation mitigation in the spec does not exist, and the existing `ValidateDigestKey` command gives a false "valid" message (section 2, item 5).
2. `ark_de` skips validation for every network-facing type, and its doc comment says the opposite (section 2, item 3).
3. `verify_ct` does not check ciphertext group elements. A bidder can sign a malformed ciphertext that passes `verify_ct` (section 2, adversarial item 6).
4. The prototype test for key separation is weak (section 1, "Limits of that test").
5. No test covers "two digests under one round" (section 3, "Limits of the evidence").
6. `WeightedBIBEMasterSecretKeyShare::virtualized_sk_shares` carries an upstream TODO about a panic on an out-of-bounds deserialized player id (`fptx_weighted.rs` lines 74 to 79). Upstream says instances are only built locally. Confirm this holds for the timelock path.
7. The residual items listed at the end of the spec (two-session chunky DKG, AD layout, ceremony) are still open and are not covered by any test.

## 5. Associated data layout (for completeness)

The spec lists the AD layout as a residual review item. It is not one of the three questions, so this packet does not analyze it. For reference: `proto:src/lib.rs` `BidAd` is `{ tag, chain_id, epoch, auction_id, shard, bidder, deadline_us }`, encoded with BCS. The test `ciphertext_rebound_to_other_bidder_auction_epoch_or_shard_is_rejected` changes each field in turn and expects `verify_ct` to fail. The identity is `H(vk, AD)` (`shared/ids.rs` line 41). `verify_ct` compares the stored AD bytes to the supplied AD, checks the identity, and checks the Ed25519 signature (`shared/ciphertext/mod.rs` line 103).

## 6. Evidence the reviewer may want added

These are suggestions. None has been done.

- A test that reads a digest key file with one corrupted point and shows the node must refuse it.
- A startup validator that checks on-curve, subgroup, ratio, and `tau_g2` consistency, then a test that fails on each corruption.
- A test that two distinct digests for one round are rejected by the registry.
- A test that a share released for one digest key and secret is invalid under another, with the same `tau`.
- A test that a signed ciphertext with a malformed `ct_g2` is rejected at submission or isolated at reveal.
- Round exhaustion and epoch change tests on the Move side.

## 6a. Outcome: required changes (engineering review, no external cryptographer)

No external cryptographer has reviewed or signed anything. This section records the outcome of the engineering review of this packet, dated 2026-10-30, made for forgectl feature `crypto-review-signoff`. It is required changes, not a sign-off. The Reviewer record in section 7 is blank and stays open.

A cryptographer review is still recommended before any release or mainnet use. Closing this feature does not satisfy that recommendation.

### Question A: separate secret versus shared mempool secret

Outcome: proceeds on the separate-secret default. The cost report (`docs/code-reviews/aptos-upgrade-cost-report.md`, section g, "Key separation") says a cryptographer's sign-off is needed only to reuse the mempool's secret. The Option C spec chose the separate secret, the safe default. No external sign-off is needed to proceed. Reusing the mempool secret stays forbidden until a cryptographer signs off. The prototype's key-separation test is weak (section 4, item 4), so a stronger test is carried below.

### Question B: DigestKey deserialization skipping validation

Outcome: required changes. The packet found that the spec's startup validation is not implemented (section 2).

- B-1. Replace `Validate::No` in the DigestKey file loader (`read_round`) with validated deserialization, or enforce a full startup validation (on-curve, subgroup, tau power ratio checks, `tau_g2` consistency, `prepared_toeplitz_input` consistency). Add a negative test with one corrupted point.
- B-2. Replace `Validate::No` in `ark_de` with validated deserialization, or subgroup-check every network-facing type on receipt. Fix its doc comment.
- B-3. Make `ValidateDigestKey` honest. It must run the real validation, not only parse and check size.
- B-4. Enforce the DigestKey blake3 hash against an expected value, and refuse to start on mismatch. The hash is only a metric today.
- B-5. Make `verify_ct` reject ciphertexts with malformed group elements at submission, so one bad bid cannot abort a batch reveal.

### Question C: FPTX round allocation

Outcome: required changes.

- C-1. Build and test the rule that one round never yields two keys under one secret: one digest per round per epoch, enforced on chain, with a test that two distinct digests for one round are rejected.
- C-2. Test that a share released for one digest key and secret is invalid under another, with the same `tau`.
- C-3. Add Move-side tests for round exhaustion and epoch change.
- Open for the cryptographer: whether `r_i` reuse across epochs is safe (C2), and the exact assumption for two digests under one round (C1).

### Where each required change is carried (forgectl features)

- `natives-port-timelock`: B-2 and B-5.
- `node-share-verification`: subgroup check of share points before store (part of B-2), and C-2.
- `node-validator-txn-timelock`: C-1 and C-3.
- `node-dkg-chunky-wiring`: B-1, B-3, B-4.

## 7. Reviewer record

TO BE FILLED BY THE HUMAN CRYPTOGRAPHER. The agent that prepared this packet did not fill it, must not fill it, and cannot attest to any entry in it. Leave every field blank until a human completes the review.

| Field | Entry |
|---|---|
| Question A: separate secret vs shared mempool secret | Sign-off / Required changes (circle one): |
| Question A: notes or required changes | |
| Question B: DigestKey deserialization skipping validation | Sign-off / Required changes (circle one): |
| Question B: notes or required changes | |
| Question C: FPTX round allocation | Sign-off / Required changes (circle one): |
| Question C: notes or required changes | |
| Reviewer name | |
| Reviewer affiliation | |
| Date (YYYY-MM-DD) | |
| Commit sha reviewed (full 40 characters) | |
| Packet commit sha at time of review | |
| Signature or attestation (human only) | |

Merge rule: the forgectl feature `crypto-review-signoff` is not complete and is not merged until a human has filled this section.

## 8. Revision for the new design (IBE on chunky DKG)

Date: 2026-10-30. Prepared by an AI agent. This section is not a review and not a sign-off.

### 8a. What changed

The new design is in [aptos-upgrade-ibe-chunky-spec.md](aptos-upgrade-ibe-chunky-spec.md). The timelock keeps atomica's Boneh-Franklin IBE and hardens it. The secret comes from the timelock's own chunky DKG session. The encrypted mempool's ciphers and secret are not used.

The FPTX-specific questions no longer apply:

- Question B, the `DigestKey` file questions (B-1, B-3, B-4 as written, and B1 to B3, B6, B7): the new design has no `DigestKey`. The nearest remaining item is the chunky `PublicParameters` blob, covered by new questions in section 9.
- Question C, FPTX round allocation (all of C1 to C7): the new design has one key per identity and no rounds. The registry rules that remain are in the spec, section 7.

Questions A and the network-facing deserialization part of B still apply in changed form.

### 8b. Mapping of earlier required changes

| Earlier change | Status under the new design | Replacement or reason |
|---|---|---|
| Question A, separate secret from the mempool | Carried | Spec section 4. A separate chunky DKG session with its own session id tag, own state, own parameters blob. Reusing the mempool secret stays forbidden |
| B-1, validate the `DigestKey` file loader | Moot, with a carried analogue | No `DigestKey`. The analogue is startup validation of the timelock `PublicParameters` blob (validated deserialization, hash compare, `G_2` equals the generator). Carried by `node-dkg-chunky-wiring` |
| B-2, validated deserialization for network-facing `ark_de` types | Carried, narrower | Shares and bid ciphertext points are decoded with `Validate::Yes` (on curve, prime-order subgroup, not identity). Share points: carried by `node-share-verification`. Bid `U`: carried by `natives-port-timelock`. No FPTX types remain |
| B-3, honest `ValidateDigestKey` command | Moot | The command validates a `DigestKey`, which is not used. If a node command for the timelock parameters blob is added, it must run the real validation. Carried by `node-dkg-chunky-wiring` |
| B-4, compare the `DigestKey` hash to an expected value | Carried as an analogue | `TimelockEpochConfig.public_parameters_hash`. The node refuses to start the timelock service on mismatch. Carried by `node-dkg-chunky-wiring` |
| B-5, `verify_ct` rejects malformed group elements | Replaced | `ibe::verify_bid_ct` checks `U` group membership, associated data and the one-time signature at submission. Spec section 6. Carried by `natives-port-timelock` |
| C-1, one digest per round per epoch | Moot | No digests and no rounds. One key per identity, and the registry status moves one way (spec section 7, rules 1 and 2). The nearest test is the one-way status test |
| C-2, a share for one digest key and secret is invalid under another | Replaced | A share for another identity, epoch or secret fails the per-share pairing check. A transcript for another session id fails verification. Carried by `node-share-verification` and `node-dkg-chunky-wiring` |
| C-3, Move tests for round exhaustion and epoch change | Replaced | Round exhaustion is moot. Epoch change stays, with expiry `epoch-ended`, dropped shares, no straddle, and refund. Carried by `node-validator-txn-timelock` |
| Cost report gap 1, one bad share breaks a reveal | New check | Per-share pairing check before store, and a verified rebuilt key before a reveal counts. Spec section 5 |
| Cost report gap 2, bids forged from other bids | New wrapper | AES-128-GCM and one-time Ed25519 wrapper. Spec section 6 |
| Cost report gap 3, no range proofs | Carried by construction | Chunky PVSS with DeKART range proofs replaces `scalar_elgamal` |
| Cost report gap 4, one secret serves two protocols | Carried by construction | The dual-output DKG is retired. Randomness stays on DAS PVSS |
| Section 4, cross-cutting findings 1 and 5 (`ValidateDigestKey`, two digests per round) | Moot | The objects they describe are gone |
| Section 4, cross-cutting finding 2 (`ark_de` skips validation) | Carried | Same as B-2 |
| Section 4, cross-cutting finding 3 (`verify_ct` and group elements) | Replaced | Same as B-5 |
| Section 4, cross-cutting finding 6 (`virtualized_sk_shares` TODO panic) | Moot | That FPTX type is not used. The new share types must return an error, not panic, on an out-of-bounds player id |
| Section 4, cross-cutting finding 7 (two chunky sessions, AD layout, ceremony) | Carried | Two sessions, AD layout and parameters ceremony are items in section 9 |

### 8c. Evidence status

- The prototype in `atomica/proto-fptx-timelock/` (11 passing tests, local run, not CI) covers the superseded FPTX design only. It is a reference. It gives no evidence for the new design.
- No test or prototype yet exists for the new design. Every claim in the new spec is a claim to confirm or refute.
- No external cryptographer has reviewed or signed anything. Review is still recommended before any release or mainnet use.

## 9. Open questions for the new design

For the human cryptographer. Each refers to the [new spec](aptos-upgrade-ibe-chunky-spec.md).

- A1. Is a second chunky DKG session per epoch, separated only by a session id tag, sound at 1.49.6? The session id is the generic `session_id: &A` of `deal` and `verify` (spec section 4). Is any further transcript domain separation needed, given that the DST is fixed in `weighted_transcript_v2.rs` line 56?
- A2. Is it acceptable that both sessions encrypt shares to the same validator consensus key (`epoch_manager.rs` line 1204)? Is there a cross-session attack on chunked ElGamal with a reused key?
- A3. May the timelock reuse the mempool's chunky `PublicParameters` blob (DeKART setup), or must it have its own, as the spec decides? What breaks if one setup's trapdoor leaks: only range-proof soundness, or also confidentiality?
- B1. Is the CCA argument in spec section 6 sound? The construction is Boneh-Franklin KEM, HKDF over the pairing value plus `U`, identity, `vk` and the associated data hash, AES-128-GCM, and a one-time Ed25519 signature. Is a Fujisaki-Okamoto re-encryption check also required at decrypt time? State the assumption used.
- B2. Is it safe that one key per timelock opens every ciphertext under its identity, and that the only freeze is the contract rule that no bid is accepted at or after the deadline?
- B3. Is `H1` (RFC 9380 BLS12381G1 SSWU with the Atomica DST) acceptable, and is the identity layout (chain, epoch, timelock id, deadline) enough?
- C1. Does the per-share pairing check, plus validated decoding, fully close the bad-share problem? Is a rebuilt-key check still needed for an accepted DKG output?
- C2. Does upstream chunky transcript verification already imply that the dealt public key `V0` equals the interpolation of the public key shares `Vs`? The spec adds an explicit check. Is it redundant?
- C3. Is the threshold taken from the DKG (`reconstruct_threshold_in_weights`) the right threshold for confidentiality and liveness of timelock reveals, or does the timelock need its own secrecy and reconstruction thresholds?
- D1. Is the no-straddle rule enough: reveal only while `current_epoch == info.epoch`, expiry at epoch change, shares of two epochs never combined?
- D2. Are the reveal window (10 minutes after the deadline) and the 15 minute registration guard acceptable?
- D3. Round exhaustion is gone. Is the registration flood risk (no fee, no cap) an acceptable economic risk, or does it affect the confidentiality claim?
