mod a2;
mod capture;
mod report;
use a2::{analyze_send, same_send_scenario};
pub use capture::CapturedPair;
pub use report::{render_captured_report, render_report};

use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Payment {
    pub txid: String,
    pub height: u64,
    pub value_zat: u64,
    pub memo_plaintext: String,
    pub pool: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroundTruth {
    pub payment: Payment,
    pub sensitive_txids: Vec<String>,
    #[serde(default)]
    pub sensitive_addresses: Vec<String>,
    pub candidate_txids: Vec<String>,
}

fn valid_txid(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl GroundTruth {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            valid_txid(&self.payment.txid),
            "payment txid must be canonical lowercase display hex"
        );
        ensure!(
            !self.payment.memo_plaintext.is_empty(),
            "empty fixture memo cannot exercise memo leakage"
        );
        ensure!(
            self.payment.value_zat > 0,
            "fixture payment must have positive value"
        );
        ensure!(
            self.sensitive_txids.contains(&self.payment.txid),
            "payment must be in sensitive_txids"
        );
        ensure!(
            self.candidate_txids.contains(&self.payment.txid),
            "payment must be in candidate_txids"
        );
        ensure!(
            self.sensitive_txids
                .iter()
                .chain(&self.candidate_txids)
                .all(|id| valid_txid(id)),
            "ground truth contains invalid txid"
        );
        ensure!(
            self.sensitive_addresses.iter().all(|s| !s.is_empty()),
            "empty sensitive address is invalid"
        );
        let unique: BTreeSet<_> = self.candidate_txids.iter().collect();
        ensure!(
            unique.len() == self.candidate_txids.len(),
            "candidate_txids must be unique and in public scan order"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Observation {
    pub seq: u64,
    pub session: String,
    pub method: String,
    pub args: Value,
    #[serde(default)]
    pub ts: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Note {
    pub txid: String,
    pub height: u64,
    pub value_zat: u64,
    #[serde(default)]
    pub memo: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalletEvidence {
    pub mode: String,
    pub scan_start: u64,
    pub scan_end: u64,
    pub compact_blocks_scanned: u64,
    pub candidate_txids: Vec<String>,
    pub merchant_deliveries: usize,
    pub notes: Vec<Note>,
    pub memo_policy: String,
    #[serde(default)]
    pub send_attempt: Option<SendAttempt>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestedPool {
    Sapling,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShieldedFailure {
    InsufficientScannedValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    Refuse,
    Transparent,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SendOutcome {
    Refused,
    TransparentSubmissionAttempted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubmissionResult {
    pub rpc_completed: bool,
    pub error_code: Option<i32>,
    pub error_message: String,
}

/// Subject-side exercise evidence is checked against the captured RPC bytes below.
/// It establishes an attempted local policy decision, never consensus acceptance.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SendAttempt {
    pub requested_pool: RequestedPool,
    pub requested_value_zat: u64,
    pub available_note_upper_bound_zat: u64,
    pub shielded_attempted: bool,
    pub shielded_failure: ShieldedFailure,
    pub failure_detail: String,
    pub fallback_policy: FallbackPolicy,
    pub outcome: SendOutcome,
    pub fallback_raw_tx_hex: Option<String>,
    pub submitted_txid: Option<String>,
    pub transparent_destination: Option<String>,
    pub submission_result: Option<SubmissionResult>,
    pub consensus_valid: bool,
    pub funds_moved: bool,
    pub input_model: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmittedTransaction {
    pub observation_sequence: u64,
    pub txid: String,
    pub raw_tx_hex: String,
    pub transparent_inputs: usize,
    pub transparent_outputs: usize,
    pub output_values_zat: Vec<u64>,
    pub output_scripts_hex: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RunInput {
    pub observations: Vec<Observation>,
    pub app_log: String,
    pub wallet: WalletEvidence,
}

impl RunInput {
    pub fn parse(observations: &str, app_log: &str, wallet: &str) -> Result<Self> {
        ensure!(
            !observations.trim().is_empty(),
            "empty observation log is missing evidence, not a clean run"
        );
        parse_application_log(app_log)?;
        let mut parsed = Vec::new();
        for (index, line) in observations.lines().enumerate() {
            ensure!(
                !line.trim().is_empty(),
                "blank observation line {}",
                index + 1
            );
            let obs: Observation = serde_json::from_str(line)
                .with_context(|| format!("invalid observation JSON on line {}", index + 1))?;
            ensure!(
                obs.seq == index as u64 + 1,
                "observation sequence is incomplete or duplicated at line {}",
                index + 1
            );
            ensure!(
                !obs.session.is_empty() && !obs.method.is_empty(),
                "missing observation identity"
            );
            parsed.push(obs);
        }
        Ok(Self {
            observations: parsed,
            app_log: app_log.into(),
            wallet: serde_json::from_str(wallet).context("invalid wallet evidence JSON")?,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub id: &'static str,
    pub title: &'static str,
    pub exercised: bool,
    pub leaked: bool,
    pub evidence: String,
    pub inference: String,
    pub fix: &'static str,
    pub observation_sequences: Vec<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    pub profile: String,
    pub validated: bool,
    pub findings: Vec<Finding>,
    pub observation_count: usize,
    pub transaction_fetches: Vec<String>,
    pub wallet: WalletEvidence,
    pub app_log_excerpt: String,
    pub full_window_schedule: bool,
    pub submitted_transactions: Vec<SubmittedTransaction>,
}

impl RunResult {
    pub fn leaked_ids(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|f| f.leaked)
            .map(|f| f.id.to_owned())
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct PairReport {
    pub schema_version: u32,
    pub project: &'static str,
    pub passed: bool,
    pub expected_leaks: Vec<String>,
    pub exercised_checks: Vec<&'static str>,
    pub ground_truth: Payment,
    pub candidate_txids: Vec<String>,
    pub provenance: Provenance,
    pub leaky: RunResult,
    pub fixed: RunResult,
    pub trust_boundary: &'static str,
}

impl PairReport {
    pub fn new(
        gt: &GroundTruth,
        leaky: RunResult,
        fixed: RunResult,
        expected_leaks: Vec<String>,
    ) -> Self {
        let passed = enforce(&leaky, &expected_leaks).is_ok()
            && enforce(&fixed, &[]).is_ok()
            && require_exercised(&fixed, &expected_leaks).is_ok()
            && leaky.wallet.scan_start == fixed.wallet.scan_start
            && leaky.wallet.scan_end == fixed.wallet.scan_end
            && same_send_scenario(&leaky.wallet.send_attempt, &fixed.wallet.send_attempt);
        let exercised_checks = leaky
            .findings
            .iter()
            .filter(|f| f.exercised)
            .map(|f| f.id)
            .collect();
        Self {
            schema_version: 1,
            project: "Darkroom",
            passed,
            expected_leaks,
            exercised_checks,
            ground_truth: gt.payment.clone(),
            candidate_txids: gt.candidate_txids.clone(),
            provenance: Provenance::from_env(),
            leaky,
            fixed,
            trust_boundary: "Real wallet cryptography and captured gRPC/application traces on a staged test chain. The observer is the light-wallet server plus application logs. Findings establish these specific disclosures; zero findings does not prove the absence of other leaks. Darkside does not establish consensus validity. A transaction query links a session to interest in a transaction; it does not independently establish recipient identity. Full-window retrieval removes selection within this fixed public window, not all timing or network metadata.",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Provenance {
    pub repository_url: String,
    pub fixture_source_url: String,
    pub commit: Option<String>,
    pub ci_run_url: Option<String>,
}

impl Provenance {
    fn from_env() -> Self {
        let repository = std::env::var("GITHUB_REPOSITORY")
            .ok()
            .or_else(|| std::env::var("DARKROOM_REPOSITORY").ok())
            .filter(|s| {
                s.split('/').count() == 2
                    && s.split('/').all(|p| {
                        !p.is_empty()
                            && p.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                    })
            })
            .unwrap_or_else(|| "willy264/zcash".into());
        let repository_url = format!("https://github.com/{repository}");
        let commit = std::env::var("GITHUB_SHA")
            .ok()
            .filter(|s| (40..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit()));
        let ci_run_url = std::env::var("GITHUB_RUN_ID")
            .ok()
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .map(|id| format!("{repository_url}/actions/runs/{id}"));
        let revision = commit.as_deref().unwrap_or("main");
        let fixture_source_url = format!("{repository_url}/blob/{revision}/fixtures/README.md");
        Self {
            repository_url,
            fixture_source_url,
            commit,
            ci_run_url,
        }
    }
}

pub fn analyze(input: &RunInput, gt: &GroundTruth) -> Result<RunResult> {
    gt.validate()?;
    let app_events = parse_application_log(&input.app_log)?;
    ensure!(!input.observations.is_empty(), "missing observations");
    let session = &input.observations[0].session;
    ensure!(
        input.observations.iter().all(|o| o.session == *session),
        "one run must contain exactly one observer session"
    );
    let w = &input.wallet;
    ensure!(
        w.candidate_txids == gt.candidate_txids,
        "scanned public candidates disagree with fixture ground truth order"
    );
    ensure!(w.scan_end >= w.scan_start, "invalid scan interval");
    ensure!(
        w.compact_blocks_scanned == w.scan_end - w.scan_start + 1,
        "wallet did not scan the complete public interval"
    );
    ensure!(
        gt.payment.height >= w.scan_start && gt.payment.height <= w.scan_end,
        "scan does not cover payment"
    );
    ensure!(
        input
            .observations
            .iter()
            .any(|o| o.method == "GetBlockRange"
                && o.args.get("start").and_then(Value::as_u64) == Some(w.scan_start)
                && o.args.get("end").and_then(Value::as_u64) == Some(w.scan_end)),
        "no observed range request matches wallet scan evidence"
    );
    let note = w
        .notes
        .iter()
        .find(|n| {
            n.txid == gt.payment.txid
                && n.height == gt.payment.height
                && n.value_zat == gt.payment.value_zat
        })
        .context(
            "wallet did not report a decrypted note matching fixture txid, height, and amount",
        )?;
    ensure!(
        matches!(w.memo_policy.as_str(), "none" | "full-window"),
        "unknown wallet memo policy"
    );
    if w.memo_policy == "full-window" {
        ensure!(
            note.memo.as_deref() == Some(gt.payment.memo_plaintext.as_str()),
            "decrypted memo does not match fixture ground truth"
        );
        ensure!(
            gt.candidate_txids.len() >= 2
                && gt
                    .candidate_txids
                    .iter()
                    .any(|id| !gt.sensitive_txids.contains(id)),
            "full-window mitigation needs unrelated candidate transactions"
        );
        ensure!(
            gt.candidate_txids
                .iter()
                .any(|id| !w.notes.iter().any(|note| note.txid == *id)),
            "fixture has no unrelated transaction that the subject cannot decrypt"
        );
        let reconciliations = app_events
            .iter()
            .filter(|event| event.get("event").and_then(Value::as_str) == Some("order_reconciled"))
            .count();
        ensure!(
            w.merchant_deliveries > 0 && reconciliations == w.merchant_deliveries,
            "merchant reconciliation events do not match successful wallet deliveries"
        );
    }
    let fetch_obs: Vec<_> = input
        .observations
        .iter()
        .filter(|o| o.method == "GetTransaction")
        .collect();
    let fetches: Vec<String> = fetch_obs
        .iter()
        .map(|o| {
            let id = o
                .args
                .get("txid")
                .and_then(Value::as_str)
                .context("GetTransaction missing txid")?;
            ensure!(
                valid_txid(id),
                "GetTransaction txid is not canonical display hex"
            );
            Ok(id.to_owned())
        })
        .collect::<Result<_>>()?;
    if w.memo_policy == "full-window" {
        ensure!(
            gt.candidate_txids.iter().all(|txid| fetches.contains(txid)),
            "wallet claims full-window memo retrieval but observed requests omit public candidates"
        );
    }
    let full_window_schedule = gt.candidate_txids.len() >= 2 && fetches == gt.candidate_txids;
    let mut a1_hits: Vec<u64> = Vec::new();
    if !full_window_schedule {
        for (o, txid) in fetch_obs.iter().zip(&fetches) {
            if gt.sensitive_txids.contains(txid) {
                a1_hits.push(o.seq);
            }
        }
    }
    for obs in &input.observations {
        if matches!(
            obs.method.as_str(),
            "GetTaddressTxids"
                | "GetTaddressTransactions"
                | "GetAddressUtxos"
                | "GetAddressUtxosStream"
                | "GetTaddressBalance"
                | "GetTaddressBalanceStream"
        ) {
            let args = string_values(&obs.args);
            if args.iter().any(|s| gt.sensitive_addresses.contains(s)) {
                a1_hits.push(obs.seq);
            }
        }
    }
    let a1 = Finding {
        id: "A1", title: "Targeted transaction fetch", exercised: true, leaked: !a1_hits.is_empty(),
        evidence: if !a1_hits.is_empty() {
            format!("{} sensitive query event(s); GetTransaction sequence: {}", a1_hits.len(), fetches.join(" → "))
        } else if full_window_schedule {
            format!("{} public candidate transactions fetched exactly once in chain order; no extra targeted query.", fetches.len())
        } else if fetches.is_empty() {
            format!("GetBlockRange({}, {}); no GetTransaction calls.", w.scan_start, w.scan_end)
        } else { "No query named a ground-truth sensitive transaction or address.".into() },
        inference: if a1_hits.is_empty() {
            "No selective link to the payment appears in this trace. The server still sees the session, scan window, request order, and timing.".into()
        } else {
            "The observer links this session to interest in the specific shielded transaction or address. Ground truth confirms the payment belongs to this test wallet; the query alone does not prove recipient identity.".into()
        },
        fix: "Scan locally. When memos are needed, retrieve every transaction in the same fixed public window in chain order, with no extra lookup for a match.",
        observation_sequences: a1_hits,
    };
    let (a2, submitted_transactions) = analyze_send(input)?;
    let mut log_values = vec![input.app_log.clone()];
    for event in &app_events {
        log_values.extend(string_values(event));
    }
    let memo_leaked = log_values
        .iter()
        .any(|s| s.contains(&gt.payment.memo_plaintext));
    let address_leaked = gt
        .sensitive_addresses
        .iter()
        .any(|a| log_values.iter().any(|s| s.contains(a)));
    let mut fields = Vec::new();
    if memo_leaked {
        fields.push("memo");
    }
    if address_leaked {
        fields.push("address");
    }
    let a3 = Finding {
        id: "A3",
        title: "Private memo in application logs",
        exercised: w.memo_policy == "full-window",
        leaked: !fields.is_empty(),
        evidence: if fields.is_empty() {
            "No fixture memo or sensitive address appears in the nonempty application log.".into()
        } else {
            format!(
                "Plaintext {} in app.log matches the independently authored fixture.",
                fields.join(" + ")
            )
        },
        inference: if fields.is_empty() {
            "The named private fields are absent from the captured app log. Other log fields remain visible.".into()
        } else {
            "The application log reveals the encrypted payment's private content after legitimate local decryption.".into()
        },
        fix: "Redact memo and address fields before writing application logs or analytics.",
        observation_sequences: vec![],
    };
    Ok(RunResult {
        profile: w.mode.clone(),
        validated: true,
        findings: vec![a1, a3, a2],
        observation_count: input.observations.len(),
        transaction_fetches: fetches,
        wallet: w.clone(),
        app_log_excerpt: input.app_log.chars().take(2400).collect(),
        full_window_schedule,
        submitted_transactions,
    })
}

pub fn enforce(result: &RunResult, expected: &[String]) -> Result<()> {
    ensure!(result.validated, "evidence was not validated");
    let actual: BTreeSet<_> = result.leaked_ids().into_iter().collect();
    let expected_set: BTreeSet<_> = expected.iter().cloned().collect();
    ensure!(
        expected_set.len() == expected.len(),
        "duplicate expected leak id"
    );
    ensure!(
        expected
            .iter()
            .all(|id| matches!(id.as_str(), "A1" | "A2" | "A3")),
        "unknown expected leak id"
    );
    if actual != expected_set {
        bail!("expected findings {expected_set:?}, observed {actual:?}");
    }
    require_exercised(result, expected)
}

pub fn require_exercised(result: &RunResult, expected: &[String]) -> Result<()> {
    for id in expected {
        ensure!(
            result.findings.iter().any(|f| f.id == id && f.exercised),
            "expected {id} was not exercised"
        );
    }
    Ok(())
}

fn parse_application_log(log: &str) -> Result<Vec<Value>> {
    ensure!(
        !log.trim().is_empty(),
        "empty app log cannot prove the merchant ran"
    );
    log.lines()
        .enumerate()
        .map(|(index, line)| {
            ensure!(
                !line.trim().is_empty(),
                "blank application log line {}",
                index + 1
            );
            let event: Value = serde_json::from_str(line)
                .with_context(|| format!("invalid application JSON on line {}", index + 1))?;
            ensure!(
                event.is_object(),
                "application log line {} must be a JSON object",
                index + 1
            );
            Ok(event)
        })
        .collect()
}

fn string_values(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(items) => items.iter().flat_map(string_values).collect(),
        Value::Object(items) => items.values().flat_map(string_values).collect(),
        _ => vec![],
    }
}

struct TransparentPrefix {
    bytes: Vec<u8>,
    version: u32,
    inputs: usize,
    input_script_sizes: Vec<usize>,
    outputs: usize,
    output_values_zat: Vec<u64>,
    output_scripts_hex: Vec<String>,
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(text, "{byte:02x}").expect("writing to a string cannot fail");
    }
    text
}

/// Parse only the transparent vectors, not consensus proofs or signatures.
/// The original CompactSize, bounds, header, group, and locktime checks remain mandatory.
fn parse_transparent(raw: &str) -> Result<TransparentPrefix> {
    ensure!(
        raw.len().is_multiple_of(2) && raw.is_ascii(),
        "invalid transaction hex length"
    );
    let bytes = (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut cursor = Cursor {
        bytes: &bytes,
        pos: 0,
    };
    let header = cursor.u32()?;
    let version = header & 0x7fff_ffff;
    ensure!(
        (1..=5).contains(&version),
        "unsupported transaction version {version}"
    );
    if version >= 3 {
        ensure!(header & 0x8000_0000 != 0, "missing overwintered flag");
        let group = cursor.u32()?;
        let expected = match version {
            3 => 0x03c4_8270,
            4 => 0x892f_2085,
            5 => 0x26a7_270a,
            _ => unreachable!(),
        };
        ensure!(group == expected, "unexpected transaction version group");
    }
    if version == 5 {
        cursor.skip(12)?;
    }
    let inputs = cursor.count()?;
    let mut input_script_sizes = Vec::new();
    for _ in 0..inputs {
        cursor.skip(36)?;
        let size = cursor.count()?;
        input_script_sizes.push(size);
        cursor.skip(size)?;
        cursor.skip(4)?;
    }
    let outputs = cursor.count()?;
    let mut output_values_zat = Vec::new();
    let mut output_scripts_hex = Vec::new();
    for _ in 0..outputs {
        output_values_zat.push(u64::from_le_bytes(cursor.take(8)?.try_into()?));
        let size = cursor.count()?;
        output_scripts_hex.push(encode_hex(cursor.take(size)?));
    }
    if version < 5 {
        cursor.skip(4)?;
    }
    Ok(TransparentPrefix {
        bytes,
        version,
        inputs,
        input_script_sizes,
        outputs,
        output_values_zat,
        output_scripts_hex,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(n)
            .context("transaction offset overflow")?;
        ensure!(end <= self.bytes.len(), "truncated transaction");
        let result = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(result)
    }
    fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n)?;
        Ok(())
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn count(&mut self) -> Result<usize> {
        let first = self.take(1)?[0];
        let n = match first {
            253 => {
                let n = u16::from_le_bytes(self.take(2)?.try_into()?) as u64;
                ensure!(n >= 253, "noncanonical CompactSize");
                n
            }
            254 => {
                let n = self.u32()? as u64;
                ensure!(n > u16::MAX as u64, "noncanonical CompactSize");
                n
            }
            255 => {
                let n = u64::from_le_bytes(self.take(8)?.try_into()?);
                ensure!(n > u32::MAX as u64, "noncanonical CompactSize");
                n
            }
            n => n as u64,
        };
        ensure!(
            n <= self.bytes.len() as u64,
            "transaction vector exceeds available bytes"
        );
        Ok(n as usize)
    }
}
