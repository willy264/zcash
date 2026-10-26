//! Synthetic evidence tests for the capture boundary, not production demo traces.
use analyzer::{analyze, CapturedPair, GroundTruth, RunInput};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Lab {
    root: PathBuf,
    truth: GroundTruth,
    originals: BTreeMap<String, Vec<u8>>,
}

impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "darkroom-capture-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let memo = "订单 α </script><script>window.__injected=1</script> & \u{2028}\u{2029} reply:ztestsapling1private";
        let truth: GroundTruth = serde_json::from_value(json!({
            "payment":{"txid":"b".repeat(64),"height":104,"value_zat":25000000,"memo_plaintext":memo,"pool":"sapling"},
            "sensitive_txids":["b".repeat(64)], "sensitive_addresses":["ztestsapling1private"],
            "candidate_txids":["a".repeat(64),"b".repeat(64),"c".repeat(64)]
        })).unwrap();
        let mut originals = BTreeMap::new();
        for (profile, fetches, line_ending, trailing) in [
            ("leaky", &["b", "a", "b", "c"][..], "\r\n", true),
            ("fixed", &["a", "b", "c"][..], "\n", false),
        ] {
            fs::create_dir(root.join(profile)).unwrap();
            let mut observations = vec![
                " { \"args\": { \"end\": 110, \"start\": 100 }, \"unknown_metadata\": {\"opaque\": \"\\u0041\", \"unicode\": \"café 🧪\"}, \"method\" : \"GetBlockRange\", \"session\":\"subject-1\", \"seq\":1 }".to_owned()
            ];
            for (index, id) in fetches.iter().enumerate() {
                observations.push(format!(
                    "  {} ",
                    json!({
                        "seq":index+2,"session":"subject-1","method":"GetTransaction",
                        "args":{"txid":id.repeat(64)},"opaque_extra":"kept only in original bytes"
                    })
                ));
            }
            let mut observations = observations.join(line_ending);
            if trailing {
                observations.push_str(line_ending);
            }
            let app_log = format!(
                " {} {}",
                json!({"event":"order_reconciled",
                "memo":if profile == "leaky" {memo} else {"[REDACTED]"},
                "opaque_extra":{"spelling":"\\u0041","unicode":"雪"}}),
                line_ending
            );
            let mut wallet = serde_json::to_string_pretty(&json!({
                "mode":if profile == "leaky" {"leaky"} else {"good"},
                "scan_start":100,"scan_end":110,"compact_blocks_scanned":11,
                "candidate_txids":truth.candidate_txids,"merchant_deliveries":1,
                "notes":[{"txid":truth.payment.txid,"height":104,"value_zat":25000000,"memo":memo}],
                "memo_policy":"full-window","opaque_extra":{"raw":"\\u0041","unicode":"你好"}
            }))
            .unwrap()
            .replace('\n', line_ending);
            if trailing {
                wallet.push_str(line_ending);
            }
            for (name, bytes) in [
                ("observations.jsonl", observations.into_bytes()),
                ("app.log", app_log.into_bytes()),
                ("wallet.json", wallet.into_bytes()),
            ] {
                fs::write(root.join(profile).join(name), &bytes).unwrap();
                originals.insert(format!("{profile}-{name}"), bytes);
            }
        }
        Self {
            root,
            truth,
            originals,
        }
    }

    fn capture(&self) -> CapturedPair {
        CapturedPair::load(&self.root, &self.truth, vec!["A1".into(), "A3".into()]).unwrap()
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        // Only remove files in this test's uniquely created directories; no recursive deletion.
        for name in ["leaky", "fixed", "downloads"] {
            let path = self.root.join(name);
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let _ = fs::remove_file(entry.path());
                }
                let _ = fs::remove_dir(path);
            }
        }
        let _ = fs::remove_dir(&self.root);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn embedded_bytes_hashes_and_downloads_match_original_formatting_exactly() {
    let lab = Lab::new();
    let captured = lab.capture();
    assert!(captured.report().passed);
    let output = lab.root.join("downloads");
    captured.write_downloads(&output).unwrap();
    let payload: Value = serde_json::from_str(&captured.replay_json()).unwrap();
    assert_eq!(payload["schema_version"], 1);
    assert_eq!(
        payload["report"],
        serde_json::to_value(captured.report()).unwrap()
    );
    assert_eq!(payload["files"].as_object().unwrap().len(), 6);
    for (name, original) in &lab.originals {
        let file = &payload["files"][name];
        assert_eq!(
            STANDARD.decode(file["base64"].as_str().unwrap()).unwrap(),
            *original,
            "{name}"
        );
        assert_eq!(
            file["byte_length"].as_u64().unwrap(),
            original.len() as u64,
            "{name}"
        );
        assert_eq!(
            file["sha256"].as_str().unwrap(),
            hex(&Sha256::digest(original)),
            "{name}"
        );
        assert_eq!(fs::read(output.join(name)).unwrap(), *original, "{name}");
    }
    let observations = &lab.originals["leaky-observations.jsonl"];
    assert!(observations.ends_with(b"\r\n"));
    assert!(std::str::from_utf8(observations)
        .unwrap()
        .contains("\\u0041"));
    assert!(!lab.originals["fixed-observations.jsonl"].ends_with(b"\n"));
}

#[test]
fn changing_or_deleting_sources_after_capture_cannot_change_the_report_or_downloads() {
    let lab = Lab::new();
    let captured = lab.capture();
    let before = captured.replay_json();
    for name in ["observations.jsonl", "app.log", "wallet.json"] {
        fs::write(
            lab.root.join("leaky").join(name),
            b"replacement, not validated evidence",
        )
        .unwrap();
        fs::remove_file(lab.root.join("fixed").join(name)).unwrap();
    }
    assert!(CapturedPair::load(&lab.root, &lab.truth, vec!["A1".into(), "A3".into()]).is_err());
    assert_eq!(captured.replay_json(), before);
    assert!(captured.report().passed);
    let output = lab.root.join("downloads");
    captured.write_downloads(&output).unwrap();
    for (name, original) in &lab.originals {
        assert_eq!(fs::read(output.join(name)).unwrap(), *original, "{name}");
    }
}

#[test]
fn mutable_input_does_not_attach_stale_bytes_to_the_capture() {
    let lab = Lab::new();
    let captured = lab.capture();
    let before = captured.replay_json();
    let text = |name: &str| std::str::from_utf8(&lab.originals[name]).unwrap();
    let mut separate = RunInput::parse(
        text("leaky-observations.jsonl"),
        text("leaky-app.log"),
        text("leaky-wallet.json"),
    )
    .unwrap();
    separate.wallet.notes[0].value_zat += 1;
    separate.app_log = "{\"event\":\"listening\"}".into();
    assert!(analyze(&separate, &lab.truth).is_err());
    assert_eq!(captured.replay_json(), before);
    assert!(captured.report().passed);
}

#[test]
fn invalid_utf8_or_json_is_rejected_without_rewriting_the_evidence() {
    let lab = Lab::new();
    let path = lab.root.join("fixed/app.log");
    fs::write(&path, [0xff, 0xfe]).unwrap();
    let error =
        CapturedPair::load(&lab.root, &lab.truth, vec!["A1".into(), "A3".into()]).unwrap_err();
    assert!(format!("{error:#}").contains("not valid UTF-8"));
    assert_eq!(fs::read(&path).unwrap(), [0xff, 0xfe]);
    fs::write(&path, b"{broken JSON").unwrap();
    assert!(CapturedPair::load(&lab.root, &lab.truth, vec!["A1".into(), "A3".into()]).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"{broken JSON");
}

#[test]
fn replay_json_is_safe_in_an_inert_script_without_changing_the_captured_data() {
    let lab = Lab::new();
    let captured = lab.capture();
    let encoded = captured.replay_json();
    assert!(!encoded.contains(['<', '>', '&', '\u{2028}', '\u{2029}']));
    let decoded: Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        decoded["report"]["ground_truth"]["memo_plaintext"],
        lab.truth.payment.memo_plaintext
    );
    let app = STANDARD
        .decode(
            decoded["files"]["leaky-app.log"]["base64"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(app, lab.originals["leaky-app.log"]);
    assert!(std::str::from_utf8(&app)
        .unwrap()
        .contains("</script><script>"));
}
