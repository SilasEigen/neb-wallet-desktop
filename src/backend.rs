//! What the window asks of the wallet library: the key file, the balance and a payment. No drawing here.

use std::path::{Path, PathBuf};

use neb_wallet::wallet::{change_lock, recipient_lock};
use neb_wallet::{FEE, RpcClient, Wallet, WalletKey, build_transaction, format_amount, parse_amount, scan, select};

pub const DEFAULT_NODE: &str = "http://127.0.0.1:8432";

/// The folder of the key file and the settings: `NEB_WALLET_DIR` when set, else the data folder of the user.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("NEB_WALLET_DIR") {
        return PathBuf::from(dir);
    }
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join("neb-wallet")
}

/// The path of the key file as the window shows it: the folder of the user is written `~`, so that a picture of the
/// window does not carry the name of the account.
pub fn shown_wallet_path() -> String {
    let path = wallet_path();
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    match home.and_then(|h| path.strip_prefix(h).ok().map(Path::to_path_buf)) {
        Some(rest) => Path::new("~").join(rest).display().to_string(),
        None => path.display().to_string(),
    }
}

pub fn wallet_path() -> PathBuf {
    data_dir().join("wallet.txt")
}

fn settings_path() -> PathBuf {
    data_dir().join("node.txt")
}

/// The JSON-RPC address of the node, as saved by the user.
pub fn load_node() -> String {
    std::fs::read_to_string(settings_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_NODE.to_string())
}

pub fn save_node(url: &str) -> Result<(), String> {
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), url.trim()).map_err(|e| e.to_string())
}

/// Creates the key file with one new key, encrypted with `password`.
pub fn create(path: &Path, password: &str) -> Result<Wallet, String> {
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    Wallet::create_encrypted(path, password).map_err(|e| e.to_string())
}

/// Creates the key file from a 64-hex secret, encrypted with `password`.
pub fn import(path: &Path, secret: &str, password: &str) -> Result<Wallet, String> {
    let key = WalletKey::from_text(secret.trim())?;
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let mut wallet = Wallet::empty(path.display().to_string());
    wallet.keys.push(key);
    wallet.encrypt(password).map_err(|e| e.to_string())?;
    Ok(wallet)
}

/// Opens the key file. A wrong password is an error.
pub fn unlock(path: &Path, password: &str) -> Result<Wallet, String> {
    let wallet = Wallet::load_with_password(path, password).map_err(|e| e.to_string())?;
    if wallet.keys.is_empty() {
        return Err("the wallet has no keys".into());
    }
    Ok(wallet)
}

pub fn address(wallet: &Wallet) -> String {
    wallet.keys.first().map(|k| k.address.clone()).unwrap_or_default()
}

/// What the node says of the coins of the wallet, as texts ready to show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Balance {
    pub height: u32,
    pub spendable: String,
    /// Coinbase outputs younger than 100 blocks; empty when there are none.
    pub maturing: String,
}

/// Reads the whole chain through the node: one call per block.
pub fn balance(wallet: &Wallet, node: &str) -> Result<Balance, String> {
    let client = RpcClient::new(node)?;
    let scan = scan(&client, wallet).map_err(|e| e.to_string())?;
    let (mature, all) = (scan.mature_balance(), scan.balance());
    Ok(Balance {
        height: scan.tip_height,
        spendable: format_amount(mature),
        maturing: if all > mature { format_amount(all - mature) } else { String::new() },
    })
}

pub fn fee() -> String {
    format_amount(FEE)
}

/// True when `text` is an address of the network.
pub fn is_address(text: &str) -> bool {
    recipient_lock(text.trim()).is_ok()
}

/// True when `text` is an amount above zero.
pub fn is_amount(text: &str) -> bool {
    parse_amount(text.trim()).is_ok_and(|a| a > 0)
}

/// Signs and submits a payment; returns the transaction id. The steps are those of `neb-wallet send`.
pub fn send(wallet: &Wallet, node: &str, to: &str, amount: &str) -> Result<String, String> {
    let amount = parse_amount(amount.trim()).map_err(|e| e.to_string())?;
    let recipient = recipient_lock(to.trim()).map_err(|e| e.to_string())?;
    let change = change_lock(wallet).map_err(|e| e.to_string())?;
    let client = RpcClient::new(node)?;
    let scan = scan(&client, wallet).map_err(|e| e.to_string())?;
    let selection = select(&scan.mature(), amount, FEE).map_err(|e| e.to_string())?;
    // The chain id is the genesis block id (spec/node.md section 5.2).
    let chain_id = hex32(&client.block_hash(0)?).ok_or("bad genesis hash")?;
    let tx = build_transaction(wallet, &selection, recipient, &chain_id, change).map_err(|e| e.to_string())?;
    let hex: String = tx.encode().iter().map(|b| format!("{b:02x}")).collect();
    client.send_raw_transaction(&hex)
}

fn hex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("neb-wallet-desktop-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temporary folder");
        dir.join("wallet.txt")
    }

    #[test]
    fn create_then_unlock_gives_the_same_address() {
        let path = temp("create");
        let made = Wallet::create_encrypted(&path, "a password").expect("a new wallet");
        let opened = unlock(&path, "a password").expect("the right password");
        assert_eq!(address(&made), address(&opened));
        assert!(is_address(&address(&opened)));
        assert!(Wallet::is_encrypted(&path).expect("a readable file"));
    }

    #[test]
    fn unlock_refuses_a_wrong_password() {
        let path = temp("wrong");
        Wallet::create_encrypted(&path, "a password").expect("a new wallet");
        assert!(unlock(&path, "another").is_err());
    }

    #[test]
    fn import_keeps_the_key() {
        let (first, second) = (temp("import-a"), temp("import-b"));
        let made = Wallet::create_encrypted(&first, "one").expect("a new wallet");
        let secret = made.keys[0].secret_hex();
        let mut copy = Wallet::empty(second.display().to_string());
        copy.keys.push(WalletKey::from_text(&secret).expect("a valid secret"));
        copy.encrypt("two").expect("a writable file");
        assert_eq!(address(&unlock(&second, "two").expect("the right password")), address(&made));
    }

    #[test]
    fn amounts_and_addresses_are_checked() {
        assert!(is_amount("1.5"));
        assert!(!is_amount("0"));
        assert!(!is_amount("abc"));
        assert!(!is_address("neb1bad"));
    }
}
