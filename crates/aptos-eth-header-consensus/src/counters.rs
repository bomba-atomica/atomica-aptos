// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_gauge, IntGauge};
use once_cell::sync::Lazy;

/// Count of the pending messages sent to itself in the channel.
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_eth_header_consensus_pending_self_messages",
        "Count of the pending Ethereum header consensus messages sent to itself in the channel"
    )
    .unwrap()
});
