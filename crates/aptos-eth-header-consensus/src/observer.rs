// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::eth_rpc::{EthHeader, EthHeaderSource};
use aptos_channels::aptos_channel;
use aptos_logger::{debug, warn};
use futures::{FutureExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};

/// A task that polls the own Ethereum RPC for the head at the configured tag and pushes it to the manager.
pub struct EthHeaderObserver {
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl EthHeaderObserver {
    pub fn spawn(
        source: Arc<dyn EthHeaderSource>,
        eth_chain_id: u64,
        poll_interval: Duration,
        observation_tx: aptos_channel::Sender<(), EthHeader>,
    ) -> Self {
        let (close_tx, close_rx) = oneshot::channel();
        let join_handle = tokio::spawn(Self::run(
            source,
            eth_chain_id,
            poll_interval,
            observation_tx,
            close_rx,
        ));
        Self {
            close_tx,
            join_handle,
        }
    }

    async fn run(
        source: Arc<dyn EthHeaderSource>,
        eth_chain_id: u64,
        poll_interval: Duration,
        observation_tx: aptos_channel::Sender<(), EthHeader>,
        close_rx: oneshot::Receiver<()>,
    ) {
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut close_rx = close_rx.into_stream();
        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    match observe(source.as_ref(), eth_chain_id).await {
                        Ok(header) => {
                            debug!(block_number = header.number, "Ethereum head observed.");
                            let _ = observation_tx.push((), header);
                        },
                        Err(e) => warn!("Ethereum header observation failed: {e:#}"),
                    }
                },
                _ = close_rx.select_next_some() => break,
            }
        }
    }

    pub async fn shutdown(self) {
        let _ = self.close_tx.send(());
        let _ = self.join_handle.await;
    }
}

async fn observe(source: &dyn EthHeaderSource, eth_chain_id: u64) -> anyhow::Result<EthHeader> {
    let chain_id = source.chain_id().await?;
    anyhow::ensure!(
        chain_id == eth_chain_id,
        "own RPC is on chain {chain_id}, the chain config says {eth_chain_id}"
    );
    source.head().await
}
