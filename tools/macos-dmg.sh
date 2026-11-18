#!/bin/bash
# Puts a built wallet in a macOS application and the application in a disk image. Runs on macOS only.
# Usage: tools/macos-dmg.sh VERSION PLATFORM PROGRAM OUT_DIR
# The application is signed ad hoc, not with a developer certificate: macOS asks the user to allow it on first start.
set -euo pipefail
version=$1
platform=$2
program=$3
out=$4
app="NEB Wallet.app"
stage=$(mktemp -d)
mkdir -p "$stage/$app/Contents/MacOS" "$out"
cp "$program" "$stage/$app/Contents/MacOS/neb-wallet-gui"
chmod 755 "$stage/$app/Contents/MacOS/neb-wallet-gui"
cat >"$stage/$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>NEB Wallet</string>
  <key>CFBundleDisplayName</key><string>NEB Wallet</string>
  <key>CFBundleIdentifier</key><string>org.neuralenginebyte.wallet</string>
  <key>CFBundleExecutable</key><string>neb-wallet-gui</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
codesign --force --deep --sign - "$stage/$app"
codesign --verify --deep "$stage/$app"
image="$out/neb-wallet-desktop-$version-$platform.dmg"
hdiutil create -volname "NEB Wallet" -srcfolder "$stage" -ov -format UDZO "$image"
rm -rf "$stage"
shasum -a 256 "$image"
