// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-attested Ethereum headers on a live swarm (fork only).
//!
//! Four validators observe one stub Ethereum JSON-RPC server. Each validator polls the stub, asks its
//! peers to sign the header, and a quorum certificate goes on chain as `ObservedEthHeader`. The test
//! waits until `ethereum_headers::get_state_root` returns the stub's state root.

use crate::smoke_test_environment::SwarmBuilder;
use anyhow::{anyhow, Result};
use aptos_api_types::ViewRequest;
use aptos_forge::NodeExt;
use aptos_logger::info;
use aptos_rest_client::Client;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use serde_json::{json, Value};
use std::{convert::Infallible, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

const ETH_CHAIN_ID: u64 = 31337;
const BLOCK_NUMBER: u64 = 42;

fn block_hash() -> String {
    format!("0x{}", "ab".repeat(32))
}

fn state_root() -> String {
    format!("0x{}", "cd".repeat(32))
}

/// Answers `eth_chainId` and `eth_getBlockByNumber` like an Ethereum node whose finalized head is
/// `BLOCK_NUMBER`.
fn stub_response(request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(json!(1));
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "eth_chainId" => json!(format!("0x{ETH_CHAIN_ID:x}")),
        "eth_getBlockByNumber" => {
            let tag = request["params"][0].as_str().unwrap_or("");
            if tag == "finalized" || tag == format!("0x{BLOCK_NUMBER:x}") {
                json!({
                    "number": format!("0x{BLOCK_NUMBER:x}"),
                    "hash": block_hash(),
                    "stateRoot": state_root(),
                })
            } else {
                Value::Null
            }
        },
        _ => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "no"}}),
    };
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Starts the stub on a free local port and returns its URL.
async fn spawn_stub_eth_rpc() -> String {
    let make_svc = make_service_fn(|_| async {
        Ok::<_, Infallible>(service_fn(|req: Request<Body>| async move {
            let body = hyper::body::to_bytes(req.into_body())
                .await
                .unwrap_or_default();
            let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            Ok::<_, Infallible>(Response::new(Body::from(
                stub_response(&request).to_string(),
            )))
        }))
    });
    let server = Server::bind(&SocketAddr::from(([127, 0, 0, 1], 0))).serve(make_svc);
    let url = format!("http://{}", server.local_addr());
    tokio::spawn(server);
    url
}

async fn view(client: &Client, function: &str) -> Result<Vec<Value>> {
    let request = ViewRequest {
        function: FromStr::from_str(function).map_err(|e| anyhow!("function id: {e:?}"))?,
        type_arguments: vec![],
        arguments: vec![json!(BLOCK_NUMBER.to_string())],
    };
    Ok(client.view(&request, None).await?.into_inner())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn certified_header_on_live_swarm() {
    let rpc_url = spawn_stub_eth_rpc().await;
    info!("stub Ethereum RPC at {rpc_url}");
    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            config.eth_header_observer.enabled = true;
            config.eth_header_observer.rpc_url = rpc_url.clone();
            config.eth_header_observer.poll_interval_ms = 1_000;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            conf.consensus_config.enable_validator_txns();
            conf.eth_chain_id = Some(ETH_CHAIN_ID);
        }))
        .build()
        .await;
    let client = swarm.validators().next().unwrap().rest_client();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        let certified = view(&client, "0x1::ethereum_headers::is_certified")
            .await
            .map(|v| v[0] == json!(true))
            .unwrap_or(false);
        if certified {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "block {BLOCK_NUMBER} was not certified within 180 seconds"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let root = view(&client, "0x1::ethereum_headers::get_state_root")
        .await
        .unwrap();
    assert_eq!(root[0], json!(state_root()));
    let hash = view(&client, "0x1::ethereum_headers::get_block_hash")
        .await
        .unwrap();
    assert_eq!(hash[0], json!(block_hash()));
    info!("block {BLOCK_NUMBER} certified with the stub state root");
}
