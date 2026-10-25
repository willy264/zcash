//! Pinned lightwalletd protocol, with generated clients and servers.
pub mod walletrpc {
    tonic::include_proto!("cash.z.wallet.sdk.rpc");
}
pub use walletrpc::compact_tx_streamer_client::CompactTxStreamerClient;
pub use walletrpc::compact_tx_streamer_server::{CompactTxStreamer, CompactTxStreamerServer};
pub use walletrpc::darkside_streamer_client::DarksideStreamerClient;
pub use walletrpc::*;
