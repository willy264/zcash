use std::{
    fs::OpenOptions,
    io::Write,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Parser)]
#[command(about = "A local merchant log sink for Darkroom's memo-disclosure experiment")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:18080")]
    listen: SocketAddr,
    #[arg(long, default_value = "app.log")]
    log: PathBuf,
}

#[derive(Clone)]
struct App {
    log: Arc<Mutex<std::fs::File>>,
    redact: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Order {
    memo: String,
    #[serde(default, alias = "address", alias = "reply_address")]
    reply_addr: String,
}

fn record(order: &Order, redact: bool) -> Value {
    if redact {
        json!({"event": "order_reconciled", "memo": "[REDACTED]", "reply_addr": "[REDACTED]"})
    } else {
        json!({"event": "order_reconciled", "memo": order.memo, "reply_addr": order.reply_addr})
    }
}

async fn reconcile(
    State(app): State<App>,
    Json(order): Json<Order>,
) -> Result<StatusCode, StatusCode> {
    let line = record(&order, app.redact);
    let mut file = app
        .log
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    writeln!(file, "{line}")
        .and_then(|()| file.flush())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(parent) = args.log.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.log)
        .context("open app evidence log")?;
    let app = App {
        log: Arc::new(Mutex::new(file)),
        redact: std::env::var("DARKROOM_FIX").is_ok_and(|v| v == "1"),
    };
    let router = Router::new()
        .route("/health", get(|| async { StatusCode::NO_CONTENT }))
        .route("/reconcile", post(reconcile))
        .route("/", post(reconcile))
        .layer(DefaultBodyLimit::max(8192))
        .with_state(app);
    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    eprintln!("merchant-app listening on {}", listener.local_addr()?);
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_log_does_not_contain_private_fields_even_with_log_injection_text() {
        let order = Order {
            memo: "ORDER-private\n{\"event\":\"forged\"}".into(),
            reply_addr: "zs-private-address".into(),
        };
        let fixed = record(&order, true).to_string();
        assert!(!fixed.contains("ORDER-private"));
        assert!(!fixed.contains("zs-private-address"));
        let leaky = record(&order, false).to_string();
        assert!(leaky.contains("ORDER-private"));
        assert!(
            !leaky.contains('\n'),
            "one event must remain one JSONL line"
        );
    }
}
