use crate::{analyze, encode_hex, GroundTruth, PairReport, RunInput};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const FILES: [(&str, &str); 6] = [
    ("leaky/observations.jsonl", "leaky-observations.jsonl"),
    ("leaky/app.log", "leaky-app.log"),
    ("leaky/wallet.json", "leaky-wallet.json"),
    ("fixed/observations.jsonl", "fixed-observations.jsonl"),
    ("fixed/app.log", "fixed-app.log"),
    ("fixed/wallet.json", "fixed-wallet.json"),
];

/// A report and the exact six byte buffers from which it was validated.
/// Fields stay private: callers cannot attach unrelated bytes to a mutable
/// `RunInput` or replace the report after capture.
#[derive(Debug)]
pub struct CapturedPair {
    report: PairReport,
    files: BTreeMap<&'static str, Vec<u8>>,
}

impl CapturedPair {
    pub fn load(runs: &Path, gt: &GroundTruth, expected: Vec<String>) -> Result<Self> {
        let mut files = BTreeMap::new();
        for (source, download) in FILES {
            let path = runs.join(source);
            let bytes = fs::read(&path)
                .with_context(|| format!("cannot read required evidence {}", path.display()))?;
            files.insert(download, bytes);
        }
        let read_text = |name: &str| -> Result<&str> {
            let bytes = files
                .get(name)
                .context("missing captured evidence buffer")?;
            std::str::from_utf8(bytes)
                .with_context(|| format!("captured evidence {name} is not valid UTF-8"))
        };
        let analyze_profile = |profile: &str| {
            let input = RunInput::parse(
                read_text(&format!("{profile}-observations.jsonl"))?,
                read_text(&format!("{profile}-app.log"))?,
                read_text(&format!("{profile}-wallet.json"))?,
            )?;
            analyze(&input, gt)
        };
        let leaky = analyze_profile("leaky").context("leaky evidence validation failed")?;
        let fixed = analyze_profile("fixed").context("fixed evidence validation failed")?;
        let report = PairReport::new(gt, leaky, fixed, expected);
        Ok(Self { report, files })
    }

    pub fn report(&self) -> &PairReport {
        &self.report
    }

    /// Write the captured buffers, never reopen the source paths after validation.
    pub fn write_downloads(&self, output: &Path) -> Result<()> {
        fs::create_dir_all(output)?;
        for (name, bytes) in &self.files {
            let path = output.join(name);
            fs::write(&path, bytes)
                .with_context(|| format!("write captured evidence {}", path.display()))?;
        }
        Ok(())
    }

    /// Original files are base64-encoded directly, not reconstructed from parsed
    /// JSON. The outer JSON also escapes HTML delimiters in derived report text,
    /// so private fixture strings cannot terminate an inert script data element.
    pub fn replay_json(&self) -> String {
        let files: BTreeMap<_, _> = self
            .files
            .iter()
            .map(|(name, bytes)| {
                (
                    *name,
                    json!({
                        "base64": STANDARD.encode(bytes),
                        "sha256": encode_hex(&Sha256::digest(bytes)),
                        "byte_length": bytes.len(),
                    }),
                )
            })
            .collect();
        serde_json::to_string(&json!({
            "schema_version": 1,
            "report": &self.report,
            "files": files,
        }))
        .expect("captured replay data is serializable")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
    }
}
