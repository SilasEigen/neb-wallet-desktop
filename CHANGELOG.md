# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [0.1.0] - unreleased

First release, for Windows and Linux.

- Create a key or import one. The key file is encrypted with a password, in `neb-wallet`'s format.
- Accounts: one per key in the key file. Click the brand to switch accounts, import a key or create one.
- Balance: spendable coins and coins still maturing.
- Receive with the address as text and as a QR code. Send with a review screen that shows the fee.
- The wallet uses a NEB node on the same computer, or the project's public node over HTTPS when none answers. It
  checks that the public node is on the main network.
- The window locks after 5 minutes without activity.

Not in this release: a macOS build, an installer, and executables signed with a vendor certificate (Windows warns
about an unknown publisher).
