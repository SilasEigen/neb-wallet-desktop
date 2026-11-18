#!/bin/bash
# Release build of the desktop wallet: target/release/neb-wallet-gui. Source paths are mapped to fixed prefixes, as the
# release build of the NEB node does.
set -euo pipefail
unset MSYS_NO_PATHCONV  # Git Bash must convert the paths given to the Windows tools
here=$(cd "$(dirname "$0")" && pwd)
native() { if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi; }
flags=""
for pair in "$HOME=/home" "${RUSTUP_HOME:-$HOME/.rustup}=/rustup" "${CARGO_HOME:-$HOME/.cargo}=/cargo" "$here=/wallet"; do
  flags="$flags --remap-path-prefix=$(native "${pair%=*}")=${pair##*=}"
done
case "$(rustc -vV | sed -n 's/^host: //p')" in *windows-msvc) flags="$flags -C target-feature=+crt-static" ;; esac
cd "$here"
RUSTFLAGS="$flags" cargo build --release --locked
ls -l "${CARGO_TARGET_DIR:-target}"/release/neb-wallet-gui*
