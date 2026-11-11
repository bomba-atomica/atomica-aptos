// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::bls12381::Signature;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use aptos_types::{account_address::AccountAddress, eth_header::EthHeaderObservation};
use serde::{Deserialize, Serialize};

/// The network messages of Ethereum header attestation.
#[derive(Clone, Debug, EnumConversion, Deserialize, Serialize, PartialEq)]
pub enum EthHeaderMsg {
    /// "Sign this observation if your own Ethereum RPC returns the same header."
    SignRequest(EthHeaderObservation),
    /// A signature over the requested observation.
    SignResponse(SignResponse),
}

impl EthHeaderMsg {
    pub fn name(&self) -> &str {
        match self {
            EthHeaderMsg::SignRequest(_) => "SignRequest",
            EthHeaderMsg::SignResponse(_) => "SignResponse",
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            EthHeaderMsg::SignRequest(observation) => observation.epoch,
            EthHeaderMsg::SignResponse(response) => response.epoch,
        }
    }
}

impl RBMessage for EthHeaderMsg {}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct SignResponse {
    pub epoch: u64,
    pub author: AccountAddress,
    pub signature: Signature,
}
