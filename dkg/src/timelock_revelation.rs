// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::warn;
use move_core_types::account_address::AccountAddress;

pub struct TimelockRevelationManager {
    my_addr: AccountAddress,
}

impl TimelockRevelationManager {
    pub fn new(my_addr: AccountAddress) -> Self {
        warn!("[TIMELOCK] TimelockRevelationManager created but not yet functional");
        warn!("[TIMELOCK] Secret revelation is NOT IMPLEMENTED - tests will fail");
        Self { my_addr }
    }

    pub fn maybe_reveal_for_past_interval(&mut self, _current_interval: u64) {
        // Placeholder
    }
}
