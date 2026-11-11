// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Both sides of certification: a peer signs a requested observation only when its own Ethereum view
//! agrees ([`sign_if_own_view_agrees`]), and the requester aggregates matching signatures until they hold
//! 2/3+ of the voting power ([`SignatureAggregation`]).

use crate::{
    eth_rpc::EthHeaderSource,
    types::{EthHeaderMsg, SignResponse},
};
use anyhow::{anyhow, ensure, Result};
use aptos_consensus_types::common::Author;
use aptos_crypto::{bls12381::PrivateKey, SigningKey};
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::PartialSignatures,
    epoch_state::EpochState,
    eth_header::{EthHeaderObservation, QuorumCertifiedEthHeader},
};
use std::sync::Arc;

/// Checks a requested observation against this validator's own Ethereum view and signs it if it agrees:
/// same epoch, same configured chain, a source on that chain, the block at or below the own head at the
/// configured tag, and the same block hash and state root for that block number.
pub async fn sign_if_own_view_agrees(
    request: &EthHeaderObservation,
    epoch: u64,
    eth_chain_id: u64,
    my_addr: AccountAddress,
    consensus_key: &PrivateKey,
    source: &dyn EthHeaderSource,
) -> Result<SignResponse> {
    ensure!(
        request.epoch == epoch,
        "request is for epoch {}",
        request.epoch
    );
    ensure!(
        request.eth_chain_id == eth_chain_id,
        "request is for chain {}",
        request.eth_chain_id
    );
    let own_chain_id = source.chain_id().await?;
    ensure!(
        own_chain_id == eth_chain_id,
        "own RPC is on chain {own_chain_id}, expected {eth_chain_id}"
    );
    let head = source.head().await?;
    ensure!(
        request.block_number <= head.number,
        "block {} is above own head {}",
        request.block_number,
        head.number
    );
    let own = source.header_by_number(request.block_number).await?;
    ensure!(
        own.hash == request.block_hash && own.state_root == request.state_root,
        "own RPC returns a different header for block {}",
        request.block_number
    );
    let signature = consensus_key
        .sign(request)
        .map_err(|e| anyhow!("signing failed: {e}"))?;
    Ok(SignResponse {
        epoch,
        author: my_addr,
        signature,
    })
}

/// Reliable broadcast state of one observation: collects valid signatures over exactly this observation.
pub struct SignatureAggregation {
    epoch_state: Arc<EpochState>,
    observation: EthHeaderObservation,
    signatures: Mutex<PartialSignatures>,
}

impl SignatureAggregation {
    pub fn new(epoch_state: Arc<EpochState>, observation: EthHeaderObservation) -> Self {
        Self {
            epoch_state,
            observation,
            signatures: Mutex::new(PartialSignatures::empty()),
        }
    }
}

impl BroadcastStatus<EthHeaderMsg> for Arc<SignatureAggregation> {
    type Aggregated = QuorumCertifiedEthHeader;
    type Message = EthHeaderObservation;
    type Response = SignResponse;

    fn add(&self, sender: Author, response: SignResponse) -> Result<Option<Self::Aggregated>> {
        let SignResponse {
            epoch,
            author,
            signature,
        } = response;
        ensure!(
            epoch == self.epoch_state.epoch,
            "response has a wrong epoch"
        );
        ensure!(author == sender, "response author is not the sender");
        let verifier = &self.epoch_state.verifier;
        verifier.verify(sender, &self.observation, &signature)?;

        let mut signatures = self.signatures.lock();
        signatures.add_signature(sender, signature);
        if verifier
            .check_voting_power(signatures.signatures().keys(), true)
            .is_err()
        {
            return Ok(None);
        }
        let multi_sig = verifier
            .aggregate_signatures(signatures.signatures_iter())
            .map_err(|e| anyhow!("signature aggregation failed: {e}"))?;
        Ok(Some(QuorumCertifiedEthHeader {
            observation: self.observation.clone(),
            multi_sig,
        }))
    }
}

#[cfg(test)]
mod tests;
