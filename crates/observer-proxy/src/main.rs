mod proxy;
#[cfg(test)]
mod proxy_tests;
mod transparent;

use anyhow::{Context, Result};
use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tonic::transport::{Endpoint, Server};
use walletrpc::{CompactTxStreamerClient, CompactTxStreamerServer};

#[derive(Parser)]
#[command(about = "Record the wallet's real gRPC access pattern and forward to lightwalletd")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:19067")]
    listen: SocketAddr,
    #[arg(long, default_value = "http://127.0.0.1:9067")]
    upstream: String,
    #[arg(long, default_value = "observations.jsonl")]
    log: PathBuf,
    #[arg(long, default_value = "subject-1")]
    session: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        args.listen.ip().is_loopback(),
        "the lab proxy must bind to loopback"
    );
    let channel = Endpoint::from_shared(args.upstream.clone())?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .connect()
        .await
        .context("connect to lightwalletd")?;
    let upstream = CompactTxStreamerClient::new(channel)
        .max_decoding_message_size(16 * 1024 * 1024)
        .max_encoding_message_size(16 * 1024 * 1024);
    let proxy = proxy::Proxy::new(upstream, &args.log, args.session)
        .await
        .context("create new observation log (use a fresh path for each run)")?;
    eprintln!(
        "observer-proxy ready on {} -> {}",
        args.listen, args.upstream
    );
    Server::builder()
        .add_service(
            CompactTxStreamerServer::new(proxy)
                .max_decoding_message_size(16 * 1024 * 1024)
                .max_encoding_message_size(16 * 1024 * 1024),
        )
        .serve_with_shutdown(args.listen, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
