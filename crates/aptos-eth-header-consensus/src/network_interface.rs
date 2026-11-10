// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::types::EthHeaderMsg;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_network::{
    application::{error::Error, interface::NetworkClientInterface},
    ProtocolId,
};
use bytes::Bytes;
use move_core_types::account_address::AccountAddress as PeerId;
use std::{collections::HashMap, time::Duration};

/// The protocol has no direct-send messages.
pub const DIRECT_SEND: &[ProtocolId] = &[];

/// Supported protocols in preferred order (from highest priority to lowest).
pub const RPC: &[ProtocolId] = &[ProtocolId::EthHeaderConsensusRpcBcs];

#[derive(Clone)]
pub struct EthHeaderNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<EthHeaderMsg>> EthHeaderNetworkClient<NetworkClient> {
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: EthHeaderMsg,
        rpc_timeout: Duration,
    ) -> Result<EthHeaderMsg, Error> {
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, Self::peer_network_id(peer))
            .await
    }

    pub async fn send_rpc_raw(
        &self,
        peer: PeerId,
        message: Bytes,
        rpc_timeout: Duration,
    ) -> Result<EthHeaderMsg, Error> {
        self.network_client
            .send_to_peer_rpc_raw(message, rpc_timeout, Self::peer_network_id(peer))
            .await
    }

    pub fn to_bytes_by_protocol(
        &self,
        peers: Vec<PeerId>,
        message: EthHeaderMsg,
    ) -> anyhow::Result<HashMap<PeerId, Bytes>> {
        let peer_network_ids: Vec<PeerNetworkId> =
            peers.into_iter().map(Self::peer_network_id).collect();
        Ok(self
            .network_client
            .to_bytes_by_protocol(peer_network_ids, message)?
            .into_iter()
            .map(|(peer_network_id, bytes)| (peer_network_id.peer_id(), bytes))
            .collect())
    }

    pub fn sort_peers_by_latency(&self, peers: &mut [PeerId]) {
        self.network_client
            .sort_peers_by_latency(NetworkId::Validator, peers)
    }

    fn peer_network_id(peer: PeerId) -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, peer)
    }
}
