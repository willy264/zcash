# Vendored lightwallet protocol

Vendored from `zcash/lightwalletd` release **v0.6.0**, commit
`e7436d2adb9ab5828eb8a27de534e16218841fc2`.

`service.proto` and `compact_formats.proto` are resolved from the upstream
`walletrpc/` symlinks into its pinned `lightwallet-protocol/walletrpc/` subtree.
`darkside.proto` is from `walletrpc/darkside.proto` in the same commit.
All share the protobuf package `cash.z.wallet.sdk.rpc`.

Source: https://github.com/zcash/lightwalletd/tree/v0.6.0/walletrpc
License: MIT; see `LICENSE.lightwalletd` and each source header.

The build uses `tonic-prost-build` and a Cargo-managed `protoc` binary;
no system protobuf compiler is required.
