# NEB desktop wallet

A desktop wallet for [NEB](https://github.com/SilasEigen/neb), for Windows, Linux and macOS. It is a window on the NEB node's `neb-wallet` library: the keys, the chain scan, coin selection and signing are the library's.

Status: early. CI builds and tests it on all three systems. Only the Windows build has been used by hand, and no real payment has been sent with it yet.

## What it does

- Creates a key or imports one, and stores it encrypted with a password, in `neb-wallet`'s key file format.
- Shows the balance: spendable coins and coins still maturing.
- Receive: the address as text and as a QR code.
- Send: an address and an amount, a review screen with the fee, then the payment.
- Locks itself after 5 minutes without activity.

## Running it

The wallet reads the chain from a NEB node over JSON-RPC, `http://127.0.0.1:8432` by default (a mainnet node's port). Start a node first: see the [mainnet guide](https://github.com/SilasEigen/neb/blob/main/docs/mainnet.md). You can change the node address in Settings.

## Building

You need a Rust toolchain. On Debian or Ubuntu, also install the window system libraries:

```
sudo apt-get install libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
```

Then:

```
cargo build --release --locked
```

The program is `target/release/neb-wallet-gui`. `build.sh` runs the same build with source paths mapped to fixed prefixes, so the binary does not carry the paths of the machine that built it.

## The key file

`wallet.txt`, in `%APPDATA%\neb-wallet` on Windows, `~/.local/share/neb-wallet` on Linux, `~/Library/Application Support/neb-wallet` on macOS, or in the folder named by `NEB_WALLET_DIR`. Keep a copy of the file and remember the password: nobody can recover either.

## License

Apache-2.0. See [LICENSE](LICENSE).
