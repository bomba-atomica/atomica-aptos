// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::eth_rpc::EthHeader;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    validator_signer::ValidatorSigner,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use std::collections::HashMap;

const EPOCH: u64 = 3;
const CHAIN: u64 = 31337;

/// An Ethereum view with a fixed chain id, head and headers.
struct StubSource {
    chain_id: u64,
    head: u64,
    headers: HashMap<u64, EthHeader>,
}

impl StubSource {
    fn agreeing() -> Self {
        let mut headers = HashMap::new();
        for number in 0..=10 {
            headers.insert(number, header(number, number as u8));
        }
        Self {
            chain_id: CHAIN,
            head: 10,
            headers,
        }
    }
}

#[async_trait::async_trait]
impl EthHeaderSource for StubSource {
    async fn chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn head(&self) -> Result<EthHeader> {
        self.header_by_number(self.head).await
    }

    async fn header_by_number(&self, number: u64) -> Result<EthHeader> {
        self.headers
            .get(&number)
            .copied()
            .ok_or_else(|| anyhow!("no block {number}"))
    }
}

fn header(number: u64, fill: u8) -> EthHeader {
    EthHeader {
        number,
        hash: [fill; 32],
        state_root: [fill.wrapping_add(100); 32],
    }
}

fn observation(number: u64, fill: u8) -> EthHeaderObservation {
    let h = header(number, fill);
    EthHeaderObservation {
        epoch: EPOCH,
        eth_chain_id: CHAIN,
        block_number: number,
        block_hash: h.hash,
        state_root: h.state_root,
    }
}

fn signers() -> Vec<ValidatorSigner> {
    (0..4u8).map(|i| ValidatorSigner::random([i; 32])).collect()
}

fn epoch_state(signers: &[ValidatorSigner]) -> Arc<EpochState> {
    let infos = signers
        .iter()
        .map(|s| ValidatorConsensusInfo::new(s.author(), s.public_key(), 1))
        .collect();
    Arc::new(EpochState::new(EPOCH, ValidatorVerifier::new(infos)))
}

async fn peer_signs(
    signer: &ValidatorSigner,
    request: &EthHeaderObservation,
    source: &StubSource,
) -> Result<SignResponse> {
    sign_if_own_view_agrees(
        request,
        EPOCH,
        CHAIN,
        signer.author(),
        signer.private_key(),
        source,
    )
    .await
}

#[tokio::test]
async fn peer_signs_when_its_own_rpc_agrees() {
    let signers = signers();
    let request = observation(8, 8);
    let response = peer_signs(&signers[1], &request, &StubSource::agreeing())
        .await
        .unwrap();
    assert_eq!(response.author, signers[1].author());
    assert_eq!(response.epoch, EPOCH);
    epoch_state(&signers)
        .verifier
        .verify(signers[1].author(), &request, &response.signature)
        .unwrap();
}

#[tokio::test]
async fn peer_whose_rpc_returns_a_different_header_refuses_to_sign() {
    let signers = signers();
    let mut source = StubSource::agreeing();
    // Same block number, different state root.
    let mut other_root = header(8, 8);
    other_root.state_root = [0xEE; 32];
    source.headers.insert(8, other_root);
    let err = peer_signs(&signers[1], &observation(8, 8), &source)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("different header"), "{err}");

    // Same block number, different block hash.
    let mut source = StubSource::agreeing();
    let mut other_hash = header(8, 8);
    other_hash.hash = [0xEE; 32];
    source.headers.insert(8, other_hash);
    assert!(peer_signs(&signers[1], &observation(8, 8), &source)
        .await
        .is_err());
}

#[tokio::test]
async fn peer_refuses_a_block_above_its_own_head() {
    let signers = signers();
    let mut source = StubSource::agreeing();
    source.head = 7;
    let err = peer_signs(&signers[1], &observation(8, 8), &source)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("above own head"), "{err}");
}

#[tokio::test]
async fn peer_refuses_wrong_epoch_or_chain() {
    let signers = signers();
    let source = StubSource::agreeing();
    let mut wrong_epoch = observation(8, 8);
    wrong_epoch.epoch = EPOCH + 1;
    assert!(peer_signs(&signers[1], &wrong_epoch, &source)
        .await
        .is_err());
    let mut wrong_chain = observation(8, 8);
    wrong_chain.eth_chain_id = 1;
    assert!(peer_signs(&signers[1], &wrong_chain, &source)
        .await
        .is_err());
    // The own RPC is on another chain.
    let mut other_network = StubSource::agreeing();
    other_network.chain_id = 1;
    assert!(peer_signs(&signers[1], &observation(8, 8), &other_network)
        .await
        .is_err());
}

#[tokio::test]
async fn aggregation_certifies_at_quorum_only() {
    let signers = signers();
    let state = epoch_state(&signers);
    let request = observation(8, 8);
    let aggregation = Arc::new(SignatureAggregation::new(state.clone(), request.clone()));
    let source = StubSource::agreeing();

    let mut certified = None;
    for (i, signer) in signers.iter().enumerate().take(3) {
        let response = peer_signs(signer, &request, &source).await.unwrap();
        certified = aggregation.add(signer.author(), response).unwrap();
        assert_eq!(certified.is_some(), i == 2, "quorum is 3 of 4");
    }
    let certified = certified.unwrap();
    assert_eq!(certified.observation, request);
    state
        .verifier
        .verify_multi_signatures(&request, &certified.multi_sig)
        .unwrap();
}

#[tokio::test]
async fn aggregation_rejects_bad_responses() {
    let signers = signers();
    let state = epoch_state(&signers);
    let request = observation(8, 8);
    let aggregation = Arc::new(SignatureAggregation::new(state, request.clone()));

    // A signature over another observation.
    let other = SignResponse {
        epoch: EPOCH,
        author: signers[0].author(),
        signature: signers[0].sign(&observation(8, 9)).unwrap(),
    };
    assert!(aggregation.add(signers[0].author(), other).is_err());

    // A valid signature claimed by another sender.
    let good = SignResponse {
        epoch: EPOCH,
        author: signers[0].author(),
        signature: signers[0].sign(&request).unwrap(),
    };
    assert!(aggregation.add(signers[1].author(), good.clone()).is_err());

    // A wrong epoch.
    let mut stale = good;
    stale.epoch = EPOCH - 1;
    assert!(aggregation.add(signers[0].author(), stale).is_err());
}
