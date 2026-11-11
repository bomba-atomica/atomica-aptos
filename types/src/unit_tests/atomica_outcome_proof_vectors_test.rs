// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Golden vectors for the Ethereum verifier of Aptos outcome proofs.
//!
//! The vectors come from seeded validator keys and synthetic, but fully consistent, ledger data:
//! a genesis LedgerInfo and its waypoint, an epoch change to a new validator set, LedgerInfos
//! signed by different signer sets, a transaction accumulator with 1500 leaves, and an
//! `AuctionOutcome` event in a deep transaction. Every value is the BCS of the production
//! aptos-types struct. The JSON also carries the intermediate values the Solidity verifier
//! recomputes: CryptoHasher seeds, signing messages, the RFC 9380 hash to G2 and EIP-2537
//! uncompressed points.
//!
//! Output, checked in and byte-for-byte reproducible:
//! `atomica/golden_vectors/aptos_outcome_proof_vectors.json`.
//!
//! Regenerate with
//!
//! ```text
//! cargo test -p aptos-types --lib atomica_outcome_proof_vectors_test::write_atomica_outcome_proof_vectors -- --ignored
//! ```

use crate::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    contract_event::ContractEvent,
    epoch_change::{EpochChangeProof, Verifier},
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::{
        accumulator::InMemoryAccumulator, definition::EventAccumulatorProof, AccumulatorProof,
        MerkleTreeInternalNode, TransactionAccumulatorProof, TransactionInfoWithProof,
    },
    transaction::{ExecutionStatus, TransactionInfo},
    validator_signer::ValidatorSigner,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    waypoint::Waypoint,
};
use aptos_crypto::{
    bls12381,
    hash::{
        CryptoHash, CryptoHasher, EventAccumulatorHasher, HashValue, TransactionAccumulatorHasher,
        ACCUMULATOR_PLACEHOLDER_HASH,
    },
    traits::signing_message,
    Uniform,
};
use ark_bls12_381::{
    g2::Config as G2Config, Bls12_381, Fq, G1Affine, G1Projective, G2Affine, G2Projective,
};
use ark_ec::{
    hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
    pairing::Pairing,
    AffineRepr, CurveGroup,
};
use ark_ff::{field_hashers::DefaultFieldHasher, BigInteger, PrimeField};
use ark_serialize::{CanonicalDeserialize, Compress, Validate};
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2_0_10_6::Sha256;
use std::{str::FromStr, sync::Arc};

const JSON_PATH: &str = "atomica/golden_vectors/aptos_outcome_proof_vectors.json";
const CHECKED_IN_JSON: &str =
    include_str!("../../../atomica/golden_vectors/aptos_outcome_proof_vectors.json");
const GENERATOR_COMMAND: &str = "cargo test -p aptos-types --lib atomica_outcome_proof_vectors_test::write_atomica_outcome_proof_vectors -- --ignored";

/// The DST of Aptos BLS12-381 signatures (minimal-pubkey-size, proof of possession scheme).
const BLS_DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// Voting power of the epoch 1 set (total 10, quorum 7) and the epoch 2 set (total 11, quorum 8).
const EPOCH_1_POWERS: [u64; 4] = [1, 2, 3, 4];
const EPOCH_2_POWERS: [u64; 4] = [5, 3, 2, 1];
/// Signer sets by validator index.
const EPOCH_1_QUORUM: [usize; 2] = [2, 3]; // 3 + 4 = 7, exactly the quorum
const EPOCH_2_QUORUM: [usize; 2] = [0, 1]; // 5 + 3 = 8, exactly the quorum
const EPOCH_2_BELOW_QUORUM: [usize; 2] = [0, 2]; // 5 + 2 = 7, one short

const LEAF_COUNT: u64 = 1500;
const EPOCH_1_END_VERSION: u64 = 499;
const OUTCOME_VERSION: u64 = 1337;
const OUTCOME_EVENT_INDEX: usize = 2;

/// The named address `atomica` takes its dev address in the vectors.
const OUTCOME_TYPE_TAG: &str = "0xcafe::auction::AuctionOutcome";

/// BCS layout of the `AuctionOutcome` event payload. The product repo has not fixed it yet (its
/// proof-gated LockBox issue owns the Move struct). This is the default the vectors pin: the
/// fields LockBox needs to recompute the auction key `keccak256(window_id, pair_hash, deadline)`
/// and to store the dispositions root with its leaf count.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct AuctionOutcome {
    window_id: u64,
    pair_hash: Vec<u8>,
    eth_lock_deadline: u64,
    dispositions_root: Vec<u8>,
    leaf_count: u64,
}

fn auction_outcome() -> AuctionOutcome {
    AuctionOutcome {
        window_id: 42,
        pair_hash: HashValue::sha3_256_of(b"atomica golden pair").to_vec(),
        eth_lock_deadline: 1_767_225_600,
        dispositions_root: HashValue::sha3_256_of(b"atomica golden dispositions").to_vec(),
        leaf_count: 3,
    }
}

// ---------------------------------------------------------------------------------------------
// Encoding helpers
// ---------------------------------------------------------------------------------------------

fn hx(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

fn unhex(value: &Value) -> Vec<u8> {
    hex::decode(value.as_str().unwrap()).unwrap()
}

fn tagged(tag: &str, n: u64) -> HashValue {
    HashValue::sha3_256_of(&[tag.as_bytes(), &n.to_le_bytes()].concat())
}

/// EIP-2537 field element: 64 bytes, big endian, 16 zero bytes of padding.
fn fp_eip2537(x: &Fq) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend(x.into_bigint().to_bytes_be());
    out
}

/// EIP-2537 G1 point: x || y, 128 bytes.
fn g1_eip2537(p: &G1Affine) -> Vec<u8> {
    let (x, y) = p.xy().unwrap();
    [fp_eip2537(&x), fp_eip2537(&y)].concat()
}

/// EIP-2537 G2 point: x.c0 || x.c1 || y.c0 || y.c1, 256 bytes.
fn g2_eip2537(p: &G2Affine) -> Vec<u8> {
    let (x, y) = p.xy().unwrap();
    [
        fp_eip2537(&x.c0),
        fp_eip2537(&x.c1),
        fp_eip2537(&y.c0),
        fp_eip2537(&y.c1),
    ]
    .concat()
}

fn decode_g1(compressed: &[u8]) -> G1Affine {
    G1Affine::deserialize_with_mode(compressed, Compress::Yes, Validate::Yes).unwrap()
}

fn decode_g2(compressed: &[u8]) -> Option<G2Affine> {
    G2Affine::deserialize_with_mode(compressed, Compress::Yes, Validate::No).ok()
}

/// RFC 9380 hash to G2, `expand_message_xmd` with SHA-256, under the Aptos signature DST.
fn hash_to_g2(message: &[u8]) -> G2Affine {
    type G2Hasher =
        MapToCurveBasedHasher<G2Projective, DefaultFieldHasher<Sha256, 128>, WBMap<G2Config>>;
    G2Hasher::new(BLS_DST).unwrap().hash(message).unwrap()
}

// ---------------------------------------------------------------------------------------------
// Accumulators
// ---------------------------------------------------------------------------------------------

/// Root and proof for leaf `index`, built level by level. At each level a missing right sibling
/// is the placeholder hash, as in `InMemoryAccumulator`.
fn accumulator_proof<H: CryptoHasher>(
    leaves: &[HashValue],
    index: usize,
) -> (HashValue, AccumulatorProof<H>) {
    let mut siblings = vec![];
    let mut level = leaves.to_vec();
    let mut position = index;
    while level.len() > 1 {
        siblings.push(
            level
                .get(position ^ 1)
                .copied()
                .unwrap_or(*ACCUMULATOR_PLACEHOLDER_HASH),
        );
        level = level
            .chunks(2)
            .map(|pair| {
                let right = pair
                    .get(1)
                    .copied()
                    .unwrap_or(*ACCUMULATOR_PLACEHOLDER_HASH);
                MerkleTreeInternalNode::<H>::new(pair[0], right).hash()
            })
            .collect();
        position /= 2;
    }
    let root = level[0];
    // The production accumulator gives the same root.
    assert_eq!(
        root,
        InMemoryAccumulator::<H>::from_leaves(leaves).root_hash()
    );
    (root, AccumulatorProof::new(siblings))
}

fn outcome_events() -> Vec<ContractEvent> {
    let outcome = bcs::to_bytes(&auction_outcome()).unwrap();
    vec![
        ContractEvent::new_v2_with_type_tag_str(
            "0x1::fungible_asset::Withdraw",
            bcs::to_bytes(&(AccountAddress::from_hex_literal("0xa11ce").unwrap(), 100u64)).unwrap(),
        ),
        ContractEvent::new_v2_with_type_tag_str(
            "0x1::fungible_asset::Deposit",
            bcs::to_bytes(&(AccountAddress::from_hex_literal("0xb0b").unwrap(), 100u64)).unwrap(),
        ),
        ContractEvent::new_v2_with_type_tag_str(OUTCOME_TYPE_TAG, outcome),
        ContractEvent::new_v2_with_type_tag_str(
            "0x1::fungible_asset::Deposit",
            bcs::to_bytes(&(AccountAddress::from_hex_literal("0xca7").unwrap(), 7u64)).unwrap(),
        ),
        ContractEvent::new_v2_with_type_tag_str(
            "0x1::transaction_fee::FeeStatement",
            bcs::to_bytes(&(1234u64, 1000u64, 200u64, 34u64, 0u64)).unwrap(),
        ),
    ]
}

fn transaction_info(version: u64, event_root_hash: HashValue) -> TransactionInfo {
    TransactionInfo::builder_v0()
        .transaction_hash(tagged("atomica golden txn", version))
        .state_change_hash(tagged("atomica golden write set", version))
        .event_root_hash(event_root_hash)
        .gas_used(version % 97)
        .status(ExecutionStatus::Success)
        .auxiliary_info_hash(tagged("atomica golden auxiliary info", version))
        .build()
}

// ---------------------------------------------------------------------------------------------
// Validators and signing
// ---------------------------------------------------------------------------------------------

struct ValidatorSetVectors {
    signers: Vec<ValidatorSigner>,
    epoch_state: EpochState,
}

fn validator_set(epoch: u64, powers: &[u64], seed: u8) -> ValidatorSetVectors {
    let signers: Vec<ValidatorSigner> = (0..powers.len())
        .map(|i| {
            let mut rng = StdRng::from_seed([seed + i as u8; 32]);
            let mut address = [0u8; AccountAddress::LENGTH];
            address[0] = seed;
            address[AccountAddress::LENGTH - 1] = i as u8 + 1;
            ValidatorSigner::new(
                AccountAddress::new(address),
                Arc::new(bls12381::PrivateKey::generate(&mut rng)),
            )
        })
        .collect();
    let infos = signers
        .iter()
        .zip(powers)
        .map(|(s, power)| ValidatorConsensusInfo::new(s.author(), s.public_key(), *power))
        .collect();
    ValidatorSetVectors {
        signers,
        epoch_state: EpochState::new(epoch, ValidatorVerifier::new(infos)),
    }
}

fn sign(
    ledger_info: &LedgerInfo,
    verifier: &ValidatorVerifier,
    signers: &[&ValidatorSigner],
) -> LedgerInfoWithSignatures {
    let signatures: Vec<(AccountAddress, bls12381::Signature)> = signers
        .iter()
        .map(|s| (s.author(), s.sign(ledger_info).unwrap()))
        .collect();
    let aggregate = verifier
        .aggregate_signatures(signatures.iter().map(|(a, s)| (a, s)))
        .unwrap();
    LedgerInfoWithSignatures::new(ledger_info.clone(), aggregate)
}

fn pick<'a>(set: &'a ValidatorSetVectors, indexes: &[usize]) -> Vec<&'a ValidatorSigner> {
    indexes.iter().map(|i| &set.signers[*i]).collect()
}

/// The validator set JSON: every key in both compressed and EIP-2537 form.
fn validator_set_json(set: &ValidatorSetVectors) -> Value {
    let verifier = &set.epoch_state.verifier;
    json!({
        "epoch": set.epoch_state.epoch,
        "total_voting_power": verifier.total_voting_power().to_string(),
        "quorum_voting_power": verifier.quorum_voting_power().to_string(),
        "epoch_state_bcs": hx(&bcs::to_bytes(&set.epoch_state).unwrap()),
        "validators": verifier.validator_infos.iter().enumerate().map(|(i, v)| json!({
            "index": i,
            "address": v.address.to_hex_literal(),
            "public_key": hx(&v.public_key.to_bytes()),
            "public_key_eip2537": hx(&g1_eip2537(&decode_g1(&v.public_key.to_bytes()))),
            "voting_power": v.voting_power,
        })).collect::<Vec<_>>(),
    })
}

/// The values the Solidity verifier recomputes for one signed LedgerInfo.
fn signing_json(
    signed_bcs: &[u8],
    public_keys: &[bls12381::PublicKey],
    signers: &[usize],
) -> Value {
    let signed: LedgerInfoWithSignatures = bcs::from_bytes(signed_bcs).unwrap();
    let message = signing_message(signed.ledger_info()).unwrap();
    let signature = &signed_bcs[signed_bcs.len() - bls12381::Signature::LENGTH..];
    let aggregate_public_key: G1Affine = signers
        .iter()
        .map(|i| decode_g1(&public_keys[*i].to_bytes()).into_group())
        .sum::<G1Projective>()
        .into_affine();
    json!({
        "signer_indexes": signers,
        "signer_bitmask_bcs": hx(&bcs::to_bytes(signed.signatures().get_signers_bitvec()).unwrap()),
        "signing_message": hx(&message),
        "hash_to_g2_eip2537": hx(&g2_eip2537(&hash_to_g2(&message))),
        "signature": hx(signature),
        "signature_eip2537": decode_g2(signature).map(|p| hx(&g2_eip2537(&p))),
        "aggregate_public_key_eip2537": hx(&g1_eip2537(&aggregate_public_key)),
    })
}

/// Flips the lowest bit of the compressed signature that still decodes to a curve point, so the
/// tampered signature reaches the pairing check (it is not in the prime-order subgroup).
fn flip_signature_bit(signed_bcs: &[u8]) -> Vec<u8> {
    let start = signed_bcs.len() - bls12381::Signature::LENGTH;
    for byte in (start..signed_bcs.len()).rev() {
        for bit in 0..8 {
            let mut flipped = signed_bcs.to_vec();
            flipped[byte] ^= 1 << bit;
            if decode_g2(&flipped[start..]).is_some() {
                return flipped;
            }
        }
    }
    unreachable!("no single bit flip decodes to a curve point")
}

// ---------------------------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------------------------

fn generate_json() -> String {
    let epoch_1 = validator_set(1, &EPOCH_1_POWERS, 0x11);
    let epoch_2 = validator_set(2, &EPOCH_2_POWERS, 0x22);

    // The ledger: 1500 transactions, one of which emits the AuctionOutcome event.
    let events = outcome_events();
    let event_hashes: Vec<HashValue> = events.iter().map(CryptoHash::hash).collect();
    let (event_root, event_proof) =
        accumulator_proof::<EventAccumulatorHasher>(&event_hashes, OUTCOME_EVENT_INDEX);
    let txn_infos: Vec<TransactionInfo> = (0..LEAF_COUNT)
        .map(|v| {
            if v == OUTCOME_VERSION {
                transaction_info(v, event_root)
            } else {
                transaction_info(v, *ACCUMULATOR_PLACEHOLDER_HASH)
            }
        })
        .collect();
    let leaves: Vec<HashValue> = txn_infos.iter().map(CryptoHash::hash).collect();
    let root_at = |version: u64| {
        InMemoryAccumulator::<TransactionAccumulatorHasher>::from_leaves(
            &leaves[..=version as usize],
        )
        .root_hash()
    };
    let (root, txn_proof) =
        accumulator_proof::<TransactionAccumulatorHasher>(&leaves, OUTCOME_VERSION as usize);

    // Genesis: epoch 0 ends at version 0 and names the epoch 1 set.
    let genesis = LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                0,
                0,
                HashValue::zero(),
                root_at(0),
                0,
                0,
                Some(epoch_1.epoch_state.clone()),
            ),
            HashValue::zero(),
        ),
        AggregateSignature::empty(),
    );
    let waypoint = Waypoint::new_epoch_boundary(genesis.ledger_info()).unwrap();

    // Epoch 1 ends, signed by exactly its quorum, and names the epoch 2 set.
    let epoch_change = sign(
        &LedgerInfo::new(
            BlockInfo::new(
                1,
                120,
                tagged("atomica golden block", 1),
                root_at(EPOCH_1_END_VERSION),
                EPOCH_1_END_VERSION,
                1_700_000_100_000_000,
                Some(epoch_2.epoch_state.clone()),
            ),
            tagged("atomica golden consensus data", 1),
        ),
        &epoch_1.epoch_state.verifier,
        &pick(&epoch_1, &EPOCH_1_QUORUM),
    );
    let next = EpochChangeProof::new(vec![genesis.clone(), epoch_change.clone()], false)
        .verify(&waypoint)
        .unwrap()
        .ledger_info()
        .next_epoch_state()
        .cloned()
        .unwrap();
    assert_eq!(next, epoch_2.epoch_state);

    // The outcome LedgerInfo in epoch 2, under four signer sets.
    let outcome_li = LedgerInfo::new(
        BlockInfo::new(
            2,
            240,
            tagged("atomica golden block", 2),
            root,
            LEAF_COUNT - 1,
            1_700_000_200_000_000,
            None,
        ),
        tagged("atomica golden consensus data", 2),
    );
    let quorum = bcs::to_bytes(&sign(
        &outcome_li,
        &epoch_2.epoch_state.verifier,
        &pick(&epoch_2, &EPOCH_2_QUORUM),
    ))
    .unwrap();
    let below = bcs::to_bytes(&sign(
        &outcome_li,
        &epoch_2.epoch_state.verifier,
        &pick(&epoch_2, &EPOCH_2_BELOW_QUORUM),
    ))
    .unwrap();
    let all = [0, 1, 2, 3];
    let previous_set = bcs::to_bytes(&sign(
        &outcome_li,
        &epoch_1.epoch_state.verifier,
        &pick(&epoch_1, &all),
    ))
    .unwrap();
    let flipped = flip_signature_bit(&quorum);

    let keys = |set: &ValidatorSetVectors| -> Vec<bls12381::PublicKey> {
        set.signers.iter().map(|s| s.public_key()).collect()
    };
    let case =
        |name: &str, accept: bool, signed: &[u8], set: &ValidatorSetVectors, who: &[usize]| {
            json!({
                "name": name,
                "expected": if accept { "accept" } else { "reject" },
                "ledger_info_with_signatures": hx(signed),
                "signing": signing_json(signed, &keys(set), who),
            })
        };
    let target = &events[OUTCOME_EVENT_INDEX];
    let outcome = auction_outcome();

    let doc = json!({
        "version": "1.0.0",
        "description": "Aptos outcome proofs for the Ethereum light client: epoch change chain from the genesis waypoint, signed LedgerInfos, transaction and event accumulator proofs",
        "generator": GENERATOR_COMMAND,
        "encoding": {
            "bcs": "hex BCS of the aptos-types structs, no 0x prefix",
            "g1_compressed": "ZCash compressed, 48 bytes",
            "g2_compressed": "ZCash compressed, 96 bytes",
            "eip2537_g1": "x || y, each 64 bytes big endian with 16 zero bytes of padding",
            "eip2537_g2": "x.c0 || x.c1 || y.c0 || y.c1, each 64 bytes big endian with 16 zero bytes of padding",
            "hash": "SHA3-256(seed || BCS(value)), seed = SHA3-256(\"APTOS::\" || salt)",
            "signing_message": "LedgerInfo seed || BCS(LedgerInfo)",
            "hash_to_g2": "RFC 9380 BLS12381G2_XMD:SHA-256_SSWU_RO_",
            "accumulator": "leaf-to-root siblings; a missing right subtree is the placeholder hash",
            "quorum": "signer voting power >= quorum_voting_power = total * 2 / 3 + 1",
        },
        "bls_dst": String::from_utf8(BLS_DST.to_vec()).unwrap(),
        "hasher_seeds": [
            hasher_json::<LedgerInfo>("LedgerInfo"),
            hasher_json::<TransactionInfo>("TransactionInfo"),
            hasher_json::<ContractEvent>("ContractEvent"),
            seed_json("TransactionAccumulator", TransactionAccumulatorHasher::seed()),
            seed_json("EventAccumulator", EventAccumulatorHasher::seed()),
        ],
        "accumulator_placeholder_hash": hx(ACCUMULATOR_PLACEHOLDER_HASH.as_ref()),
        "validator_sets": [validator_set_json(&epoch_1), validator_set_json(&epoch_2)],
        "genesis": {
            "ledger_info_with_signatures": hx(&bcs::to_bytes(&genesis).unwrap()),
            "ledger_info": hx(&bcs::to_bytes(genesis.ledger_info()).unwrap()),
            "waypoint": waypoint.to_string(),
            "waypoint_version": waypoint.version(),
            "waypoint_value": hx(waypoint.value().as_ref()),
        },
        "epoch_change": case(
            "epoch_1_to_2_quorum",
            true,
            &bcs::to_bytes(&epoch_change).unwrap(),
            &epoch_1,
            &EPOCH_1_QUORUM,
        ),
        "outcome_ledger_infos": [
            case("quorum", true, &quorum, &epoch_2, &EPOCH_2_QUORUM),
            case("below_quorum", false, &below, &epoch_2, &EPOCH_2_BELOW_QUORUM),
            case("flipped_signature_bit", false, &flipped, &epoch_2, &EPOCH_2_QUORUM),
            case("previous_epoch_set", false, &previous_set, &epoch_1, &all),
        ],
        "transaction": {
            "version": OUTCOME_VERSION,
            "leaf_count": LEAF_COUNT,
            "accumulator_root": hx(root.as_ref()),
            "transaction_info": hx(&bcs::to_bytes(&txn_infos[OUTCOME_VERSION as usize]).unwrap()),
            "transaction_info_hash": hx(leaves[OUTCOME_VERSION as usize].as_ref()),
            "transaction_accumulator_proof": hx(&bcs::to_bytes(&txn_proof).unwrap()),
            "siblings": txn_proof.siblings().iter().map(|h| hx(h.as_ref())).collect::<Vec<_>>(),
        },
        "event": {
            "index": OUTCOME_EVENT_INDEX,
            "event_count": events.len(),
            "event_root_hash": hx(event_root.as_ref()),
            "event": hx(&bcs::to_bytes(target).unwrap()),
            "event_hash": hx(event_hashes[OUTCOME_EVENT_INDEX].as_ref()),
            "event_accumulator_proof": hx(&bcs::to_bytes(&event_proof).unwrap()),
            "siblings": event_proof.siblings().iter().map(|h| hx(h.as_ref())).collect::<Vec<_>>(),
            "type_tag": OUTCOME_TYPE_TAG,
            "event_data": hx(target.event_data()),
            "auction_outcome_layout": "struct AuctionOutcome { window_id: u64, pair_hash: vector<u8> (32), eth_lock_deadline: u64, dispositions_root: vector<u8> (32), leaf_count: u64 }",
            "auction_outcome": {
                "window_id": outcome.window_id,
                "pair_hash": hx(&outcome.pair_hash),
                "eth_lock_deadline": outcome.eth_lock_deadline,
                "dispositions_root": hx(&outcome.dispositions_root),
                "leaf_count": outcome.leaf_count,
            },
        },
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap();
    text.push('\n');
    text
}

fn seed_json(name: &str, seed: &[u8; 32]) -> Value {
    let salt = format!("APTOS::{}", name);
    assert_eq!(
        &HashValue::sha3_256_of(salt.as_bytes()).to_vec()[..],
        &seed[..]
    );
    json!({ "type": name, "salt": salt, "seed": hx(seed) })
}

fn hasher_json<T: CryptoHash>(name: &str) -> Value {
    seed_json(name, <T::Hasher as CryptoHasher>::seed())
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap()
}

/// Writes the vectors. Deterministic: seeded keys, synthetic ledger data, deterministic BLS.
#[test]
#[ignore]
fn write_atomica_outcome_proof_vectors() {
    let text = generate_json();
    std::fs::write(repo_root().join(JSON_PATH), &text).unwrap();
    println!("wrote {JSON_PATH} ({} bytes)", text.len());
}

// ---------------------------------------------------------------------------------------------
// Checks on the checked-in bytes, with production code
// ---------------------------------------------------------------------------------------------

fn checked_in() -> Value {
    serde_json::from_str(CHECKED_IN_JSON).unwrap()
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> T {
    bcs::from_bytes(&unhex(value)).unwrap()
}

/// Verifies the genesis waypoint and the epoch change, returning the trusted epoch 2 state.
fn verified_epoch_2_state(doc: &Value) -> EpochState {
    let genesis: LedgerInfoWithSignatures = decode(&doc["genesis"]["ledger_info_with_signatures"]);
    let waypoint = Waypoint::from_str(doc["genesis"]["waypoint"].as_str().unwrap()).unwrap();
    assert_eq!(
        waypoint,
        Waypoint::new_epoch_boundary(genesis.ledger_info()).unwrap()
    );
    assert_eq!(
        unhex(&doc["genesis"]["waypoint_value"]),
        waypoint.value().to_vec()
    );
    let epoch_change: LedgerInfoWithSignatures =
        decode(&doc["epoch_change"]["ledger_info_with_signatures"]);
    let proof = EpochChangeProof::new(vec![genesis, epoch_change], false);
    let state = proof
        .verify(&waypoint)
        .unwrap()
        .ledger_info()
        .next_epoch_state()
        .cloned()
        .unwrap();
    assert_eq!(state.epoch, 2);
    state
}

#[test]
fn atomica_outcome_proof_vectors_epoch_change_verifies_from_waypoint() {
    let doc = checked_in();
    let state = verified_epoch_2_state(&doc);
    // The epoch 2 set in the JSON is the one the epoch change names.
    let set = &doc["validator_sets"][1];
    assert_eq!(
        unhex(&set["epoch_state_bcs"]),
        bcs::to_bytes(&state).unwrap()
    );

    // The chain does not verify from a wrong genesis waypoint.
    let epoch_change: LedgerInfoWithSignatures =
        decode(&doc["epoch_change"]["ledger_info_with_signatures"]);
    let genesis: LedgerInfoWithSignatures = decode(&doc["genesis"]["ledger_info_with_signatures"]);
    let wrong = Waypoint::from_str(&format!("0:{}", HashValue::zero().to_hex())).unwrap();
    assert!(
        EpochChangeProof::new(vec![genesis, epoch_change.clone()], false)
            .verify(&wrong)
            .is_err()
    );
    let epoch_1: EpochState = decode(&doc["validator_sets"][0]["epoch_state_bcs"]);
    let verifier = ValidatorVerifier::new(epoch_1.verifier.validator_infos.clone());
    EpochState::new(1, verifier).verify(&epoch_change).unwrap();
}

#[test]
fn atomica_outcome_proof_vectors_ledger_infos_accept_and_reject() {
    let doc = checked_in();
    let state = verified_epoch_2_state(&doc);
    let cases = doc["outcome_ledger_infos"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let bytes = unhex(&case["ledger_info_with_signatures"]);
        let result = bcs::from_bytes::<LedgerInfoWithSignatures>(&bytes)
            .map_err(anyhow::Error::from)
            .and_then(|li| state.verify(&li));
        match case["expected"].as_str().unwrap() {
            "accept" => result.unwrap(),
            "reject" => assert!(result.is_err(), "{} verified", case["name"]),
            other => panic!("unknown expectation {}", other),
        }
    }
}

#[test]
fn atomica_outcome_proof_vectors_signing_intermediates() {
    let doc = checked_in();
    let mut cases = vec![doc["epoch_change"].clone()];
    cases.extend(
        doc["outcome_ledger_infos"]
            .as_array()
            .unwrap()
            .iter()
            .cloned(),
    );
    let sets = doc["validator_sets"].as_array().unwrap();
    let mut pairings_checked = 0;
    for case in &cases {
        let signed: LedgerInfoWithSignatures = decode(&case["ledger_info_with_signatures"]);
        let signing = &case["signing"];
        let message = signing_message(signed.ledger_info()).unwrap();
        assert_eq!(unhex(&signing["signing_message"]), message);
        let h = hash_to_g2(&message);
        assert_eq!(unhex(&signing["hash_to_g2_eip2537"]), g2_eip2537(&h));

        // The signature bytes are the tail of the BCS.
        let bytes = unhex(&case["ledger_info_with_signatures"]);
        let signature = unhex(&signing["signature"]);
        assert_eq!(&bytes[bytes.len() - signature.len()..], &signature[..]);

        let accept = case["expected"] == "accept";
        if accept {
            // e(aggregate public key, H(m)) == e(G1, signature), the check the EVM makes.
            let agg = &signing["aggregate_public_key_eip2537"];
            let sig = decode_g2(&signature).unwrap();
            assert_eq!(unhex(&signing["signature_eip2537"]), g2_eip2537(&sig));
            let set = sets
                .iter()
                .find(|s| s["epoch"] == signed.ledger_info().epoch())
                .unwrap();
            let pk: G1Projective = signing["signer_indexes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| {
                    let v = &set["validators"][i.as_u64().unwrap() as usize];
                    let key = decode_g1(&unhex(&v["public_key"]));
                    assert_eq!(unhex(&v["public_key_eip2537"]), g1_eip2537(&key));
                    key.into_group()
                })
                .sum();
            assert_eq!(unhex(agg), g1_eip2537(&pk.into_affine()));
            assert_eq!(
                Bls12_381::pairing(pk.into_affine(), h),
                Bls12_381::pairing(G1Affine::generator(), sig)
            );
            pairings_checked += 1;
        }
    }
    assert_eq!(pairings_checked, 2);
}

#[test]
fn atomica_outcome_proof_vectors_event_inclusion() {
    let doc = checked_in();
    let state = verified_epoch_2_state(&doc);
    let quorum: LedgerInfoWithSignatures =
        decode(&doc["outcome_ledger_infos"][0]["ledger_info_with_signatures"]);
    state.verify(&quorum).unwrap();

    let txn = &doc["transaction"];
    let version = txn["version"].as_u64().unwrap();
    assert!(txn["leaf_count"].as_u64().unwrap() >= 1 << 10);
    let txn_info: TransactionInfo = decode(&txn["transaction_info"]);
    assert!(txn_info.status().is_success());
    assert_eq!(
        unhex(&txn["transaction_info_hash"]),
        txn_info.hash().to_vec()
    );
    let txn_proof: TransactionAccumulatorProof = decode(&txn["transaction_accumulator_proof"]);
    assert!(txn_proof.siblings().len() >= 10);
    TransactionInfoWithProof::new(txn_proof.clone(), txn_info.clone())
        .verify(quorum.ledger_info(), version)
        .unwrap();
    assert!(TransactionInfoWithProof::new(txn_proof, txn_info.clone())
        .verify(quorum.ledger_info(), version + 1)
        .is_err());

    let ev = &doc["event"];
    let event: ContractEvent = decode(&ev["event"]);
    let index = ev["index"].as_u64().unwrap();
    let event_proof: EventAccumulatorProof = decode(&ev["event_accumulator_proof"]);
    assert_eq!(
        unhex(&ev["event_root_hash"]),
        txn_info.event_root_hash().to_vec()
    );
    event_proof
        .verify(txn_info.event_root_hash(), event.hash(), index)
        .unwrap();
    assert_eq!(
        event.type_tag().to_canonical_string(),
        ev["type_tag"].as_str().unwrap()
    );
    assert_eq!(event.type_tag().to_canonical_string(), OUTCOME_TYPE_TAG);
    let outcome: AuctionOutcome = bcs::from_bytes(event.event_data()).unwrap();
    assert_eq!(outcome, auction_outcome());
    assert_eq!(
        ev["auction_outcome"]["window_id"].as_u64().unwrap(),
        outcome.window_id
    );
}

#[test]
fn atomica_outcome_proof_vectors_hasher_seeds() {
    let doc = checked_in();
    for seed in doc["hasher_seeds"].as_array().unwrap() {
        let salt = seed["salt"].as_str().unwrap();
        assert_eq!(
            unhex(&seed["seed"]),
            HashValue::sha3_256_of(salt.as_bytes()).to_vec()
        );
    }
    assert_eq!(
        unhex(&doc["hasher_seeds"][0]["seed"]),
        <LedgerInfo as CryptoHash>::Hasher::seed().to_vec()
    );
}

#[test]
fn atomica_outcome_proof_vectors_regeneration_reproduces_checked_in_bytes() {
    let text = generate_json();
    assert!(
        text == CHECKED_IN_JSON,
        "the checked-in JSON differs from a fresh generation: rerun `{GENERATOR_COMMAND}`"
    );
    // A second generation in the same process gives the same bytes (no hidden entropy).
    assert!(generate_json() == text);
}
