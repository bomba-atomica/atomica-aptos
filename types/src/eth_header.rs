// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-attested Ethereum headers (fork only).
//!
//! Design: `atomica/docs/decisions/eth-header-attestation.md`. Each validator observes an Ethereum header
//! through its own RPC and signs an [`EthHeaderObservation`]. A peer signs only when its own RPC returns the
//! same header. Signatures that hold 2/3+ of the voting power aggregate into a [`QuorumCertifiedEthHeader`],
//! which a block proposer includes as the `ObservedEthHeader` validator transaction. The VM checks it with
//! [`check_certified_eth_header`] and stores the header in `0x1::ethereum_headers`.

use crate::{
    aggregate_signature::AggregateSignature, on_chain_config::OnChainConfig,
    validator_verifier::ValidatorVerifier,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use serde::{Deserialize, Serialize};

/// What one validator saw at one Ethereum block. This is the signed message: its `CryptoHasher` salt is
/// the type name, so a signature over it is valid for nothing else.
#[derive(
    Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub struct EthHeaderObservation {
    /// Aptos epoch of the signers. A certificate of an old epoch is rejected.
    pub epoch: u64,
    /// EIP-155 chain id of the observed Ethereum network.
    pub eth_chain_id: u64,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
}

/// An [`EthHeaderObservation`] with a multi-signature of validators that hold 2/3+ of the voting power.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuorumCertifiedEthHeader {
    pub observation: EthHeaderObservation,
    pub multi_sig: AggregateSignature,
}

/// Why a [`QuorumCertifiedEthHeader`] is rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EthHeaderRejection {
    EpochNotCurrent,
    ChainIdMismatch,
    NotEnoughVotingPower,
    MultiSigVerificationFailed,
}

/// The checks of a certified header that need no Move state: current epoch, configured Ethereum chain,
/// 2/3+ voting power of the signers, and the multi-signature against the current validator set.
pub fn check_certified_eth_header(
    certified: &QuorumCertifiedEthHeader,
    current_epoch: u64,
    eth_chain_id: u64,
    verifier: &ValidatorVerifier,
) -> Result<(), EthHeaderRejection> {
    let observation = &certified.observation;
    if observation.epoch != current_epoch {
        return Err(EthHeaderRejection::EpochNotCurrent);
    }
    if observation.eth_chain_id != eth_chain_id {
        return Err(EthHeaderRejection::ChainIdMismatch);
    }
    let signers = certified
        .multi_sig
        .get_signers_addresses(&verifier.get_ordered_account_addresses());
    verifier
        .check_voting_power(signers.iter(), true)
        .map_err(|_| EthHeaderRejection::NotEnoughVotingPower)?;
    verifier
        .verify_multi_signatures(observation, &certified.multi_sig)
        .map_err(|_| EthHeaderRejection::MultiSigVerificationFailed)?;
    Ok(())
}

/// Reflection of Move type `0x1::ethereum_header_config::EthereumHeaderConfig`. The resource exists only
/// while header attestation is on.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EthereumHeaderConfig {
    pub eth_chain_id: u64,
}

impl OnChainConfig for EthereumHeaderConfig {
    const MODULE_IDENTIFIER: &'static str = "ethereum_header_config";
    const TYPE_IDENTIFIER: &'static str = "EthereumHeaderConfig";
}

/// Reflection of Move type `0x1::ethereum_headers::EthereumHeaders`. `headers` is the handle of the table
/// of certified headers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EthereumHeaders {
    pub headers: AccountAddress,
    pub latest_block_number: Option<u64>,
}

impl OnChainConfig for EthereumHeaders {
    const MODULE_IDENTIFIER: &'static str = "ethereum_headers";
    const TYPE_IDENTIFIER: &'static str = "EthereumHeaders";
}

/// Reflection of Move event `0x1::ethereum_headers::HeaderCertified`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeaderCertified {
    pub eth_chain_id: u64,
    pub block_number: u64,
    pub block_hash: Vec<u8>,
    pub state_root: Vec<u8>,
}

impl MoveStructType for HeaderCertified {
    const MODULE_NAME: &'static IdentStr = ident_str!("ethereum_headers");
    const STRUCT_NAME: &'static IdentStr = ident_str!("HeaderCertified");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };
    use std::collections::BTreeMap;

    fn observation(epoch: u64) -> EthHeaderObservation {
        EthHeaderObservation {
            epoch,
            eth_chain_id: 31337,
            block_number: 100,
            block_hash: [1u8; 32],
            state_root: [2u8; 32],
        }
    }

    /// Four validators with voting power 1 each, so a quorum is 3 signers.
    fn setup() -> (Vec<ValidatorSigner>, ValidatorVerifier) {
        let signers: Vec<_> = (0..4).map(|i| ValidatorSigner::random([i; 32])).collect();
        let infos = signers
            .iter()
            .map(|s| ValidatorConsensusInfo::new(s.author(), s.public_key(), 1))
            .collect();
        (signers, ValidatorVerifier::new(infos))
    }

    fn certify(
        signers: &[ValidatorSigner],
        verifier: &ValidatorVerifier,
        signed: &EthHeaderObservation,
        claimed: EthHeaderObservation,
    ) -> QuorumCertifiedEthHeader {
        let sigs: BTreeMap<_, _> = signers
            .iter()
            .map(|s| (s.author(), s.sign(signed).unwrap()))
            .collect();
        QuorumCertifiedEthHeader {
            observation: claimed,
            multi_sig: verifier.aggregate_signatures(sigs.iter()).unwrap(),
        }
    }

    #[test]
    fn quorum_certificate_is_accepted() {
        let (signers, verifier) = setup();
        let obs = observation(5);
        let qc = certify(&signers[..3], &verifier, &obs, obs.clone());
        assert_eq!(check_certified_eth_header(&qc, 5, 31337, &verifier), Ok(()));
    }

    #[test]
    fn rejections() {
        let (signers, verifier) = setup();
        let obs = observation(5);
        let qc = certify(&signers[..3], &verifier, &obs, obs.clone());
        assert_eq!(
            check_certified_eth_header(&qc, 6, 31337, &verifier),
            Err(EthHeaderRejection::EpochNotCurrent)
        );
        assert_eq!(
            check_certified_eth_header(&qc, 5, 1, &verifier),
            Err(EthHeaderRejection::ChainIdMismatch)
        );
        let under = certify(&signers[..2], &verifier, &obs, obs.clone());
        assert_eq!(
            check_certified_eth_header(&under, 5, 31337, &verifier),
            Err(EthHeaderRejection::NotEnoughVotingPower)
        );
        let mut forged_root = obs.clone();
        forged_root.state_root = [9u8; 32];
        let forged = certify(&signers[..3], &verifier, &obs, forged_root);
        assert_eq!(
            check_certified_eth_header(&forged, 5, 31337, &verifier),
            Err(EthHeaderRejection::MultiSigVerificationFailed)
        );
    }
}
