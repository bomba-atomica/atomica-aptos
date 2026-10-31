// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end run of the hardened Boneh-Franklin timelock on a live local swarm.
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md`. Evidence and defects:
//! `atomica/docs/decisions/e2e-timelock-smoke-test.md`.
//!
//! The swarm has four validators. The mempool chunky DKG is on from genesis. The test turns the timelock
//! chunky DKG on through a governance script (`timelock_dkg::enable`) and forces an epoch change. Then it:
//!
//! 1. registers a timelock and encrypts three bids to the timelock's own master public key (client side,
//!    `aptos_dkg::ibe::ciphertext::encrypt_bid`),
//! 2. makes one validator submit a corrupted share (failpoint `timelock_share::corrupt`),
//! 3. watches that no share is accepted before the deadline and that the reveal completes after it,
//! 4. decrypts the bids with the revealed key and tries a copied bid,
//! 5. registers a second timelock, forces an epoch change before its deadline and checks it expires
//!    with the reason `epoch-ended`,
//! 6. runs encrypted mempool traffic before and after, and compares the two master public keys.
//!
//! Every assertion is recorded with `check` and the run prints one `E2E-ASSERT` line per assertion. The test
//! continues after a failed assertion where it can, and fails at the end.

use crate::{
    smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic,
    utils::get_on_chain_resource,
};
use anyhow::{anyhow, Result};
use aptos::test::CliTestFramework;
use aptos_api_types::ViewRequest;
use aptos_dkg::ibe::{
    ciphertext::{decrypt_bid, encrypt_bid, BidCiphertext},
    decode_g1, decode_g2, verify_decryption_key, IDENTITY_LEN,
};
use aptos_forge::{EmitJobMode, LocalSwarm, Node, NodeExt, Swarm, SwarmExt, TransactionType};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::types::LocalAccount;
use aptos_types::{
    decryption::PerEpochEncryptionKeyResource,
    dkg::timelock_dkg::TimelockEpochConfigResource,
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    move_resource::MoveStructType,
};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::{json, Value};
use std::{str::FromStr, sync::Arc, time::Duration};

/// 30 minute epochs. Registration needs `deadline <= epoch_end - 15 minutes`.
const EPOCH_DURATION_SECS: u64 = 1800;
/// How far ahead of the chain clock the deadline of the first timelock is set.
const DEADLINE_AHEAD_US: u64 = 75_000_000;
const AUCTION_ID: u64 = 5;

#[derive(Default)]
struct Results {
    lines: Vec<(String, bool, String)>,
}

impl Results {
    fn check(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        let detail = detail.into();
        let line = format!(
            "E2E-ASSERT {name}: {} ({detail})",
            if ok { "PASS" } else { "FAIL" }
        );
        println!("{line}");
        info!("{line}");
        self.lines.push((name.to_string(), ok, detail));
    }

    fn finish(self) {
        println!("E2E-SUMMARY");
        for (name, ok, detail) in &self.lines {
            println!("  {} {name}: {detail}", if *ok { "PASS" } else { "FAIL" });
        }
        let failed: Vec<_> = self.lines.iter().filter(|l| !l.1).map(|l| &l.0).collect();
        assert!(failed.is_empty(), "failed assertions: {failed:?}");
    }
}

async fn view(client: &Client, function: &str, args: Vec<Value>) -> Result<Vec<Value>> {
    let request = ViewRequest {
        function: FromStr::from_str(function).map_err(|e| anyhow!("function id: {e:?}"))?,
        type_arguments: vec![],
        arguments: args,
    };
    Ok(client.view(&request, None).await?.into_inner())
}

fn as_u64(v: &Value) -> u64 {
    match v {
        Value::String(s) => s.parse().expect("u64 string"),
        Value::Number(n) => n.as_u64().expect("u64 number"),
        other => panic!("not an integer: {other}"),
    }
}

fn as_bytes(v: &Value) -> Vec<u8> {
    hex::decode(v.as_str().expect("hex string").trim_start_matches("0x")).expect("hex")
}

async fn chain_now_us(client: &Client) -> u64 {
    as_u64(
        &view(client, "0x1::timestamp::now_microseconds", vec![])
            .await
            .unwrap()[0],
    )
}

async fn timelock_status(client: &Client, id: u64) -> Result<(u8, u8)> {
    let status = as_u64(
        &view(
            client,
            "0x1::ibe_config::get_status",
            vec![json!(id.to_string())],
        )
        .await?[0],
    );
    let reason = as_u64(
        &view(
            client,
            "0x1::ibe_config::get_expiry_reason",
            vec![json!(id.to_string())],
        )
        .await?[0],
    );
    Ok((status as u8, reason as u8))
}

/// `(deadline_us, identity, revealed, share_count)`.
async fn timelock_row(client: &Client, id: u64) -> Result<(u64, Vec<u8>, bool, u64)> {
    let v = view(
        client,
        "0x1::ibe_config::get_timelock",
        vec![json!(id.to_string())],
    )
    .await?;
    Ok((
        as_u64(&v[0]),
        as_bytes(&v[1]),
        v[2].as_bool().expect("bool"),
        as_u64(&v[3]),
    ))
}

async fn register_timelock(
    client: &Client,
    chain_id: aptos_types::chain_id::ChainId,
    account: &mut LocalAccount,
    deadline_us: u64,
) -> Result<u64> {
    let before = as_u64(&view(client, "0x1::ibe_config::get_next_timelock_id", vec![]).await?[0]);
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(AccountAddress::ONE, Identifier::new("ibe_config").unwrap()),
        Identifier::new("register_timelock").unwrap(),
        vec![],
        vec![bcs::to_bytes(&deadline_us)?],
    ));
    let txn = account.sign_with_transaction_builder(
        aptos_sdk::transaction_builder::TransactionFactory::new(chain_id)
            .with_gas_unit_price(100)
            .with_max_gas_amount(200_000)
            .payload(payload),
    );
    client.submit_and_wait(&txn).await?;
    Ok(before)
}

/// `BCS(tag, chain_id, epoch, timelock_id, auction_id, bidder, deadline_us)`, the associated data the
/// auction contract builds on chain.
fn bid_ad(
    chain_id: u8,
    epoch: u64,
    timelock_id: u64,
    bidder: &AccountAddress,
    deadline_us: u64,
) -> Vec<u8> {
    bcs::to_bytes(&(
        "atomica/timelock-bid/v2",
        chain_id,
        epoch,
        timelock_id,
        AUCTION_ID,
        bidder.into_bytes(),
        deadline_us,
    ))
    .unwrap()
}

fn plaintext(price: u64, salt: u8) -> Vec<u8> {
    let mut p = price.to_le_bytes().to_vec();
    p.extend_from_slice(&[salt; 32]);
    p
}

/// All `TimelockShareRejectedEvent`s on chain: `(timelock_id, epoch, validator)`.
async fn rejected_share_events(client: &Client) -> Vec<(u64, u64, AccountAddress)> {
    let mut out = vec![];
    let mut start = 0u64;
    let last = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    while start <= last {
        let txns = client
            .get_transactions_bcs(Some(start), Some(100))
            .await
            .unwrap()
            .into_inner();
        if txns.is_empty() {
            break;
        }
        start += txns.len() as u64;
        for txn in &txns {
            for event in &txn.events {
                if event
                    .type_tag()
                    .to_canonical_string()
                    .ends_with("::ibe_config::TimelockShareRejectedEvent")
                {
                    let (timelock_id, epoch, validator, _ts): (u64, u64, AccountAddress, u64) =
                        bcs::from_bytes(event.event_data()).expect("rejected event");
                    out.push((timelock_id, epoch, validator));
                }
            }
        }
    }
    out
}

async fn emit_encrypted_traffic(swarm: &mut impl Swarm, secs: u64) -> u64 {
    let all: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        swarm,
        &all,
        Duration::from_secs(secs),
        200,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .expect("encrypted traffic");
    info!(
        "emitter: submitted={} committed={}",
        stats.submitted, stats.committed
    );
    stats.committed
}

const ENABLE_TIMELOCK_DKG_SCRIPT: &str = r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::timelock_dkg;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        timelock_dkg::enable(&framework_signer, 300);
        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;

/// Four validators, the mempool chunky DKG on from genesis and the timelock DKG off.
async fn start_network() -> (
    LocalSwarm,
    CliTestFramework,
    tokio::task::JoinHandle<Result<()>>,
) {
    SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = EPOCH_DURATION_SECS;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await
}

/// Waits for epoch 2, turns the timelock DKG on and waits for epoch 3, the first epoch with a timelock key.
async fn enable_timelock_dkg(swarm: &mut LocalSwarm, cli: &mut CliTestFramework) -> usize {
    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(300))
        .await
        .expect("epoch 2");
    info!("epoch 2 reached, the timelock DKG is still off");
    cli.run_script(root_idx, ENABLE_TIMELOCK_DKG_SCRIPT)
        .await
        .expect("enable timelock dkg");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(600))
        .await
        .expect("epoch 3 (timelock DKG epoch)");
    root_idx
}

/// Keeps a network with a live timelock key running for `atomica/scripts/timelock-smoke-test.sh`. Skipped
/// unless `TIMELOCK_TESTNET_INFO` names the file to write the connection variables to. The network stops
/// when the file `<info file>.stop` appears (or after 40 minutes).
#[tokio::test]
async fn timelock_testnet_for_script() {
    let Ok(info_path) = std::env::var("TIMELOCK_TESTNET_INFO") else {
        return;
    };
    let (mut swarm, mut cli, _faucet) = start_network().await;
    enable_timelock_dkg(&mut swarm, &mut cli).await;
    // The governance script advanced the root account's sequence number behind the harness's back.
    swarm
        .aptos_public_info()
        .sync_root_account_sequence_number()
        .await;
    let mut accounts = vec![];
    for _ in 0..4 {
        accounts.push(
            swarm
                .aptos_public_info()
                .create_and_fund_user_account(1_000_000_000)
                .await
                .expect("account"),
        );
    }
    let key_hex = |a: &LocalAccount| hex::encode(a.private_key().to_bytes());
    let vars = json!({
        "REST_URL": swarm.validators().next().unwrap().rest_api_endpoint().to_string(),
        "REGISTRAR_KEY": key_hex(&accounts[0]),
        "BIDDER_KEYS": accounts[1..].iter().map(key_hex).collect::<Vec<_>>().join(","),
        "ROOT_KEY": hex::encode(swarm.root_key().to_bytes()),
        "VALIDATOR_REST_URLS": swarm.validators().map(|v| v.rest_api_endpoint().to_string()).collect::<Vec<_>>(),
    });
    std::fs::write(&info_path, serde_json::to_string_pretty(&vars).unwrap()).unwrap();
    println!("TIMELOCK-TESTNET-READY {info_path}");
    let stop = format!("{info_path}.stop");
    for _ in 0..(40 * 60) {
        if std::path::Path::new(&stop).exists() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::test]
async fn timelock_e2e() {
    let mut results = Results::default();
    let (mut swarm, mut cli, _faucet) = start_network().await;
    let root_idx = enable_timelock_dkg(&mut swarm, &mut cli).await;

    let chain_id = swarm.chain_id();
    let client = swarm.validators().next().unwrap().rest_client();
    let validator_addrs: Vec<AccountAddress> = swarm.validators().map(|v| v.peer_id()).collect();

    // The config of epoch 3 exists once the timelock DKG finished in the reconfiguration.
    let epoch = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    let ready = view(&client, "0x1::ibe_config::is_ready", vec![])
        .await
        .unwrap()[0]
        .as_bool()
        .unwrap();
    results.check(
        "timelock-dkg-completes-in-live-network",
        ready,
        format!("ibe_config::is_ready={ready} at epoch {epoch}"),
    );
    if !ready {
        results.finish();
        return;
    }
    let config: TimelockEpochConfigResource = get_on_chain_resource(&client).await;
    let mpk_bytes = config.mpk.clone();
    let mempool_key: PerEpochEncryptionKeyResource = {
        client
            .get_account_resource_bcs::<PerEpochEncryptionKeyResource>(
                AccountAddress::ONE,
                &PerEpochEncryptionKeyResource::struct_tag().to_canonical_string(),
            )
            .await
            .expect("mempool key")
            .into_inner()
    };
    let mempool_mpk = mempool_key.encryption_key.clone().unwrap_or_default();
    results.check(
        "mempool-mpk-differs-from-timelock-mpk",
        !mempool_mpk.is_empty() && mempool_mpk != mpk_bytes,
        format!(
            "timelock mpk {} bytes, mempool key {} bytes, epoch {}",
            mpk_bytes.len(),
            mempool_mpk.len(),
            mempool_key.epoch
        ),
    );
    info!(
        "timelock config: threshold weight {} of {}, weights {:?}",
        config.reconstruct_threshold_weight, config.total_weight, config.weights
    );

    // Mempool still works in the same network (epoch with a live timelock key).
    let committed = emit_encrypted_traffic(&mut swarm, 15).await;
    results.check(
        "mempool-encrypted-txns-commit-with-timelock-enabled",
        committed > 0,
        format!("{committed} encrypted transactions committed"),
    );

    // Phase 2: a validator that misbehaves, three bidders and the first timelock. The honest validators are
    // held back until the corrupted share was seen on chain, so the bad share arrives first.
    let bad_validator = validator_addrs[3];
    let rest_of = |i: usize| swarm.validators().nth(i).unwrap().rest_client();
    rest_of(3)
        .set_failpoint("timelock_share::corrupt".to_string(), "return".to_string())
        .await
        .expect("set corrupt failpoint on validator 3");
    for i in 0..3 {
        rest_of(i)
            .set_failpoint("timelock_share::hold".to_string(), "return".to_string())
            .await
            .expect("set hold failpoint");
    }

    let mut bidders = vec![];
    for _ in 0..3 {
        bidders.push(
            swarm
                .aptos_public_info()
                .create_and_fund_user_account(1_000_000_000)
                .await
                .expect("bidder"),
        );
    }
    let mut registrar = swarm
        .aptos_public_info()
        .create_and_fund_user_account(1_000_000_000)
        .await
        .expect("registrar");

    let now = chain_now_us(&client).await;
    let deadline_us = now + DEADLINE_AHEAD_US;
    let t1 = register_timelock(&client, chain_id, &mut registrar, deadline_us)
        .await
        .expect("register timelock 1");
    let (row_deadline, identity_bytes, _, _) = timelock_row(&client, t1).await.unwrap();
    let t1_epoch = as_u64(
        &view(
            &client,
            "0x1::ibe_config::get_timelock_epoch",
            vec![json!(t1.to_string())],
        )
        .await
        .unwrap()[0],
    );
    let identity: [u8; IDENTITY_LEN] = identity_bytes.clone().try_into().expect("identity length");
    results.check(
        "timelock-registered-with-epoch-bound-identity",
        row_deadline == deadline_us
            && identity
                == aptos_dkg::ibe::compute_identity(chain_id.id(), t1_epoch, t1, deadline_us),
        format!("timelock {t1} epoch {t1_epoch} deadline {deadline_us}"),
    );

    // Client side V2 encryption to the timelock's own mpk.
    let mpk = decode_g2(&mpk_bytes).expect("timelock mpk is a valid G2 point");
    let mut rng = StdRng::seed_from_u64(0xA70A);
    let prices = [1_500u64, 2_750, 900];
    let mut cts = vec![];
    let mut ads = vec![];
    for (i, bidder) in bidders.iter().enumerate() {
        let ad = bid_ad(chain_id.id(), t1_epoch, t1, &bidder.address(), deadline_us);
        let ct = encrypt_bid(
            &mpk,
            &identity,
            &ad,
            &plaintext(prices[i], i as u8 + 1),
            &mut rng,
        )
        .expect("encrypt bid");
        // The submission check of the contract (`ibe::verify_bid_ct`).
        ct.verify(&ad).expect("fresh bid verifies");
        cts.push(ct);
        ads.push(ad);
    }

    // Phase 3: sealed until the deadline. Poll (now, share_count) while nothing is released.
    let mut sealed_ok = true;
    let mut samples = 0;
    let mut released = false;
    let mut revealed = false;
    let mut rejection_seen_before_release = false;
    let mut count_at_rejection = u64::MAX;
    let wait_start = std::time::Instant::now();
    while wait_start.elapsed() < Duration::from_secs(480) {
        let now = chain_now_us(&client).await;
        let (_, _, is_revealed, share_count) = timelock_row(&client, t1).await.unwrap();
        samples += 1;
        if now < deadline_us {
            if share_count > 0 || is_revealed {
                sealed_ok = false;
            }
            if view(
                &client,
                "0x1::ibe_config::get_decryption_key",
                vec![json!(t1.to_string())],
            )
            .await
            .is_ok()
            {
                sealed_ok = false;
            }
        }
        if is_revealed {
            revealed = true;
            break;
        }
        if !released && now >= deadline_us {
            // After the deadline only the corrupted share can reach the chain, and it must be rejected.
            let rejected = rejected_share_events(&client).await;
            if rejected
                .iter()
                .any(|(id, _, v)| *id == t1 && *v == bad_validator)
            {
                rejection_seen_before_release = true;
                count_at_rejection = share_count;
                for i in 0..3 {
                    rest_of(i)
                        .set_failpoint("timelock_share::hold".to_string(), "off".to_string())
                        .await
                        .expect("release hold");
                }
                released = true;
                info!("rejection of validator 3 seen on chain, honest validators released");
            }
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
    results.check(
        "sealed-until-deadline",
        sealed_ok && samples > 3,
        format!(
            "{samples} samples, no share and no key before the committed deadline {deadline_us}"
        ),
    );
    results.check(
        "corrupted-share-is-rejected-and-takes-no-weight",
        rejection_seen_before_release && count_at_rejection == 0,
        format!(
            "rejection event for validator 3 seen after the deadline while no honest share was released: {rejection_seen_before_release}, stored share weight at that time {count_at_rejection}"
        ),
    );
    results.check(
        "reveal-completes-after-deadline-with-honest-threshold-shares",
        revealed && released,
        format!(
            "revealed={revealed} (threshold weight {} of {}, validator 3 stayed corrupt)",
            config.reconstruct_threshold_weight, config.total_weight
        ),
    );

    if revealed {
        let dk_bytes = as_bytes(
            &view(
                &client,
                "0x1::ibe_config::get_decryption_key",
                vec![json!(t1.to_string())],
            )
            .await
            .unwrap()[0],
        );
        let dk = decode_g1(&dk_bytes).expect("revealed key is a valid G1 point");
        results.check(
            "revealed-key-verifies-against-timelock-mpk",
            verify_decryption_key(&dk, &identity, &mpk),
            "e(dk, g2) = e(H1(id), mpk)",
        );
        let mut opened = vec![];
        for i in 0..3 {
            opened.push(decrypt_bid(&dk, &identity, &ads[i], &cts[i]).ok());
        }
        let all_open = (0..3)
            .all(|i| opened[i].as_deref() == Some(plaintext(prices[i], i as u8 + 1).as_slice()));
        results.check(
            "bids-decrypt-with-the-revealed-key",
            all_open,
            format!(
                "opened {:?}",
                opened.iter().map(|o| o.is_some()).collect::<Vec<_>>()
            ),
        );

        // A bid copied by another account: bidder 1 submits bidder 0's ciphertext under its own AD.
        let copy_verify = cts[0].verify(&ads[1]);
        let copy_decrypt = decrypt_bid(&dk, &identity, &ads[1], &cts[0]);
        let others_ok = decrypt_bid(&dk, &identity, &ads[0], &cts[0]).is_ok()
            && decrypt_bid(&dk, &identity, &ads[2], &cts[2]).is_ok();
        results.check(
            "copied-bid-fails-and-others-decrypt",
            copy_verify.is_err() && copy_decrypt.is_err() && others_ok,
            format!(
                "copy verify {:?}, copy decrypt {:?}, original and third bid decrypt={others_ok}",
                copy_verify.err(),
                copy_decrypt.err()
            ),
        );
        let wire = cts[0].to_bytes();
        results.check(
            "bid-ciphertext-wire-format-roundtrip",
            BidCiphertext::from_bytes(&wire)
                .map(|c| c == cts[0])
                .unwrap_or(false),
            format!("{} bytes", wire.len()),
        );
    }

    // Phase 4: forced epoch change before the reveal (governance `force_end_epoch`, which ends the epoch at
    // once and runs no DKG). Deadline well after the forced change.
    let now = chain_now_us(&client).await;
    let t2_deadline = now + 600_000_000;
    let t2 = register_timelock(&client, chain_id, &mut registrar, t2_deadline)
        .await
        .expect("register timelock 2");
    let before = timelock_status(&client, t2).await.unwrap();
    swarm.aptos_public_info().reconfig().await;
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(600))
        .await
        .expect("epoch 4");
    let after = timelock_status(&client, t2).await.unwrap();
    let key_after = view(
        &client,
        "0x1::ibe_config::get_decryption_key",
        vec![json!(t2.to_string())],
    )
    .await;
    results.check(
        "epoch-change-before-reveal-expires-timelock-epoch-ended",
        before == (0, 0) && after == (2, 1) && key_after.is_err(),
        format!("status before {before:?}, after {after:?} (2,1 = expired, epoch-ended), key readable={}", key_after.is_ok()),
    );
    let t1_after = timelock_status(&client, t1).await.unwrap();
    results.check(
        "revealed-timelock-stays-revealed-across-epochs",
        t1_after == (1, 0),
        format!("status {t1_after:?}"),
    );
    let ready_after_force = view(&client, "0x1::ibe_config::is_ready", vec![])
        .await
        .unwrap()[0]
        .as_bool()
        .unwrap();
    results.check(
        "forced-epoch-end-leaves-no-timelock-key-by-design",
        !ready_after_force,
        format!("is_ready={ready_after_force} in epoch 4 (force_end_epoch runs no DKG)"),
    );

    // Phase 5: a DKG driven epoch change (governance `reconfigure`) gives a fresh, unrelated timelock key.
    let reconfigure = r#"
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;
    cli.run_script(root_idx, reconfigure)
        .await
        .expect("reconfigure with DKG");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(600))
        .await
        .expect("epoch 5");
    let ready5 = view(&client, "0x1::ibe_config::is_ready", vec![])
        .await
        .unwrap()[0]
        .as_bool()
        .unwrap();
    let config5: TimelockEpochConfigResource = get_on_chain_resource(&client).await;
    info!(
        "epoch 5 timelock config: threshold weight {} of {}, weights {:?}",
        config5.reconstruct_threshold_weight, config5.total_weight, config5.weights
    );
    // The corrupted validator behaves again, so the second reveal does not depend on which validator got
    // which weight in the new DKG (a corrupted validator with a larger weight can block a reveal).
    rest_of(3)
        .set_failpoint("timelock_share::corrupt".to_string(), "off".to_string())
        .await
        .expect("clear corrupt failpoint");
    results.check(
        "second-timelock-dkg-completes-in-a-later-epoch",
        ready5 && config5.epoch == 5,
        format!(
            "is_ready={ready5}, config epoch {}, threshold weight {} of {}, weights {:?}",
            config5.epoch,
            config5.reconstruct_threshold_weight,
            config5.total_weight,
            config5.weights
        ),
    );
    if ready5 && revealed {
        let dk_bytes = as_bytes(
            &view(
                &client,
                "0x1::ibe_config::get_decryption_key",
                vec![json!(t1.to_string())],
            )
            .await
            .unwrap()[0],
        );
        let dk = decode_g1(&dk_bytes).unwrap();
        let mpk5 = decode_g2(&config5.mpk).expect("epoch 5 mpk");
        results.check(
            "timelock-secrets-of-two-epochs-are-unrelated",
            config5.mpk != config.mpk && !verify_decryption_key(&dk, &identity, &mpk5),
            "the epoch 5 mpk differs from the epoch 3 mpk and the epoch 3 key fails under it",
        );
        // A full reveal under the second secret.
        let now = chain_now_us(&client).await;
        let t3_deadline = now + 45_000_000;
        let t3 = register_timelock(&client, chain_id, &mut registrar, t3_deadline)
            .await
            .expect("register timelock 3");
        let (_, id3_bytes, _, _) = timelock_row(&client, t3).await.unwrap();
        let id3: [u8; IDENTITY_LEN] = id3_bytes.try_into().unwrap();
        let mut revealed3 = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(300) {
            if timelock_row(&client, t3).await.unwrap().2 {
                revealed3 = true;
                break;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let ok3 = revealed3 && {
            let dk3 = decode_g1(&as_bytes(
                &view(
                    &client,
                    "0x1::ibe_config::get_decryption_key",
                    vec![json!(t3.to_string())],
                )
                .await
                .unwrap()[0],
            ))
            .unwrap();
            verify_decryption_key(&dk3, &id3, &mpk5) && !verify_decryption_key(&dk3, &id3, &mpk)
        };
        results.check(
            "reveal-works-under-the-second-epoch-secret",
            ok3,
            format!("timelock {t3} revealed={revealed3}, key verifies under the epoch 5 mpk and not under the epoch 3 mpk"),
        );
    }

    // The mempool still works after two timelock DKGs.
    let committed = emit_encrypted_traffic(&mut swarm, 15).await;
    results.check(
        "mempool-encrypted-txns-commit-after-second-timelock-dkg",
        committed > 0,
        format!("{committed} encrypted transactions committed"),
    );

    results.finish();
}
