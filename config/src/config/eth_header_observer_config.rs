// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};

/// Ethereum header attestation of a validator (fork only). See
/// `atomica/docs/decisions/eth-header-attestation.md`.
///
/// A validator observes and signs Ethereum headers only while `enabled` is set and the chain has an
/// `EthereumHeaderConfig`. It trusts only its own `rpc_url`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EthHeaderObserverConfig {
    pub enabled: bool,
    /// The validator's own Ethereum JSON-RPC endpoint.
    pub rpc_url: String,
    /// Block tag of the observed head: `finalized` (default), `safe` or `latest`. A peer signs a header
    /// only when its block is at or below the peer's own head at this tag.
    pub block_tag: String,
    pub poll_interval_ms: u64,
    pub max_network_channel_size: usize,
}

impl Default for EthHeaderObserverConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rpc_url: String::new(),
            block_tag: "finalized".to_string(),
            poll_interval_ms: 12_000,
            max_network_channel_size: 256,
        }
    }
}
