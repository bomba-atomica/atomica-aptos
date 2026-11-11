// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Atomica proof endpoints.
//!
//! These read-only endpoints serve everything an Ethereum contract needs to check that an event is
//! final on Aptos: the validator-signed `LedgerInfoWithSignatures`, the `TransactionInfo` and its
//! accumulator proof, the `ContractEvent` and its event accumulator proof, and the epoch change
//! chain from the genesis waypoint. Every value is hex BCS of the pinned aptos-types struct. Nothing
//! here is trusted: a verifier checks every byte against the validator quorum.

use crate::{
    context::{api_spawn_blocking, Context},
    failpoint::fail_point_poem,
    response::{
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        InternalError, NotFoundError,
    },
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{AptosErrorCode, HexEncodedBytes, LedgerInfo, U64};
use aptos_crypto::hash::{
    CryptoHash, CryptoHasher, EventAccumulatorHasher, HashValue, ACCUMULATOR_PLACEHOLDER_HASH,
};
use aptos_storage_interface::AptosDbError;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::{AccumulatorProof, MerkleTreeInternalNode},
    transaction::Version,
    waypoint::Waypoint,
};
use poem_openapi::{
    param::{Path, Query},
    Object, OpenApi,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Proof that an event is part of a transaction committed under a validator-signed LedgerInfo
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct AtomicaEventProof {
    /// BCS of the latest `LedgerInfoWithSignatures`
    pub ledger_info_with_signatures: HexEncodedBytes,
    /// Version of the transaction that emitted the event
    pub version: U64,
    /// BCS of the `TransactionInfo` at `version`
    pub transaction_info: HexEncodedBytes,
    /// BCS of the `TransactionAccumulatorProof` from `transaction_info` to the
    /// LedgerInfo's `executed_state_id`
    pub transaction_accumulator_proof: HexEncodedBytes,
    /// Index of the event within the transaction
    pub event_index: U64,
    /// BCS of the `ContractEvent`
    pub event: HexEncodedBytes,
    /// BCS of the `EventAccumulatorProof` from `event` to `transaction_info.event_root_hash`
    pub event_accumulator_proof: HexEncodedBytes,
}

/// Epoch-ending LedgerInfos for the epochs `[start_epoch, end_epoch)`
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct AtomicaEpochChangeProof {
    /// BCS of each epoch-ending `LedgerInfoWithSignatures`, oldest first. Each one carries
    /// `next_epoch_state` with the validator set of the next epoch.
    pub ledger_infos_with_signatures: Vec<HexEncodedBytes>,
    /// True when the node returned a page and more epoch changes follow
    pub more: bool,
}

/// The genesis waypoint of the chain
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct AtomicaGenesisWaypoint {
    /// Waypoint in its text form, `version:hash`
    pub waypoint: String,
    /// Version of the genesis LedgerInfo
    pub version: U64,
    /// Waypoint hash of the genesis LedgerInfo
    pub value: HexEncodedBytes,
    /// BCS of the genesis `LedgerInfoWithSignatures` (epoch 0)
    pub ledger_info_with_signatures: HexEncodedBytes,
}

/// Atomica proof endpoints for cross-chain verification
#[derive(Clone)]
pub struct AtomicaProofsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl AtomicaProofsApi {
    /// Get an event inclusion proof
    ///
    /// Returns the latest validator-signed LedgerInfo, the TransactionInfo at `version` with its
    /// accumulator proof, and the event at `event_index` with its event accumulator proof. All
    /// values are hex BCS. Returns 404 for a pruned or unknown version and 400 for an event index
    /// out of range.
    #[oai(
        path = "/atomica/event_proof/:version/:event_index",
        method = "get",
        operation_id = "atomica_event_proof",
        tag = "ApiTags::Atomica"
    )]
    async fn event_proof(
        &self,
        /// Ledger version of the transaction that emitted the event
        version: Path<u64>,
        /// Index of the event within the transaction
        event_index: Path<u64>,
    ) -> BasicResultWith404<AtomicaEventProof> {
        fail_point_poem("endpoint_atomica_event_proof")?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_event_proof(version.0, event_index.0)).await
    }

    /// Get the epoch change proof
    ///
    /// Returns the epoch-ending LedgerInfos for the epochs `[start_epoch, end_epoch)`. Without
    /// `end_epoch` it returns every sealed epoch from `start_epoch`. The node pages long ranges and
    /// sets `more`.
    #[oai(
        path = "/atomica/epoch_change_proof",
        method = "get",
        operation_id = "atomica_epoch_change_proof",
        tag = "ApiTags::Atomica"
    )]
    async fn epoch_change_proof(
        &self,
        /// First epoch whose ending LedgerInfo is returned
        start_epoch: Query<u64>,
        /// Exclusive upper bound. Defaults to the first epoch that is not yet sealed.
        end_epoch: Query<Option<u64>>,
    ) -> BasicResultWith404<AtomicaEpochChangeProof> {
        fail_point_poem("endpoint_atomica_epoch_change_proof")?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_epoch_change_proof(start_epoch.0, end_epoch.0)).await
    }

    /// Get the genesis waypoint
    ///
    /// Returns the waypoint of the genesis LedgerInfo, which an external verifier pins at deploy.
    #[oai(
        path = "/atomica/genesis_waypoint",
        method = "get",
        operation_id = "atomica_genesis_waypoint",
        tag = "ApiTags::Atomica"
    )]
    async fn genesis_waypoint(&self) -> BasicResultWith404<AtomicaGenesisWaypoint> {
        fail_point_poem("endpoint_atomica_genesis_waypoint")?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_genesis_waypoint()).await
    }
}

impl AtomicaProofsApi {
    pub(crate) fn get_event_proof(
        &self,
        version: Version,
        event_index: u64,
    ) -> BasicResultWith404<AtomicaEventProof> {
        let ledger_info = self.context.get_latest_storage_ledger_info()?;
        // Read the signed LedgerInfo once and build every proof against its version.
        let signed = self.signed_ledger_info(&ledger_info)?;
        let ledger_version = signed.ledger_info().version();
        if version > ledger_version {
            return Err(version_not_found(version, &ledger_info));
        }

        let txn = self
            .context
            .db
            .get_transaction_by_version(version, ledger_version, true)
            .map_err(|err| {
                storage_error(
                    anyhow::Error::from(err).context("Failed to read transaction with proof"),
                    version,
                    &ledger_info,
                )
            })?;
        let events = txn.events.unwrap_or_default();
        let index = usize::try_from(event_index)
            .ok()
            .filter(|i| *i < events.len())
            .ok_or_else(|| {
                BasicErrorWith404::bad_request_with_code(
                    format!(
                        "Event index({}) is out of range: the transaction at version {} has {} events",
                        event_index,
                        version,
                        events.len()
                    ),
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                )
            })?;
        let leaves: Vec<HashValue> = events.iter().map(CryptoHash::hash).collect();
        let event_proof = accumulator_proof::<EventAccumulatorHasher>(&leaves, index);
        let proof = &txn.proof;

        let response = AtomicaEventProof {
            ledger_info_with_signatures: to_bcs(&signed, &ledger_info)?,
            version: version.into(),
            transaction_info: to_bcs(proof.transaction_info(), &ledger_info)?,
            transaction_accumulator_proof: to_bcs(
                proof.ledger_info_to_transaction_info_proof(),
                &ledger_info,
            )?,
            event_index: event_index.into(),
            event: to_bcs(&events[index], &ledger_info)?,
            event_accumulator_proof: to_bcs(&event_proof, &ledger_info)?,
        };
        BasicResponse::try_from_json((response, &ledger_info, BasicResponseStatus::Ok))
    }

    fn get_epoch_change_proof(
        &self,
        start_epoch: u64,
        end_epoch: Option<u64>,
    ) -> BasicResultWith404<AtomicaEpochChangeProof> {
        let ledger_info = self.context.get_latest_storage_ledger_info()?;
        let signed = self.signed_ledger_info(&ledger_info)?;
        // The latest LedgerInfo seals its own epoch only when it carries the next validator set.
        let first_open_epoch = signed.ledger_info().next_block_epoch();
        let end_epoch = end_epoch.unwrap_or(first_open_epoch);
        if start_epoch >= end_epoch || end_epoch > first_open_epoch {
            return Err(BasicErrorWith404::bad_request_with_code(
                format!(
                    "Bad epoch range [{}, {}): it must be non-empty and end at or before {}, the first epoch that is not sealed",
                    start_epoch, end_epoch, first_open_epoch
                ),
                AptosErrorCode::InvalidInput,
                &ledger_info,
            ));
        }

        let proof = self
            .context
            .db
            .get_epoch_ending_ledger_infos(start_epoch, end_epoch)
            .context("Failed to read epoch ending ledger infos")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;
        let ledger_infos_with_signatures = proof
            .ledger_info_with_sigs
            .iter()
            .map(|li| to_bcs(li, &ledger_info))
            .collect::<Result<Vec<_>, _>>()?;
        let response = AtomicaEpochChangeProof {
            ledger_infos_with_signatures,
            more: proof.more,
        };
        BasicResponse::try_from_json((response, &ledger_info, BasicResponseStatus::Ok))
    }

    fn get_genesis_waypoint(&self) -> BasicResultWith404<AtomicaGenesisWaypoint> {
        let ledger_info = self.context.get_latest_storage_ledger_info()?;
        let proof = self
            .context
            .db
            .get_epoch_ending_ledger_infos(0, 1)
            .context("Failed to read the genesis ledger info")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;
        let genesis = proof.ledger_info_with_sigs.first().ok_or_else(|| {
            BasicErrorWith404::not_found_with_code(
                "Genesis ledger info not found",
                AptosErrorCode::VersionNotFound,
                &ledger_info,
            )
        })?;
        let waypoint = Waypoint::new_epoch_boundary(genesis.ledger_info())
            .context("The genesis ledger info does not end an epoch")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;
        let response = AtomicaGenesisWaypoint {
            waypoint: waypoint.to_string(),
            version: waypoint.version().into(),
            value: HexEncodedBytes(waypoint.value().to_vec()),
            ledger_info_with_signatures: to_bcs(genesis, &ledger_info)?,
        };
        BasicResponse::try_from_json((response, &ledger_info, BasicResponseStatus::Ok))
    }

    fn signed_ledger_info(
        &self,
        ledger_info: &LedgerInfo,
    ) -> Result<LedgerInfoWithSignatures, BasicErrorWith404> {
        self.context
            .get_latest_ledger_info_with_signatures()
            .context("Failed to read the latest signed ledger info")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    ledger_info,
                )
            })
    }
}

/// Builds the proof for leaf `index` of an in-memory Merkle accumulator over `leaves`.
///
/// It matches `InMemoryAccumulator::from_leaves`: at each level a missing right sibling is the
/// placeholder hash. Siblings run from the leaf level to the root, as `AccumulatorProof` expects.
/// The caller guarantees `index < leaves.len()`.
pub fn accumulator_proof<H: CryptoHasher>(
    leaves: &[HashValue],
    index: usize,
) -> AccumulatorProof<H> {
    let mut siblings = Vec::new();
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
    AccumulatorProof::new(siblings)
}

fn to_bcs<T: Serialize>(
    value: &T,
    ledger_info: &LedgerInfo,
) -> Result<HexEncodedBytes, BasicErrorWith404> {
    bcs::to_bytes(value).map(HexEncodedBytes).map_err(|err| {
        BasicErrorWith404::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
    })
}

fn version_not_found(version: Version, ledger_info: &LedgerInfo) -> BasicErrorWith404 {
    BasicErrorWith404::not_found_with_code(
        format!("Ledger version({}) not found", version),
        AptosErrorCode::VersionNotFound,
        ledger_info,
    )
}

/// A pruned or missing version is a 404: the proof can no longer be served from this node.
fn storage_error(
    err: anyhow::Error,
    version: Version,
    ledger_info: &LedgerInfo,
) -> BasicErrorWith404 {
    match err
        .chain()
        .find_map(|cause| cause.downcast_ref::<AptosDbError>())
    {
        Some(AptosDbError::LedgerPruned { .. }) => BasicErrorWith404::not_found_with_code(
            format!("Ledger version({}) has been pruned", version),
            AptosErrorCode::VersionPruned,
            ledger_info,
        ),
        Some(AptosDbError::NotFound(_)) => version_not_found(version, ledger_info),
        _ => BasicErrorWith404::internal_with_code(err, AptosErrorCode::InternalError, ledger_info),
    }
}
