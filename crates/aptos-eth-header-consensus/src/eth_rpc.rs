// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The validator's own view of Ethereum, read through its own JSON-RPC endpoint.

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

/// The header fields a validator attests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthHeader {
    pub number: u64,
    pub hash: [u8; 32],
    pub state_root: [u8; 32],
}

/// A source of Ethereum headers. Production uses [`JsonRpcSource`]. Tests use stubs.
#[async_trait::async_trait]
pub trait EthHeaderSource: Send + Sync {
    /// EIP-155 chain id of the network behind the source.
    async fn chain_id(&self) -> Result<u64>;
    /// The head at the configured block tag (`finalized` by default).
    async fn head(&self) -> Result<EthHeader>;
    /// The header of `number`.
    async fn header_by_number(&self, number: u64) -> Result<EthHeader>;
}

/// Reads headers with `eth_chainId` and `eth_getBlockByNumber`.
pub struct JsonRpcSource {
    client: reqwest::Client,
    url: String,
    block_tag: String,
}

impl JsonRpcSource {
    pub fn new(url: String, block_tag: String, timeout: Duration) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("cannot build the Ethereum RPC client")?;
        Ok(Self {
            client,
            url,
            block_tag,
        })
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let response: Value = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("{method} request failed"))?
            .json()
            .await
            .with_context(|| format!("{method} response is not JSON"))?;
        if let Some(error) = response.get("error") {
            return Err(anyhow!("{method} returned an error: {error}"));
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("{method} response has no result"))
    }

    async fn block(&self, tag: String) -> Result<EthHeader> {
        let block = self
            .call("eth_getBlockByNumber", json!([tag, false]))
            .await?;
        parse_block(&block).with_context(|| format!("block {tag}"))
    }
}

#[async_trait::async_trait]
impl EthHeaderSource for JsonRpcSource {
    async fn chain_id(&self) -> Result<u64> {
        parse_quantity(&self.call("eth_chainId", json!([])).await?)
    }

    async fn head(&self) -> Result<EthHeader> {
        self.block(self.block_tag.clone()).await
    }

    async fn header_by_number(&self, number: u64) -> Result<EthHeader> {
        let header = self.block(format!("0x{number:x}")).await?;
        ensure!(
            header.number == number,
            "asked for block {number}, got {}",
            header.number
        );
        Ok(header)
    }
}

/// Parses the header fields of an `eth_getBlockByNumber` result. A `null` result is an error.
pub fn parse_block(block: &Value) -> Result<EthHeader> {
    ensure!(!block.is_null(), "block not found");
    let field = |name: &str| {
        block
            .get(name)
            .ok_or_else(|| anyhow!("block has no {name}"))
    };
    Ok(EthHeader {
        number: parse_quantity(field("number")?)?,
        hash: parse_hash(field("hash")?)?,
        state_root: parse_hash(field("stateRoot")?)?,
    })
}

fn parse_quantity(value: &Value) -> Result<u64> {
    let s = value
        .as_str()
        .ok_or_else(|| anyhow!("quantity is not a string"))?;
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("quantity has no 0x prefix"))?;
    u64::from_str_radix(digits, 16).context("quantity is not a hex u64")
}

fn parse_hash(value: &Value) -> Result<[u8; 32]> {
    let s = value
        .as_str()
        .ok_or_else(|| anyhow!("hash is not a string"))?;
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("hash has no 0x prefix"))?;
    let mut out = [0u8; 32];
    hex::decode_to_slice(digits, &mut out).context("hash is not 32 hex bytes")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_block() {
        let block = json!({
            "number": "0x1b4",
            "hash": format!("0x{}", "11".repeat(32)),
            "stateRoot": format!("0x{}", "22".repeat(32)),
        });
        assert_eq!(parse_block(&block).unwrap(), EthHeader {
            number: 436,
            hash: [0x11; 32],
            state_root: [0x22; 32],
        });
    }

    #[test]
    fn rejects_malformed_blocks() {
        assert!(parse_block(&Value::Null).is_err());
        let short = json!({"number": "0x1", "hash": "0x11", "stateRoot": "0x22"});
        assert!(parse_block(&short).is_err());
        let no_prefix = json!({
            "number": "1",
            "hash": format!("0x{}", "11".repeat(32)),
            "stateRoot": format!("0x{}", "22".repeat(32)),
        });
        assert!(parse_block(&no_prefix).is_err());
    }
}
