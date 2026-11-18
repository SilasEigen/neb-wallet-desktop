# NEB desktop wallet

A desktop wallet for [NEB](https://github.com/SilasEigen/neb), for Windows, Linux and macOS. It is a window on the NEB node's `neb-wallet` library: the keys, the chain scan, coin selection and signing are the library's.

Status: early. CI builds and tests it on Windows and Linux; the macOS job has not run yet. Only the Windows build has been used by hand, and no real payment has been sent with it yet.

## What it does

- Creates a key or imports one, and stores it encrypted with a password, in `neb-wallet`'s key file format.
- Accounts: one per key in the key file. Click the brand to switch accounts, import a key or create one.
- Shows the balance: spendable coins and coins still maturing.
- Receive: the address as text and as a QR code.
- Send: an address and an amount, a review screen with the fee, then the payment.
- Locks itself after 5 minutes without activity.

## Running it

The wallet reads the chain from a NEB node over JSON-RPC. It first tries a node on the same computer, `http://127.0.0.1:8432` (a mainnet node's port). When none answers, it asks the project's public node over HTTPS. The public node only reports balances and relays signed payments: the key never leaves your computer.

To run your own node, see the [mainnet guide](https://github.com/SilasEigen/neb/blob/main/docs/mainnet.md). You can set another node address in Settings.

## Privacy

With your own node, the wallet talks to nothing else. With the public node, its operator can see which IP address asks about which NEB address. Run your own node to avoid that.

## Verifying a download

Each release has a `SHA256SUMS` file signed with the NEB release key, the one that signs the node releases (fingerprint `EBF3 A9A0 37BC D121 0464 2A30 D7FC 5241 9E25 BBC4`):

```
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c --ignore-missing SHA256SUMS
```

Windows warns about an unknown publisher because the executable carries no vendor certificate.

## Building

You need a Rust toolchain. On Debian or Ubuntu, also install the window system libraries:

```
sudo apt-get install libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
```

Then:

```
cargo build --release --locked
```

The program is `target/release/neb-wallet-gui`. `build.sh` runs the same build with source paths mapped to fixed prefixes, so the binary does not carry the paths of the machine that built it. `tools/pack.py` puts a built program and the documents of the repository in a release archive.

## The key file

`wallet.txt`, in `%APPDATA%\neb-wallet` on Windows, `~/.local/share/neb-wallet` on Linux, `~/Library/Application Support/neb-wallet` on macOS, or in the folder named by `NEB_WALLET_DIR`. Keep a copy of the file and remember the password: nobody can recover either.

## License

Apache-2.0. See [LICENSE](LICENSE).
