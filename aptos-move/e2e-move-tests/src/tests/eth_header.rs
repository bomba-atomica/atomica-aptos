// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `ObservedEthHeader` validator transaction against the real framework (fork only).
//!
//! The genesis validator set is replaced by four validators whose BLS keys the test holds, so the test can
//! build real quorum certificates. Each rejected certificate must be discarded and leave state unchanged.

use crate::MoveHarness;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    eth_header::{EthHeaderObservation, QuorumCertifiedEthHeader},
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    validator_config::ValidatorConfig,
    validator_info::ValidatorInfo,
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
    validator_verifier::ValidatorVerifier,
};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType, value::MoveValue};
use std::collections::BTreeMap;

const CHAIN: u64 = 31337;
const BLOCK: u64 = 100;

struct Env {
    h: MoveHarness,
    signers: Vec<ValidatorSigner>,
    verifier: ValidatorVerifier,
    epoch: u64,
}

impl Env {
    /// Four validators with voting power 1 each (a quorum is 3), attestation on for `CHAIN`.
    fn new() -> Self {
        let mut env = Self::without_config();
        env.h
            .executor
            .exec("ethereum_header_config", "set", vec![], vec![
                MoveValue::Signer(CORE_CODE_ADDRESS)
                    .simple_serialize()
                    .unwrap(),
                bcs::to_bytes(&CHAIN).unwrap(),
            ]);
        env
    }

    fn without_config() -> Self {
        let mut h = MoveHarness::new();
        let signers: Vec<_> = (0..4u8)
            .map(|i| ValidatorSigner::random([i + 1; 32]))
            .collect();
        let infos: Vec<_> = signers
            .iter()
            .enumerate()
            .map(|(i, s)| {
                ValidatorInfo::new(
                    s.author(),
                    1,
                    ValidatorConfig::new(s.public_key(), vec![], vec![], i as u64),
                )
            })
            .collect();
        let mut validator_set = ValidatorSet::new(infos);
        validator_set.total_voting_power = 4;
        h.executor.write_state_value(
            StateKey::on_chain_config::<ValidatorSet>().unwrap(),
            bcs::to_bytes(&validator_set).unwrap(),
        );
        let verifier = ValidatorVerifier::from(&validator_set);
        let epoch = ConfigurationResource::fetch_config(&h.executor.get_state_view())
            .unwrap()
            .unwrap()
            .epoch();
        Self {
            h,
            signers,
            verifier,
            epoch,
        }
    }

    fn observation(&self, root: u8) -> EthHeaderObservation {
        EthHeaderObservation {
            epoch: self.epoch,
            eth_chain_id: CHAIN,
            block_number: BLOCK,
            block_hash: [7u8; 32],
            state_root: [root; 32],
        }
    }

    /// A certificate claiming `claimed`, signed over `signed` by the first `count` validators.
    fn certify(
        &self,
        count: usize,
        signed: &EthHeaderObservation,
        claimed: EthHeaderObservation,
    ) -> QuorumCertifiedEthHeader {
        let sigs: BTreeMap<_, _> = self.signers[..count]
            .iter()
            .map(|s| (s.author(), s.sign(signed).unwrap()))
            .collect();
        QuorumCertifiedEthHeader {
            observation: claimed,
            multi_sig: self.verifier.aggregate_signatures(sigs.iter()).unwrap(),
        }
    }

    fn quorum(&self, observation: EthHeaderObservation) -> QuorumCertifiedEthHeader {
        self.certify(3, &observation.clone(), observation)
    }

    fn run(&mut self, certified: QuorumCertifiedEthHeader) -> TransactionOutput {
        let txn =
            Transaction::ValidatorTransaction(ValidatorTransaction::ObservedEthHeader(certified));
        let output = self
            .h
            .executor
            .execute_transaction_block(vec![txn])
            .unwrap()
            .pop()
            .unwrap();
        if !output.status().is_discarded() {
            self.h.executor.apply_write_set(output.write_set());
        }
        output
    }

    fn view(&mut self, function: &str, block: u64) -> Option<Vec<u8>> {
        let out = self.h.execute_view_function(
            str::parse(&format!("0x1::ethereum_headers::{function}")).unwrap(),
            vec![],
            vec![bcs::to_bytes(&block).unwrap()],
        );
        out.values.ok().map(|v| v[0].clone())
    }

    fn state_root(&mut self, block: u64) -> Option<Vec<u8>> {
        let certified: bool = bcs::from_bytes(&self.view("is_certified", block).unwrap()).unwrap();
        if !certified {
            return None;
        }
        let bytes = self.view("get_state_root", block).unwrap();
        Some(bcs::from_bytes(&bytes).unwrap())
    }

    fn assert_rejected(&mut self, certified: QuorumCertifiedEthHeader) {
        let before = self.state_root(BLOCK);
        let output = self.run(certified);
        assert!(output.status().is_discarded(), "{:?}", output.status());
        assert!(output.write_set().is_empty());
        assert_eq!(self.state_root(BLOCK), before);
    }
}

#[test]
fn accepts_quorum() {
    let mut env = Env::new();
    let output = env.run(env.quorum(env.observation(9)));
    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    assert_eq!(env.state_root(BLOCK), Some(vec![9u8; 32]));
    let header_certified = TypeTag::Struct(Box::new(
        aptos_types::eth_header::HeaderCertified::struct_tag(),
    ));
    let event = output
        .events()
        .iter()
        .find(|e| e.type_tag() == &header_certified)
        .expect("HeaderCertified event");
    let event: aptos_types::eth_header::HeaderCertified =
        bcs::from_bytes(event.event_data()).unwrap();
    assert_eq!(event.block_number, BLOCK);
    assert_eq!(event.eth_chain_id, CHAIN);
    assert_eq!(event.state_root, vec![9u8; 32]);
}

#[test]
fn rejects_under_quorum() {
    let mut env = Env::new();
    let observation = env.observation(9);
    let certified = env.certify(2, &observation, observation.clone());
    env.assert_rejected(certified);
    assert_eq!(env.state_root(BLOCK), None);
}

#[test]
fn rejects_wrong_epoch() {
    let mut env = Env::new();
    let mut observation = env.observation(9);
    observation.epoch += 1;
    let certified = env.quorum(observation);
    env.assert_rejected(certified);
    assert_eq!(env.state_root(BLOCK), None);
}

#[test]
fn rejects_wrong_chain_id() {
    let mut env = Env::new();
    let mut observation = env.observation(9);
    observation.eth_chain_id = 1;
    let certified = env.quorum(observation);
    env.assert_rejected(certified);
    assert_eq!(env.state_root(BLOCK), None);
}

#[test]
fn rejects_forged_signature() {
    let mut env = Env::new();
    // The quorum signed root 9. The certificate claims root 10.
    let certified = env.certify(4, &env.observation(9), env.observation(10));
    env.assert_rejected(certified);
    assert_eq!(env.state_root(BLOCK), None);
}

#[test]
fn rejects_conflicting_root() {
    let mut env = Env::new();
    env.run(env.quorum(env.observation(9)));
    assert_eq!(env.state_root(BLOCK), Some(vec![9u8; 32]));
    // A valid certificate of another root for the same block, and the same certificate again.
    env.assert_rejected(env.quorum(env.observation(10)));
    env.assert_rejected(env.quorum(env.observation(9)));
    assert_eq!(env.state_root(BLOCK), Some(vec![9u8; 32]));
}

#[test]
fn rejects_while_attestation_is_off() {
    let mut env = Env::without_config();
    env.assert_rejected(env.quorum(env.observation(9)));
    assert_eq!(env.state_root(BLOCK), None);
}
