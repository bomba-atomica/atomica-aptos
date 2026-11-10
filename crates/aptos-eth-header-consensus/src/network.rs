// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Network plumbing, copied from `aptos-jwk-consensus` with the message type changed.

use crate::{
    network_interface::{EthHeaderNetworkClient, RPC},
    types::EthHeaderMsg,
};
use anyhow::bail;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkId;
use aptos_consensus_types::common::Author;
use aptos_logger::warn;
use aptos_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_types::account_address::AccountAddress;
use bytes::Bytes;
use futures::Stream;
use futures_channel::oneshot;
use futures_util::{
    stream::{select, select_all, StreamExt},
    SinkExt,
};
use std::{collections::HashMap, time::Duration};

pub struct IncomingRpcRequest {
    pub msg: EthHeaderMsg,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

pub struct NetworkSender {
    author: AccountAddress,
    network_client: EthHeaderNetworkClient<NetworkClient<EthHeaderMsg>>,
    self_sender: aptos_channels::Sender<Event<EthHeaderMsg>>,
}

impl NetworkSender {
    pub fn new(
        author: AccountAddress,
        network_client: EthHeaderNetworkClient<NetworkClient<EthHeaderMsg>>,
        self_sender: aptos_channels::Sender<Event<EthHeaderMsg>>,
    ) -> Self {
        Self {
            author,
            network_client,
            self_sender,
        }
    }
}

#[async_trait::async_trait]
impl RBNetworkSender<EthHeaderMsg> for NetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        receiver: AccountAddress,
        raw_message: Bytes,
        timeout: Duration,
    ) -> anyhow::Result<EthHeaderMsg> {
        Ok(self
            .network_client
            .send_rpc_raw(receiver, raw_message, timeout)
            .await?)
    }

    async fn send_rb_rpc(
        &self,
        receiver: AccountAddress,
        message: EthHeaderMsg,
        timeout: Duration,
    ) -> anyhow::Result<EthHeaderMsg> {
        if receiver == self.author {
            let (tx, rx) = oneshot::channel();
            let protocol = RPC[0];
            let self_msg = Event::RpcRequest(self.author, message, protocol, tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = tokio::time::timeout(timeout, rx).await {
                let response_msg =
                    tokio::task::spawn_blocking(move || protocol.from_bytes(&bytes)).await??;
                Ok(response_msg)
            } else {
                bail!("self rpc failed");
            }
        } else {
            Ok(self
                .network_client
                .send_rpc(receiver, message, timeout)
                .await?)
        }
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<Author>,
        message: EthHeaderMsg,
    ) -> anyhow::Result<HashMap<Author, Bytes>> {
        self.network_client.to_bytes_by_protocol(peers, message)
    }

    fn sort_peers_by_latency(&self, peers: &mut [AccountAddress]) {
        self.network_client.sort_peers_by_latency(peers)
    }
}

pub trait RpcResponseSender: Send + Sync {
    fn send(&mut self, response: anyhow::Result<EthHeaderMsg>);
}

pub struct RealRpcResponseSender {
    pub inner: Option<oneshot::Sender<Result<Bytes, RpcError>>>,
    pub protocol: ProtocolId,
}

impl RpcResponseSender for RealRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<EthHeaderMsg>) {
        let rpc_response = response
            .and_then(|msg| self.protocol.to_bytes(&msg).map(Bytes::from))
            .map_err(RpcError::ApplicationError);
        if let Some(tx) = self.inner.take() {
            let _ = tx.send(rpc_response);
        }
    }
}

pub struct NetworkReceivers {
    pub rpc_rx: aptos_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

pub struct NetworkTask {
    all_events: Box<dyn Stream<Item = Event<EthHeaderMsg>> + Send + Unpin>,
    rpc_tx: aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

impl NetworkTask {
    pub fn new(
        network_service_events: NetworkServiceEvents<EthHeaderMsg>,
        self_receiver: aptos_channels::Receiver<Event<EthHeaderMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (rpc_tx, rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for Ethereum header consensus!");
        }

        let network_events: Vec<_> = network_and_events.into_values().collect();
        let network_events = select_all(network_events).fuse();
        let all_events = Box::new(select(network_events, self_receiver));

        (NetworkTask { rpc_tx, all_events }, NetworkReceivers {
            rpc_rx,
        })
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            if let Event::RpcRequest(peer_id, msg, protocol, response_sender) = message {
                let req = IncomingRpcRequest {
                    msg,
                    sender: peer_id,
                    response_sender: Box::new(RealRpcResponseSender {
                        inner: Some(response_sender),
                        protocol,
                    }),
                };
                if let Err(e) = self.rpc_tx.push(peer_id, (peer_id, req)) {
                    warn!(error = ?e, "aptos channel closed");
                };
            }
        }
    }
}
