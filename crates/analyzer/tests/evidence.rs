use analyzer::{
    analyze, enforce, render_report, require_exercised, GroundTruth, PairReport, RunInput,
};
use serde_json::json;

fn gt() -> GroundTruth {
    serde_json::from_value(json!({
        "payment": {"txid": "b".repeat(64), "height": 104, "value_zat": 25000000,
            "memo_plaintext": "ORDER-7F3A reply:ztestsapling1private", "pool": "sapling"},
        "sensitive_txids": ["b".repeat(64)],
        "sensitive_addresses": ["ztestsapling1private"],
        "candidate_txids": ["a".repeat(64), "b".repeat(64), "c".repeat(64)]
    }))
    .unwrap()
}

fn evidence(fetches: &[&str], leaky_app: bool) -> (String, String, String) {
    let truth = gt();
    let mut obs = vec![
        json!({"seq":1,"session":"subject-1","method":"GetBlockRange","args":{"start":100,"end":110}}),
    ];
    for (index, tx) in fetches.iter().enumerate() {
        obs.push(json!({"seq":index+2,"session":"subject-1","method":"GetTransaction","args":{"txid":tx.repeat(64)}}));
    }
    let app = json!({"event":"order_reconciled", "memo": if leaky_app { truth.payment.memo_plaintext.clone() } else { "[REDACTED]".into() }}).to_string();
    let wallet = json!({"mode": if leaky_app {"leaky"} else {"good"}, "scan_start":100,"scan_end":110,"compact_blocks_scanned":11,"candidate_txids":truth.candidate_txids,"merchant_deliveries":1,
        "notes":[{"txid":truth.payment.txid,"height":104,"value_zat":25000000,"memo":truth.payment.memo_plaintext}], "memo_policy":"full-window"}).to_string();
    (
        obs.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        app,
        wallet,
    )
}

fn input(fetches: &[&str], leaky_app: bool) -> RunInput {
    let (obs, app, wallet) = evidence(fetches, leaky_app);
    RunInput::parse(&obs, &app, &wallet).unwrap()
}

#[test]
fn real_expectations_require_both_the_detector_and_fix() {
    let leaky = analyze(&input(&["b", "a", "b", "c"], true), &gt()).unwrap();
    assert_eq!(leaky.leaked_ids(), ["A1", "A3"]);
    assert!(enforce(&leaky, &["A1".into(), "A3".into()]).is_ok());
    assert!(enforce(&leaky, &[]).is_err());
    let fixed = analyze(&input(&["a", "b", "c"], false), &gt()).unwrap();
    assert!(fixed.leaked_ids().is_empty());
    assert!(fixed.full_window_schedule);
    assert!(enforce(&fixed, &[]).is_ok());
    assert!(enforce(&fixed, &["A1".into(), "A3".into()]).is_err());
    assert!(
        !fixed
            .findings
            .iter()
            .find(|f| f.id == "A2")
            .unwrap()
            .exercised
    );
}

#[test]
fn missing_or_broken_evidence_never_means_clean() {
    let (obs, app, wallet) = evidence(&["a", "b", "c"], false);
    assert!(RunInput::parse("", &app, &wallet).is_err());
    assert!(RunInput::parse(&obs, "", &wallet).is_err());
    assert!(RunInput::parse(&obs, &app, "").is_err());
    assert!(RunInput::parse(&(obs.clone() + "\nnot json"), &app, &wallet).is_err());
    assert!(RunInput::parse(&obs.replace("\"seq\":2", "\"seq\":9"), &app, &wallet).is_err());
}

#[test]
fn startup_only_app_log_does_not_exercise_redaction() {
    let mut run = input(&["a", "b", "c"], false);
    run.app_log = "{\"event\":\"listening\"}".into();
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn malformed_private_data_after_valid_reconciliation_is_rejected() {
    let (obs, app, wallet) = evidence(&["a", "b", "c"], false);
    let broken = format!("{app}\n{{\"memo\":\"\\u004fRDER-7F3A reply:\\u007atestsapling1private\"");
    assert!(RunInput::parse(&obs, &broken, &wallet).is_err());
    let mut run = input(&["a", "b", "c"], false);
    run.app_log = broken;
    assert!(analyze(&run, &gt()).is_err());
    for suffix in ["\n\n", "\n[]", "\nnull", "\n\"not an event\""] {
        assert!(RunInput::parse(&obs, &(app.clone() + suffix), &wallet).is_err());
    }
}

#[test]
fn failed_direct_cli_rerun_invalidates_old_green_entrypoints() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let name = format!(
        "darkroom-analyzer-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(name);
    let output = root.join("report");
    let truth = gt();
    fs::create_dir_all(&root).unwrap();
    let truth_path = root.join("ground_truth.json");
    fs::write(
        &truth_path,
        json!({"payment":truth.payment,"sensitive_txids":truth.sensitive_txids,
        "sensitive_addresses":truth.sensitive_addresses,"candidate_txids":truth.candidate_txids})
        .to_string(),
    )
    .unwrap();
    for (profile, fetches, leaky) in [
        ("leaky", &["b", "a", "b", "c"][..], true),
        ("fixed", &["a", "b", "c"][..], false),
    ] {
        let directory = root.join(profile);
        fs::create_dir_all(&directory).unwrap();
        let (obs, app, wallet) = evidence(fetches, leaky);
        for (file, content) in [
            ("observations.jsonl", obs),
            ("app.log", app),
            ("wallet.json", wallet),
        ] {
            fs::write(directory.join(file), content).unwrap();
        }
    }
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_analyzer"))
            .arg("--runs")
            .arg(&root)
            .arg("--ground-truth")
            .arg(&truth_path)
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let initial = invoke();
    assert!(
        initial.status.success(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );
    assert!(output.join("index.html").is_file());
    let old: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("result.json")).unwrap()).unwrap();
    assert_eq!(old["passed"], true);

    fs::write(root.join("fixed/app.log"), "{invalid private data").unwrap();
    assert!(!invoke().status.success());
    assert!(!output.join("index.html").exists());
    assert!(!output.join("result.json").exists());

    // Ground-truth parse failures occur even earlier than input-log validation.
    fs::write(output.join("index.html"), "old green").unwrap();
    fs::write(output.join("result.json"), "{\"passed\":true}").unwrap();
    fs::write(&truth_path, "{").unwrap();
    assert!(!invoke().status.success());
    assert!(!output.join("index.html").exists());
    assert!(!output.join("result.json").exists());

    // All paths below are inside this test's unique, explicitly created directory.
    for directory in [root.join("leaky"), root.join("fixed"), output] {
        for entry in fs::read_dir(&directory).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::remove_dir(directory).unwrap();
    }
    fs::remove_file(truth_path).unwrap();
    fs::remove_dir(root).unwrap();
}

#[test]
fn inconsistent_note_or_range_is_rejected() {
    let mut run = input(&["a", "b", "c"], false);
    run.wallet.notes[0].value_zat += 1;
    assert!(analyze(&run, &gt()).is_err());
    run.wallet.notes[0].value_zat -= 1;
    run.wallet.compact_blocks_scanned -= 1;
    assert!(analyze(&run, &gt()).is_err());
    run.wallet.compact_blocks_scanned += 1;
    run.wallet.notes[0].memo = Some("invented".into());
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn missing_delivery_or_changed_public_candidates_fail_validation() {
    let mut run = input(&["a", "b", "c"], false);
    run.wallet.merchant_deliveries = 0;
    assert!(analyze(&run, &gt()).is_err());
    run.wallet.merchant_deliveries = 1;
    run.wallet.candidate_txids.swap(0, 1);
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn extra_sensitive_lookup_or_sensitive_first_order_still_leaks() {
    for fetches in [&["a", "b", "c", "b"][..], &["b", "a", "c"][..]] {
        let result = analyze(&input(fetches, false), &gt()).unwrap();
        assert_eq!(result.leaked_ids(), ["A1"]);
    }
}

#[test]
fn unrelated_transaction_is_not_called_a_sensitive_link() {
    let mut run = input(&["a"], false);
    run.wallet.memo_policy = "none".into();
    let result = analyze(&run, &gt()).unwrap();
    assert!(result.leaked_ids().is_empty());
    assert!(!result.full_window_schedule);
}

#[test]
fn claims_of_complete_memo_retrieval_need_complete_request_evidence() {
    assert!(analyze(&input(&["a"], false), &gt()).is_err());
}

#[test]
fn fixed_run_cannot_skip_the_exercised_app_check() {
    let mut run = input(&["a", "b", "c"], false);
    run.wallet.memo_policy = "none".into();
    let result = analyze(&run, &gt()).unwrap();
    assert!(enforce(&result, &[]).is_ok());
    assert!(require_exercised(&result, &["A1".into(), "A3".into()]).is_err());
}

#[test]
fn address_arrays_are_examined_and_sessions_cannot_pool_cover() {
    let mut run = input(&["a", "b", "c"], false);
    run.observations.push(serde_json::from_value(json!({"seq":5,"session":"subject-1","method":"GetAddressUtxos","args":{"addresses":["ztestsapling1private"]}})).unwrap());
    assert_eq!(analyze(&run, &gt()).unwrap().leaked_ids(), ["A1"]);
    run.observations[1].session = "another-wallet".into();
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn json_escaped_private_fields_are_decoded_before_search() {
    let mut run = input(&["a", "b", "c"], false);
    run.app_log =
        "{\"event\":\"order_reconciled\",\"memo\":\"ORDER-7F3A reply:\\u007atestsapling1private\"}"
            .into();
    assert_eq!(analyze(&run, &gt()).unwrap().leaked_ids(), ["A3"]);
}

#[test]
fn untrusted_memo_is_escaped_in_every_html_context() {
    let mut truth = gt();
    truth.payment.memo_plaintext = "<script>alert('x')</script>&\" {{CI_LINK}}".into();
    let mut leaky_input = input(&["b", "a", "b", "c"], true);
    leaky_input.wallet.notes[0].memo = Some(truth.payment.memo_plaintext.clone());
    leaky_input.app_log =
        json!({"event":"order_reconciled","memo":truth.payment.memo_plaintext}).to_string();
    let mut fixed_input = input(&["a", "b", "c"], false);
    fixed_input.wallet.notes[0].memo = Some(truth.payment.memo_plaintext.clone());
    let report = PairReport::new(
        &truth,
        analyze(&leaky_input, &truth).unwrap(),
        analyze(&fixed_input, &truth).unwrap(),
        vec!["A1".into(), "A3".into()],
    );
    let html = render_report(&report);
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("{{CI_LINK}}"));
    assert!(html.contains("NOT EXERCISED"));
    assert!(report.passed);
}

#[test]
fn broadcast_with_bad_bytes_is_an_error_not_a_clean_transparent_check() {
    let mut run = input(&["a", "b", "c"], false);
    run.observations.push(serde_json::from_value(json!({"seq":5,"session":"subject-1","method":"SendTransaction","args":{"raw_tx_hex":"05000080"}})).unwrap());
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn transparent_output_is_detected_from_bytes_independent_of_proxy_boolean() {
    let mut run = input(&["a", "b", "c"], false);
    // V1, no inputs, one 1-zat output with empty script, locktime zero.
    let raw = "01000000000101000000000000000000000000";
    run.observations.push(serde_json::from_value(json!({"seq":5,"session":"subject-1","method":"SendTransaction","args":{"raw_tx_hex":raw,"has_transparent_component":false}})).unwrap());
    assert_eq!(analyze(&run, &gt()).unwrap().leaked_ids(), ["A2"]);
}
