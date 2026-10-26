//! A deliberately small receive-only subject. Ground truth is never an input to scanning.

pub mod fallback;

use anyhow::{anyhow, bail, ensure, Context, Result};
use sapling_crypto::{
    keys::PreparedIncomingViewingKey,
    note_encryption::{
        try_sapling_compact_note_decryption, try_sapling_note_decryption, CompactOutputDescription,
        Zip212Enforcement,
    },
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use zcash_client_backend::proto::compact_formats::CompactSaplingOutput;
use zcash_keys::encoding::decode_extended_spending_key;
use zcash_primitives::transaction::Transaction;
use zcash_protocol::consensus::{BlockHeight, BranchId, Network};

/// Finish the public retrieval schedule before any receiver-specific interpretation.
/// This function has no key, owned-note set, or memo validation input.
pub async fn collect_public_responses<C, R, F, Fut>(
    candidates: impl IntoIterator<Item = C>,
    mut fetch: F,
) -> Result<Vec<R>>
where
    F: FnMut(C) -> Fut,
    Fut: std::future::Future<Output = Result<R>>,
{
    let mut responses = Vec::new();
    for candidate in candidates {
        responses.push(fetch(candidate).await?);
    }
    Ok(responses)
}

#[derive(Debug, Deserialize)]
pub struct WalletConfig {
    pub network: String,
    pub scan_start: u64,
    pub scan_end: u64,
    pub extended_spending_key: String,
}

impl WalletConfig {
    pub fn read(path: &Path) -> Result<Self> {
        let config: Self = serde_json::from_slice(&fs::read(path)?)?;
        ensure!(
            config.network == "test",
            "this lab fixture only supports testnet"
        );
        ensure!(
            config.scan_start <= config.scan_end,
            "reversed public scan range"
        );
        Ok(config)
    }

    pub fn viewing_key(&self) -> Result<PreparedIncomingViewingKey> {
        // This is an explicitly public upstream fixture key, never a production wallet.
        let spending =
            decode_extended_spending_key("secret-extended-key-test", &self.extended_spending_key)
                .map_err(|e| anyhow!("decode public fixture key: {e}"))?;
        let full = spending.to_diversifiable_full_viewing_key();
        Ok(PreparedIncomingViewingKey::new(
            &full.to_ivk(zip32::Scope::External),
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceivedNote {
    pub txid: String,
    pub height: u64,
    pub output_index: usize,
    pub value_zat: u64,
    pub recipient_addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

pub fn enforcement(height: u64) -> Result<Zip212Enforcement> {
    let height = BlockHeight::from(u32::try_from(height)?);
    Ok(
        zcash_primitives::transaction::components::sapling::zip212_enforcement(
            &Network::TestNetwork,
            height,
        ),
    )
}

pub fn compact_note(
    key: &PreparedIncomingViewingKey,
    txid: &str,
    height: u64,
    output_index: usize,
    cmu: Vec<u8>,
    ephemeral_key: Vec<u8>,
    ciphertext: Vec<u8>,
) -> Result<Option<ReceivedNote>> {
    // Validate lengths before the backend's convenience conversion, which assumes cmu is 32 bytes.
    ensure!(cmu.len() == 32, "invalid compact note commitment length");
    ensure!(
        ephemeral_key.len() == 32,
        "invalid compact ephemeral key length"
    );
    ensure!(ciphertext.len() == 52, "invalid compact ciphertext length");
    let proto = CompactSaplingOutput {
        cmu,
        ephemeral_key,
        ciphertext,
    };
    let output: CompactOutputDescription = (&proto)
        .try_into()
        .map_err(|e| anyhow!("invalid compact Sapling output: {e}"))?;
    Ok(
        try_sapling_compact_note_decryption(key, &output, enforcement(height)?).map(
            |(note, address)| ReceivedNote {
                txid: txid.to_owned(),
                height,
                output_index,
                value_zat: note.value().inner(),
                memo: None,
                recipient_addr: zcash_keys::encoding::encode_payment_address(
                    "ztestsapling",
                    &address,
                ),
            },
        ),
    )
}

pub fn parse_transaction(raw: &[u8], height: u64) -> Result<Transaction> {
    let height = BlockHeight::from(u32::try_from(height)?);
    let branch = BranchId::for_height(&Network::TestNetwork, height);
    let mut cursor = std::io::Cursor::new(raw);
    let transaction =
        Transaction::read(&mut cursor, branch).context("parse real transaction bytes")?;
    ensure!(
        cursor.position() == raw.len() as u64,
        "trailing transaction bytes"
    );
    Ok(transaction)
}

pub fn full_notes(
    key: &PreparedIncomingViewingKey,
    transaction: &Transaction,
    height: u64,
) -> Result<Vec<ReceivedNote>> {
    let mut found = Vec::new();
    if let Some(bundle) = transaction.sapling_bundle() {
        for (output_index, output) in bundle.shielded_outputs().iter().enumerate() {
            if let Some((note, address, memo)) =
                try_sapling_note_decryption(key, output, enforcement(height)?)
            {
                let memo = zcash_protocol::memo::Memo::from_bytes(&memo)?;
                let text = match memo {
                    zcash_protocol::memo::Memo::Text(text) => text.to_string(),
                    zcash_protocol::memo::Memo::Empty => String::new(),
                    _ => {
                        bail!("fixture has a non-text memo; the merchant demonstration needs text")
                    }
                };
                found.push(ReceivedNote {
                    txid: transaction.txid().to_string(),
                    height,
                    output_index,
                    value_zat: note.value().inner(),
                    memo: Some(text),
                    recipient_addr: zcash_keys::encoding::encode_payment_address(
                        "ztestsapling",
                        &address,
                    ),
                });
            }
        }
    }
    Ok(found)
}

#[derive(Debug, Serialize)]
pub struct FixtureEvidence {
    pub file: String,
    pub staged_file: String,
    pub txid: String,
    pub transparent_inputs: usize,
    pub transparent_outputs: usize,
    pub sapling_spends: usize,
    pub sapling_outputs: usize,
    pub compact_matches: Vec<ReceivedNote>,
    pub decrypted_notes: Vec<ReceivedNote>,
}

/// Offline fixture gate: independently parse, compute the ID, and decrypt both ciphertext forms.
pub fn verify_fixtures(config: &WalletConfig, directory: &Path) -> Result<Vec<FixtureEvidence>> {
    let key = config.viewing_key()?;
    let staging_path = directory
        .parent()
        .unwrap_or(Path::new("fixtures"))
        .join("staging.json");
    let staging: serde_json::Value = serde_json::from_slice(
        &fs::read(staging_path).context("fixture gate requires the committed staging manifest")?,
    )?;
    let staged_transactions = staging["transactions"]
        .as_array()
        .context("staging manifest missing transactions")?;
    ensure!(
        staging["start_height"].as_u64() == Some(config.scan_start)
            && staging["end_height"].as_u64() == Some(config.scan_end),
        "staging range disagrees with wallet's public range"
    );
    let mut files: Vec<_> = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    files.sort_by_key(|entry| entry.path());
    let mut evidence = Vec::new();
    for entry in files {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("hex") {
            continue;
        }
        let raw = hex::decode(fs::read_to_string(&path)?.trim())?;
        let staged = staged_transactions
            .iter()
            .find(|item| item["txid"].as_str() == path.file_stem().and_then(|name| name.to_str()))
            .context("fixture transaction missing from staging manifest")?;
        let height = staged["height"]
            .as_u64()
            .context("staged transaction missing height")?;
        let staged_file = staged["file"]
            .as_str()
            .context("staged transaction missing file")?;
        let staged_path = directory
            .parent()
            .unwrap_or(Path::new("fixtures"))
            .join(staged_file);
        let staged_raw = hex::decode(fs::read_to_string(&staged_path)?.trim())?;
        ensure!(
            raw == staged_raw,
            "staged bytes differ from the independently verified upstream fixture: {}",
            staged_path.display()
        );
        ensure!(
            (config.scan_start..=config.scan_end).contains(&height),
            "staged transaction outside fixed public range"
        );
        let tx = parse_transaction(&raw, height)?;
        let txid = tx.txid().to_string();
        ensure!(
            path.file_stem().and_then(|s| s.to_str()) == Some(&txid),
            "fixture filename does not match the real transaction ID: {}",
            path.display()
        );
        let (transparent_inputs, transparent_outputs) = tx
            .transparent_bundle()
            .map(|b| (b.vin.len(), b.vout.len()))
            .unwrap_or((0, 0));
        ensure!(
            transparent_inputs == 0 && transparent_outputs == 0,
            "fixture has transparent inputs or outputs: {txid}"
        );
        ensure!(tx.sprout_bundle().is_none(), "fixture uses Sprout: {txid}");
        let bundle = tx
            .sapling_bundle()
            .context("fixture must contain Sapling")?;
        ensure!(
            !bundle.shielded_spends().is_empty() && !bundle.shielded_outputs().is_empty(),
            "fixture must be a Sapling spend with shielded outputs"
        );
        let mut compact_matches = Vec::new();
        for (index, output) in bundle.shielded_outputs().iter().enumerate() {
            let compact = CompactSaplingOutput::from(output);
            if let Some(found) = compact_note(
                &key,
                &txid,
                height,
                index,
                compact.cmu,
                compact.ephemeral_key,
                compact.ciphertext,
            )? {
                compact_matches.push(found);
            }
        }
        let decrypted_notes = full_notes(&key, &tx, height)?;
        ensure!(
            compact_matches.len() == decrypted_notes.len(),
            "compact/full decryption disagree"
        );
        for (compact, full) in compact_matches.iter().zip(&decrypted_notes) {
            ensure!(
                compact.txid == full.txid
                    && compact.output_index == full.output_index
                    && compact.value_zat == full.value_zat,
                "compact/full note mismatch"
            );
        }
        evidence.push(FixtureEvidence {
            file: path.to_string_lossy().replace('\\', "/"),
            staged_file: staged_path.to_string_lossy().replace('\\', "/"),
            txid,
            transparent_inputs,
            transparent_outputs,
            sapling_spends: bundle.shielded_spends().len(),
            sapling_outputs: bundle.shielded_outputs().len(),
            compact_matches,
            decrypted_notes,
        });
    }
    ensure!(!evidence.is_empty(), "no fixture transactions found");
    ensure!(
        evidence.len() == staged_transactions.len(),
        "staging manifest and fixture directory disagree"
    );
    ensure!(
        evidence.iter().any(|tx| tx
            .decrypted_notes
            .iter()
            .any(|note| note.memo.as_ref().is_some_and(|memo| !memo.is_empty()))),
        "fixture gate requires a real decryptable payment with a nonempty memo"
    );
    ensure!(
        evidence
            .iter()
            .any(|tx| tx.compact_matches.is_empty() && tx.decrypted_notes.is_empty()),
        "fixture gate requires unrelated cover transactions that do not decrypt"
    );

    // Ground truth is consulted ONLY by this offline gate, never by the network subject.
    // The committed expected values are authored once after fixture selection.
    let truth_path = directory
        .parent()
        .unwrap_or(Path::new("fixtures"))
        .join("ground_truth.json");
    let truth: serde_json::Value = serde_json::from_slice(
        &fs::read(truth_path)
            .context("fixture gate requires independently authored ground truth")?,
    )?;
    ensure!(
        truth["scan_start"].as_u64() == Some(config.scan_start)
            && truth["scan_end"].as_u64() == Some(config.scan_end),
        "ground truth range disagrees with public scan range"
    );
    let staged_ids: Vec<String> = staged_transactions
        .iter()
        .map(|tx| {
            Ok(tx["txid"]
                .as_str()
                .context("staged transaction missing txid")?
                .to_owned())
        })
        .collect::<Result<_>>()?;
    let expected_candidates: Vec<String> =
        serde_json::from_value(truth["candidate_txids"].clone())?;
    ensure!(
        staged_ids == expected_candidates,
        "ground truth candidates disagree with staged public order"
    );
    let payment = &truth["payment"];
    let expected_txid = payment["txid"]
        .as_str()
        .context("ground truth missing payment txid")?;
    let expected_value = payment["value_zat"]
        .as_u64()
        .context("ground truth missing value")?;
    let expected_memo = payment["memo_plaintext"]
        .as_str()
        .context("ground truth missing memo")?;
    let expected_height = payment["height"]
        .as_u64()
        .context("ground truth missing height")?;
    let expected_index = payment["output_index"]
        .as_u64()
        .context("ground truth missing output index")?;
    let expected_recipient = payment["recipient_address"]
        .as_str()
        .context("ground truth missing recipient")?;
    ensure!(
        payment["pool"].as_str() == Some("sapling"),
        "unexpected ground truth pool"
    );
    ensure!(
        evidence
            .iter()
            .map(|tx| tx.decrypted_notes.len())
            .sum::<usize>()
            == 1,
        "fixture must contain exactly one recipient note and unrelated cover transactions"
    );
    ensure!(
        evidence
            .iter()
            .flat_map(|tx| &tx.decrypted_notes)
            .any(|note| note.txid == expected_txid
                && note.value_zat == expected_value
                && note.height == expected_height
                && note.output_index as u64 == expected_index
                && note.recipient_addr == expected_recipient
                && note.memo.as_deref() == Some(expected_memo)),
        "real fixture decryption does not match committed payment ground truth"
    );
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    fn payment() -> (WalletConfig, Transaction, u64) {
        let root = fixture_root();
        let config = WalletConfig::read(&root.join("wallet.json")).unwrap();
        let truth: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("ground_truth.json")).unwrap()).unwrap();
        let txid = truth["payment"]["txid"].as_str().unwrap();
        let height = truth["payment"]["height"].as_u64().unwrap();
        let raw = hex::decode(
            fs::read_to_string(root.join("transactions").join(format!("{txid}.hex")))
                .unwrap()
                .trim(),
        )
        .unwrap();
        (config, parse_transaction(&raw, height).unwrap(), height)
    }

    #[test]
    fn unrelated_key_cannot_claim_the_payment() {
        let (_, tx, height) = payment();
        let spending = sapling_crypto::zip32::ExtendedSpendingKey::master(&[0u8; 32]).unwrap();
        let full = spending.to_diversifiable_full_viewing_key();
        let wrong_key = PreparedIncomingViewingKey::new(&full.to_ivk(zip32::Scope::External));
        assert!(full_notes(&wrong_key, &tx, height).unwrap().is_empty());
        for (index, output) in tx
            .sapling_bundle()
            .unwrap()
            .shielded_outputs()
            .iter()
            .enumerate()
        {
            let compact = CompactSaplingOutput::from(output);
            assert!(compact_note(
                &wrong_key,
                &tx.txid().to_string(),
                height,
                index,
                compact.cmu,
                compact.ephemeral_key,
                compact.ciphertext
            )
            .unwrap()
            .is_none());
        }
    }

    #[test]
    fn corrupt_compact_ciphertext_does_not_match_ground_truth() {
        let (config, tx, height) = payment();
        let key = config.viewing_key().unwrap();
        let full = full_notes(&key, &tx, height).unwrap();
        assert!(!full.is_empty(), "positive decryption control");
        let index = full[0].output_index;
        let output = &tx.sapling_bundle().unwrap().shielded_outputs()[index];
        let mut compact = CompactSaplingOutput::from(output);
        compact.ciphertext[0] ^= 0xff;
        assert!(compact_note(
            &key,
            &tx.txid().to_string(),
            height,
            index,
            compact.cmu,
            compact.ephemeral_key,
            compact.ciphertext
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn malformed_network_output_is_an_error_not_a_panic() {
        let config = WalletConfig::read(&fixture_root().join("wallet.json")).unwrap();
        let key = config.viewing_key().unwrap();
        assert!(compact_note(
            &key,
            "",
            config.scan_start,
            0,
            vec![0; 31],
            vec![0; 32],
            vec![0; 52]
        )
        .is_err());
    }

    #[tokio::test]
    async fn full_public_batch_finishes_before_local_payload_errors() {
        let mut observed = Vec::new();
        let responses = collect_public_responses(["cover-a", "payment", "cover-b"], |id| {
            observed.push(id);
            // Deliberately malformed first payload: local parsing must happen later.
            std::future::ready(Ok(Vec::<u8>::new()))
        })
        .await
        .unwrap();
        assert_eq!(observed, ["cover-a", "payment", "cover-b"]);
        assert_eq!(responses.len(), 3);
        assert!(parse_transaction(&responses[0], 1_028_505).is_err());
    }

    #[tokio::test]
    async fn matching_and_nonmatching_keys_have_the_same_public_fetch_order() {
        let (config, _, height) = payment();
        let right_key = config.viewing_key().unwrap();
        let wrong_spending =
            sapling_crypto::zip32::ExtendedSpendingKey::master(&[0u8; 32]).unwrap();
        let wrong_full = wrong_spending.to_diversifiable_full_viewing_key();
        let wrong_key = PreparedIncomingViewingKey::new(&wrong_full.to_ivk(zip32::Scope::External));
        let root = fixture_root();
        let truth: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("ground_truth.json")).unwrap()).unwrap();
        let candidates: Vec<String> =
            serde_json::from_value(truth["candidate_txids"].clone()).unwrap();
        let mut match_counts = Vec::new();
        for key in [&right_key, &wrong_key] {
            let mut observed = Vec::new();
            let responses = collect_public_responses(candidates.iter(), |txid| {
                observed.push(txid.clone());
                let raw = hex::decode(
                    fs::read_to_string(root.join("transactions").join(format!("{txid}.hex")))
                        .unwrap()
                        .trim(),
                )
                .unwrap();
                std::future::ready(Ok(raw))
            })
            .await
            .unwrap();
            assert_eq!(observed, candidates);
            let matches: usize = responses
                .iter()
                .map(|raw| {
                    let tx = parse_transaction(raw, height).unwrap();
                    full_notes(key, &tx, height).unwrap().len()
                })
                .sum();
            match_counts.push(matches);
        }
        assert_eq!(
            match_counts,
            [1, 0],
            "actual cryptographic positive and negative controls"
        );
    }
}
