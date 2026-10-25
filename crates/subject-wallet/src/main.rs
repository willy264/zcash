use anyhow::{ensure, Context, Result};
use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use subject_wallet::{
    collect_public_responses, compact_note, full_notes, parse_transaction, verify_fixtures,
    ReceivedNote, WalletConfig,
};
use walletrpc::{BlockId, BlockRange, CompactTxStreamerClient, TxFilter};
use zcash_primitives::transaction::TxId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Good,
    Leaky,
}

#[derive(Parser, Debug)]
#[command(about = "Darkroom receive-only subject using real Sapling trial decryption")]
struct Args {
    #[arg(long, value_enum, default_value = "good")]
    mode: Mode,
    #[arg(long, default_value = "http://127.0.0.1:19067")]
    endpoint: String,
    #[arg(long, default_value = "fixtures/wallet.json")]
    config: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Recover memos by fetching every transaction in the fixed public scan window.
    #[arg(long)]
    memo: bool,
    /// Full URL of the merchant reconciliation endpoint. Requires --memo.
    #[arg(long, requires = "memo")]
    merchant_url: Option<String>,
    /// Offline: parse and independently decrypt every committed fixture transaction.
    #[arg(long)]
    verify_fixture: bool,
    #[arg(long, default_value = "fixtures/transactions")]
    fixture_directory: PathBuf,
}

#[derive(Serialize)]
struct WalletEvidence {
    schema_version: u32,
    mode: Mode,
    scan_start: u64,
    scan_end: u64,
    compact_blocks_scanned: u64,
    compact_transactions_scanned: usize,
    candidate_txids: Vec<String>,
    notes: Vec<ReceivedNote>,
    memo_policy: &'static str,
    full_window_fetch_count: usize,
    targeted_fetch_count: usize,
    merchant_deliveries: usize,
    wallet_engine: &'static str,
}

fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    fs::write(path, data)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = WalletConfig::read(&args.config)?;
    if args.verify_fixture {
        let evidence = verify_fixtures(&config, &args.fixture_directory)?;
        if let Some(output) = args.output {
            save_json(&output, &evidence)?;
        }
        println!("{}", serde_json::to_string_pretty(&evidence)?);
        return Ok(());
    }
    let key = config.viewing_key()?;
    let channel = tonic::transport::Endpoint::from_shared(args.endpoint)?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .connect()
        .await
        .context("connect to observer proxy")?;
    let mut client = CompactTxStreamerClient::new(channel);
    let range = BlockRange {
        start: Some(BlockId {
            height: config.scan_start,
            ..Default::default()
        }),
        end: Some(BlockId {
            height: config.scan_end,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut blocks = client.get_block_range(range).await?.into_inner();
    let mut blocks_scanned = 0u64;
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let mut targeted = HashSet::new();
    let mut notes = Vec::new();
    while let Some(block) = tokio::time::timeout(Duration::from_secs(30), blocks.message())
        .await
        .context("compact block stream stalled")??
    {
        ensure!(
            block.height == config.scan_start + blocks_scanned,
            "server returned a gap or reordered compact blocks"
        );
        ensure!(
            block.height <= config.scan_end,
            "server returned blocks outside scan range"
        );
        blocks_scanned += 1;
        for tx in block.vtx {
            let id_bytes: [u8; 32] = tx
                .txid
                .as_slice()
                .try_into()
                .context("compact transaction ID must be 32 bytes")?;
            let txid = TxId::from_bytes(id_bytes).to_string();
            if seen.insert(txid.clone()) {
                // Ownership is irrelevant to this fixed public candidate list.
                candidates.push((txid.clone(), id_bytes, block.height));
            }
            for (output_index, output) in tx.outputs.into_iter().enumerate() {
                if let Some(found) = compact_note(
                    &key,
                    &txid,
                    block.height,
                    output_index,
                    output.cmu,
                    output.ephemeral_key,
                    output.ciphertext,
                )? {
                    // Intentional leak: only successfully decrypted notes cause this lookup.
                    if args.mode == Mode::Leaky && targeted.insert(txid.clone()) {
                        let response = client
                            .get_transaction(TxFilter {
                                hash: id_bytes.to_vec(),
                                ..Default::default()
                            })
                            .await?
                            .into_inner();
                        let downloaded = parse_transaction(&response.data, block.height)?;
                        ensure!(
                            downloaded.txid().to_string() == txid,
                            "server returned the wrong transaction for a targeted lookup"
                        );
                    }
                    notes.push(found);
                }
            }
        }
    }
    ensure!(
        blocks_scanned == config.scan_end - config.scan_start + 1,
        "server did not return the complete fixed scan window"
    );
    let mut full_window_fetch_count = 0;
    if args.memo {
        let full_window =
            collect_public_responses(candidates.iter().cloned(), |(txid, bytes, height)| {
                let mut connection = client.clone();
                async move {
                    let response = connection
                        .get_transaction(TxFilter {
                            hash: bytes.to_vec(),
                            ..Default::default()
                        })
                        .await?
                        .into_inner();
                    Ok((txid, height, response.data))
                }
            })
            .await?;
        full_window_fetch_count = full_window.len();
        // Fetch the ENTIRE public batch before local parsing/decryption. Owned-note
        // errors (including invalid memo text) must not truncate the observable schedule.
        for (txid, height, raw) in full_window {
            let tx = parse_transaction(&raw, height)?;
            ensure!(
                tx.txid().to_string() == txid,
                "server returned the wrong transaction during full-window retrieval"
            );
            for full in full_notes(&key, &tx, height)? {
                let compact = notes
                    .iter_mut()
                    .find(|n| n.txid == full.txid && n.output_index == full.output_index)
                    .context("full decryption found a note missing from compact scan")?;
                ensure!(
                    compact.value_zat == full.value_zat
                        && compact.recipient_addr == full.recipient_addr,
                    "full decryption disagrees with compact scan"
                );
                compact.memo = full.memo;
            }
        }
        ensure!(
            notes.iter().all(|note| note.memo.is_some()),
            "full-window retrieval failed to recover a scanned note's memo"
        );
    }
    // The fixture's positive control is checked only AFTER the ownership-independent
    // retrieval schedule. Even a key with no matches requests the same public batch.
    ensure!(
        !notes.is_empty(),
        "no notes decrypted for the configured public test key"
    );
    let mut merchant_deliveries = 0;
    if let Some(url) = args.merchant_url {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        for note in &notes {
            let memo = note
                .memo
                .as_ref()
                .context("merchant requires a decrypted memo")?;
            ensure!(
                !memo.is_empty(),
                "merchant demonstration requires nonempty decrypted memo"
            );
            http.post(&url)
                .json(&serde_json::json!({
                    "memo": memo, "reply_addr": note.recipient_addr,
                }))
                .send()
                .await
                .context("deliver decrypted memo to merchant")?
                .error_for_status()
                .context("merchant rejected reconciliation")?;
            merchant_deliveries += 1;
        }
    }
    let evidence = WalletEvidence {
        schema_version: 1,
        mode: args.mode,
        scan_start: config.scan_start,
        scan_end: config.scan_end,
        compact_blocks_scanned: blocks_scanned,
        compact_transactions_scanned: candidates.len(),
        candidate_txids: candidates.into_iter().map(|(id, _, _)| id).collect(),
        notes,
        memo_policy: if args.memo { "full-window" } else { "none" },
        full_window_fetch_count,
        targeted_fetch_count: targeted.len(),
        merchant_deliveries,
        wallet_engine:
            "librustzcash + sapling-crypto; compact trial decryption; full memo decryption",
    };
    let mode_name = if args.mode == Mode::Good {
        "good"
    } else {
        "leaky"
    };
    let output = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!(".runs/{mode_name}/wallet.json")));
    save_json(&output, &evidence)?;
    println!(
        "{mode_name}: scanned {blocks_scanned} compact blocks, decrypted {} note(s); evidence {}",
        evidence.notes.len(),
        output.display()
    );
    Ok(())
}
