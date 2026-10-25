use ::walletrpc::*;
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::{path::Path, time::Duration};
use tonic::transport::Endpoint;

#[derive(Deserialize)]
pub struct Manifest {
    pub chain_name: String,
    pub branch_id: String,
    pub start_height: u64,
    pub end_height: u64,
    pub transactions: Vec<StagedTransaction>,
}

#[derive(Deserialize)]
pub struct StagedTransaction {
    pub file: String,
    pub height: u64,
}

pub async fn stage(endpoint: &str, fixtures: &Path) -> Result<()> {
    let manifest: Manifest =
        serde_json::from_slice(&tokio::fs::read(fixtures.join("staging.json")).await?)
            .context("read fixtures/staging.json")?;
    ensure!(
        manifest.start_height > 0 && manifest.end_height >= manifest.start_height,
        "invalid fixture height range"
    );
    ensure!(
        manifest.end_height - manifest.start_height < 10_000,
        "fixture range exceeds lab limit"
    );
    ensure!(
        !manifest.transactions.is_empty(),
        "fixture contains no transactions"
    );
    let height = i32::try_from(manifest.start_height)?;
    let tip = i32::try_from(manifest.end_height)?;
    let mut transactions = Vec::new();
    let fixture_dir = tokio::fs::canonicalize(fixtures).await?;
    for transaction in manifest.transactions {
        ensure!(
            (manifest.start_height..=manifest.end_height).contains(&transaction.height),
            "transaction height outside staged chain"
        );
        let path = tokio::fs::canonicalize(fixtures.join(&transaction.file)).await?;
        ensure!(
            path.starts_with(&fixture_dir),
            "transaction file must be inside fixture directory"
        );
        let text = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("read {}", path.display()))?;
        let data =
            hex::decode(text.trim()).with_context(|| format!("decode {}", path.display()))?;
        ensure!(!data.is_empty(), "empty transaction fixture");
        transactions.push(RawTransaction {
            data,
            height: transaction.height,
        });
    }
    let channel = Endpoint::from_shared(endpoint.to_owned())?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .connect()
        .await?;
    let mut darkside = DarksideStreamerClient::new(channel.clone());
    darkside
        .reset(DarksideMetaState {
            sapling_activation: height,
            branch_id: manifest.branch_id,
            chain_name: manifest.chain_name,
            ..Default::default()
        })
        .await
        .context("DarksideStreamer.Reset")?;
    darkside
        .stage_blocks_create(DarksideEmptyBlocks {
            height,
            nonce: 7,
            count: tip - height + 1,
        })
        .await
        .context("DarksideStreamer.StageBlocksCreate")?;
    let transaction_count = transactions.len();
    darkside
        .stage_transactions_stream(tokio_stream::iter(transactions))
        .await
        .context("DarksideStreamer.StageTransactionsStream")?;
    darkside
        .apply_staged(DarksideHeight { height: tip })
        .await
        .context("DarksideStreamer.ApplyStaged")?;
    // ApplyStaged activates the mock chain before the compact-block ingestor catches up.
    // Wait for the served compact tip instead of relying on a fixed sleep in CI.
    let mut wallet = CompactTxStreamerClient::new(channel);
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if let Ok(block) = wallet
                .get_block(BlockId {
                    height: manifest.end_height,
                    hash: vec![],
                })
                .await
            {
                if block.into_inner().height == manifest.end_height {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("timed out waiting for the staged compact tip")?;
    println!(
        "staged {transaction_count} transactions at heights {}..={} via DarksideStreamer",
        manifest.start_height, manifest.end_height
    );
    Ok(())
}
