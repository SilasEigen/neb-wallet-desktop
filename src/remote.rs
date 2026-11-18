//! The node the wallet talks to: a neb-node on this computer or, when there is none, the public node of the project.
//! A node that serves `getutxos` gives the outputs of the wallet in one call per lock; an older node is read block by
//! block. The public node is only trusted for what it shows: the key and the signature stay on this computer, and the
//! network refuses a payment built from a wrong answer.

use std::time::Duration;

use neb_chain::OutPoint;
use neb_json::Value;
use neb_wallet::wallet::{OwnedUtxo, Scan};
use neb_wallet::{RpcClient, Wallet};

use crate::backend::{DEFAULT_NODE, hex32};

/// The public node of the main network, behind TLS.
pub const PUBLIC_NODE: &str = "https://neuralenginebyte.org/rpc";
/// The genesis block id of the main network: the public node must be on this chain.
const MAINNET: &str = "da2f5af7bb4c5e2bcc8d558b2f4f01d4c88557a943e6cff9cd035638fe9439ae";

/// One JSON-RPC call. An `https://` address goes through TLS; any other one uses the plain client of the library.
pub fn call(node: &str, method: &str, params: Vec<Value>) -> Result<Value, String> {
    if !node.starts_with("https://") {
        return RpcClient::new(node)?.call(method, params);
    }
    let request = Value::Object(
        [
            ("jsonrpc".to_string(), Value::String("1.0".into())),
            ("id".to_string(), Value::Number(1)),
            ("method".to_string(), Value::String(method.into())),
            ("params".to_string(), Value::Array(params)),
        ]
        .into(),
    );
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).build();
    let sent = agent.post(node).set("Content-Type", "application/json").send_string(&request.to_string());
    // A refused request comes back as a JSON-RPC error with a 4xx or 5xx status: the body is read either way.
    let response = match sent {
        Ok(response) | Err(ureq::Error::Status(_, response)) => response,
        Err(e) => return Err(format!("connect {node}: {e}")),
    };
    let body = response.into_string().map_err(|e| e.to_string())?;
    let value = neb_json::parse(&body).map_err(|e| e.to_string())?;
    if let Some(error) = value.get("error")
        && !error.is_null()
    {
        let message = error.get("message").and_then(Value::as_str).unwrap_or("rpc error");
        let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
        return Err(format!("{message} (code {code})"));
    }
    value.get("result").cloned().ok_or_else(|| "response has no result".to_string())
}

/// The nodes to try, in order: the saved one, then the public node when the saved one is the default.
fn candidates(saved: &str) -> Vec<&str> {
    if saved == DEFAULT_NODE { vec![DEFAULT_NODE, PUBLIC_NODE] } else { vec![saved] }
}

/// The first node that answers, with the height of its best chain. The public node must be on the main network.
pub fn reach(saved: &str) -> Result<(String, u32), String> {
    let mut last = String::new();
    for node in candidates(saved) {
        let height = match call(node, "getblockcount", vec![]) {
            Ok(value) => value.as_u64().and_then(|h| u32::try_from(h).ok()).ok_or("the block count is not a height")?,
            Err(e) => {
                last = e;
                continue;
            }
        };
        if node == PUBLIC_NODE && call(node, "getblockhash", vec![Value::Number(0)])?.as_str() != Some(MAINNET) {
            return Err("the public node is not on the main network".into());
        }
        return Ok((node.to_string(), height));
    }
    Err(last)
}

/// The unspent outputs of the wallet keys on the best chain of `node`, whose height is `tip`.
pub fn scan(node: &str, tip: u32, wallet: &Wallet) -> Result<Scan, String> {
    let mut utxos = Vec::new();
    for key in &wallet.keys {
        for lock in [key.lock_hex(), key.multisig_lock_hex()] {
            match call(node, "getutxos", vec![Value::String(lock.clone())]) {
                Ok(list) => utxos.extend(owned(&list, &lock, wallet)?),
                // A node older than `getutxos`, on this computer: the library reads the chain block by block.
                Err(e) if e.contains("method not found") && !node.starts_with("https://") => {
                    return neb_wallet::scan(&RpcClient::new(node)?, wallet).map_err(|e| e.to_string());
                }
                Err(e) => return Err(e),
            }
        }
    }
    utxos.sort_by_key(|u| u.outpoint);
    Ok(Scan { utxos, tip_height: tip })
}

/// Reads the answer of `getutxos` for `lock`. An output that no key of the wallet can spend is left out: anybody can
/// create an output with the lock data of somebody else under another lock type.
fn owned(list: &Value, lock: &str, wallet: &Wallet) -> Result<Vec<OwnedUtxo>, String> {
    let bad = || "the node gave an output that cannot be read".to_string();
    let lock_data = hex32(lock).ok_or_else(bad)?;
    let mut utxos = Vec::new();
    for output in list.as_array().ok_or_else(bad)? {
        let int = |name: &str| output.get(name).and_then(Value::as_u64).ok_or_else(bad);
        let lock_type = u8::try_from(int("locktype")?).map_err(|_| bad())?;
        if wallet.key_for_output(lock_type, &lock_data).is_none() {
            continue;
        }
        let txid = output.get("txid").and_then(Value::as_str).and_then(hex32).ok_or_else(bad)?;
        utxos.push(OwnedUtxo {
            outpoint: OutPoint { txid, index: u32::try_from(int("index")?).map_err(|_| bad())? },
            amount: int("amount")?,
            lock_type,
            lock_data,
            height: u32::try_from(int("height")?).map_err(|_| bad())?,
            coinbase: output.get("coinbase").and_then(Value::as_bool).ok_or_else(bad)?,
        });
    }
    Ok(utxos)
}

#[cfg(test)]
mod tests {
    use neb_wallet::WalletKey;

    use super::*;

    fn wallet() -> Wallet {
        let mut wallet = Wallet::empty("unused");
        wallet.keys.push(WalletKey::from_secret([7; 32]));
        wallet
    }

    #[test]
    fn the_public_node_is_tried_only_after_the_default_one() {
        assert_eq!(candidates(DEFAULT_NODE), vec![DEFAULT_NODE, PUBLIC_NODE]);
        assert_eq!(candidates("http://10.0.0.5:8432"), vec!["http://10.0.0.5:8432"]);
    }

    #[test]
    fn owned_keeps_the_outputs_a_key_can_spend() {
        let wallet = wallet();
        let lock = wallet.keys[0].lock_hex();
        let txid = "ab".repeat(32);
        // The second output has the lock data of the key under the multisig lock type: the key cannot spend it.
        let answer = format!(
            r#"[{{"txid":"{txid}","index":1,"amount":2000000000,"locktype":1,"height":7,"coinbase":true}},
                {{"txid":"{txid}","index":2,"amount":5,"locktype":2,"height":8,"coinbase":false}}]"#
        );
        let utxos = owned(&neb_json::parse(&answer).unwrap(), &lock, &wallet).unwrap();
        assert_eq!(utxos.len(), 1);
        let first = &utxos[0];
        assert_eq!((first.outpoint.index, first.amount, first.height, first.coinbase), (1, 2_000_000_000, 7, true));
        assert_eq!(first.outpoint.txid, [0xab; 32]);
        assert!(wallet.keys[0].controls(first.lock_type, &first.lock_data));
    }

    // Needs the Internet: `cargo test -- --ignored`. The public node answers over TLS and is on the main network.
    #[test]
    #[ignore]
    fn the_public_node_answers_over_tls() {
        assert!(call(PUBLIC_NODE, "getblockcount", vec![]).unwrap().as_u64().is_some());
        assert_eq!(call(PUBLIC_NODE, "getblockhash", vec![Value::Number(0)]).unwrap().as_str(), Some(MAINNET));
    }

    // Needs a node on this computer: `cargo test -- --ignored`. A new key holds nothing, on the height of the node.
    #[test]
    #[ignore]
    fn a_new_key_holds_nothing_on_the_local_node() {
        let (node, tip) = reach(DEFAULT_NODE).unwrap();
        assert_eq!(node, DEFAULT_NODE);
        let mut wallet = Wallet::empty("unused");
        wallet.keys.push(WalletKey::generate());
        let scan = scan(&node, tip, &wallet).unwrap();
        assert_eq!((scan.utxos.len(), scan.tip_height), (0, tip));
    }

    #[test]
    fn owned_refuses_an_answer_it_cannot_read() {
        let wallet = wallet();
        let lock = wallet.keys[0].lock_hex();
        let unreadable = [
            r#"{}"#,
            r#"[{"txid":"zz","index":0,"amount":1,"locktype":1,"height":0,"coinbase":true}]"#,
            r#"[{"index":0}]"#,
        ];
        for answer in unreadable {
            assert!(owned(&neb_json::parse(answer).unwrap(), &lock, &wallet).is_err(), "{answer}");
        }
    }
}
