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

// Complete unsigned V4 test transaction. Its SHA256d display txid was checked
// independently with Python hashlib; this input intentionally cannot move funds.
const FALLBACK_RAW: &str = "0400008085202f890144444444444444444444444444444444444444444444444444444444444444440000000000ffffffff01509f7d01000000001976a914555555555555555555555555555555555555555588ac00000000820000000000000000000000000000";
const FALLBACK_TXID: &str = "52c836366052c28b9d57fa30c7d081d5644c929a13e8a3eb9bf1da290343c9fc";

fn attempt(submitted: bool) -> serde_json::Value {
    json!({
        "requested_pool":"sapling", "requested_value_zat":25010000,
        "available_note_upper_bound_zat":25000000, "shielded_attempted":true,
        "shielded_failure":"insufficient_scanned_value", "failure_detail":"requested amount exceeds all scanned notes",
        "fallback_policy":if submitted {"transparent"} else {"refuse"},
        "outcome":if submitted {"transparent_submission_attempted"} else {"refused"},
        "fallback_raw_tx_hex":if submitted {Some(FALLBACK_RAW)} else {None},
        "submitted_txid":if submitted {Some(FALLBACK_TXID)} else {None},
        "transparent_destination":if submitted {Some("tmHVYzYLdw3nPgMxhcdZdpXCGQ9Mt9sVNSw")} else {None},
        "submission_result":if submitted {Some(json!({"rpc_completed":true,"error_code":0,"error_message":""}))} else {None},
        "consensus_valid":false,"funds_moved":false,
        "input_model":"nonexistent prevout; unsigned input"
    })
}

fn exercise_send(run: &mut RunInput, submitted: bool) {
    run.wallet.mode = if submitted { "leaky" } else { "good" }.into();
    run.wallet.send_attempt = Some(serde_json::from_value(attempt(submitted)).unwrap());
    if submitted {
        run.observations.push(
            serde_json::from_value(json!({"seq":run.observations.len()+1,
            "session":"subject-1","method":"SendTransaction",
            "args":{"raw_tx_hex":FALLBACK_RAW,"has_transparent_component":false}}))
            .unwrap(),
        );
    }
}

fn fallback_bytes() -> Vec<u8> {
    (0..FALLBACK_RAW.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&FALLBACK_RAW[offset..offset + 2], 16).unwrap())
        .collect()
}

/// Make mutated observations and subject metadata agree, so rejection must come
/// from structural validation rather than an incidental byte/txid mismatch.
fn replace_captured_fallback(run: &mut RunInput, bytes: &[u8]) {
    use sha2::{Digest, Sha256};
    let raw = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let txid = Sha256::digest(Sha256::digest(bytes))
        .iter()
        .rev()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let attempt = run.wallet.send_attempt.as_mut().unwrap();
    attempt.fallback_raw_tx_hex = Some(raw.clone());
    attempt.submitted_txid = Some(txid);
    run.observations
        .iter_mut()
        .find(|observation| observation.method == "SendTransaction")
        .unwrap()
        .args["raw_tx_hex"] = json!(raw);
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
            .arg("--expected-leaks")
            .arg("A1,A3")
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
    exercise_send(&mut run, true);
    let result = analyze(&run, &gt()).unwrap();
    assert_eq!(result.leaked_ids(), ["A2"]);
    assert!(require_exercised(&result, &["A2".into()]).is_ok());
    assert_eq!(result.submitted_transactions[0].raw_tx_hex, FALLBACK_RAW);
    assert_eq!(result.submitted_transactions[0].txid, FALLBACK_TXID);
    assert_eq!(
        result.submitted_transactions[0].output_values_zat,
        [25010000]
    );
    assert!(result
        .findings
        .iter()
        .find(|f| f.id == "A2")
        .unwrap()
        .inference
        .contains("acceptance is not established"));
}

#[test]
fn a2_fixed_refusal_is_exercised_only_with_an_actual_attempt_and_no_send() {
    let mut run = input(&["a", "b", "c"], false);
    exercise_send(&mut run, false);
    let result = analyze(&run, &gt()).unwrap();
    assert!(result.leaked_ids().is_empty());
    assert!(require_exercised(&result, &["A1".into(), "A3".into(), "A2".into()]).is_ok());
    assert!(result.submitted_transactions.is_empty());

    run.wallet.send_attempt.as_mut().unwrap().shielded_attempted = false;
    assert!(analyze(&run, &gt())
        .unwrap_err()
        .to_string()
        .contains("skipped"));
    run.wallet.send_attempt = None;
    let skipped = analyze(&run, &gt()).unwrap();
    assert!(require_exercised(&skipped, &["A2".into()]).is_err());
}

#[test]
fn a2_unknown_or_incomplete_attempts_are_rejected() {
    for field in [
        "outcome",
        "requested_pool",
        "shielded_failure",
        "fallback_policy",
    ] {
        let mut value = attempt(false);
        value[field] = json!("unknown");
        assert!(
            serde_json::from_value::<analyzer::SendAttempt>(value).is_err(),
            "{field}"
        );
    }
    let mut value = attempt(false);
    value.as_object_mut().unwrap().remove("shielded_attempted");
    assert!(serde_json::from_value::<analyzer::SendAttempt>(value).is_err());
    let mut run = input(&["a", "b", "c"], false);
    exercise_send(&mut run, true);
    run.wallet.send_attempt = None;
    assert!(analyze(&run, &gt())
        .unwrap_err()
        .to_string()
        .contains("send_attempt"));
}

#[test]
fn a2_refusal_cannot_hide_a_send_or_claim_a_constructed_replacement() {
    let mut run = input(&["a", "b", "c"], false);
    exercise_send(&mut run, true);
    run.wallet.mode = "good".into();
    run.wallet.send_attempt = Some(serde_json::from_value(attempt(false)).unwrap());
    assert!(analyze(&run, &gt())
        .unwrap_err()
        .to_string()
        .contains("refusal claimed"));
    run.observations.pop();
    run.wallet
        .send_attempt
        .as_mut()
        .unwrap()
        .fallback_raw_tx_hex = Some(FALLBACK_RAW.into());
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn a2_submission_requires_the_exact_captured_transaction_and_one_send() {
    let mut run = input(&["a", "b", "c"], false);
    exercise_send(&mut run, true);
    run.wallet
        .send_attempt
        .as_mut()
        .unwrap()
        .fallback_raw_tx_hex = Some(format!("{FALLBACK_RAW}00"));
    assert!(analyze(&run, &gt())
        .unwrap_err()
        .to_string()
        .contains("bytes differ"));
    run.wallet
        .send_attempt
        .as_mut()
        .unwrap()
        .fallback_raw_tx_hex = Some(FALLBACK_RAW.into());
    run.wallet.send_attempt.as_mut().unwrap().submitted_txid = Some("a".repeat(64));
    assert!(analyze(&run, &gt())
        .unwrap_err()
        .to_string()
        .contains("txid does not match"));
    run.wallet.send_attempt.as_mut().unwrap().submitted_txid = Some(FALLBACK_TXID.into());
    let captured = run.observations.pop().unwrap();
    assert!(analyze(&run, &gt()).is_err());
    run.observations.push(captured.clone());
    run.observations.push(captured);
    assert!(analyze(&run, &gt()).is_err());
}

#[test]
fn a2_failure_amount_destination_and_rpc_result_must_be_consistent() {
    for (field, value) in [
        ("requested_value_zat", json!(25000000)),
        ("requested_value_zat", json!(25020000)),
        ("available_note_upper_bound_zat", json!(24999999)),
        ("transparent_destination", json!("invented")),
        ("consensus_valid", json!(true)),
        ("funds_moved", json!(true)),
        ("input_model", json!("funded and signed input")),
        ("failure_detail", json!("")),
        (
            "submission_result",
            json!({"rpc_completed":false,"error_code":0,"error_message":"failure"}),
        ),
    ] {
        let mut run = input(&["a", "b", "c"], false);
        exercise_send(&mut run, true);
        let mut evidence = attempt(true);
        evidence[field] = value;
        run.wallet.send_attempt = Some(serde_json::from_value(evidence).unwrap());
        assert!(analyze(&run, &gt()).is_err(), "{field}");
    }
}

#[test]
fn a2_parser_rejects_noncanonical_and_truncated_inputs_before_attempt_checks() {
    for raw in ["0400008085202f89fd0100", "0400008085202f890144", "05000080"] {
        let mut run = input(&["a", "b", "c"], false);
        run.observations.push(serde_json::from_value(json!({"seq":5,"session":"subject-1","method":"SendTransaction","args":{"raw_tx_hex":raw}})).unwrap());
        assert!(analyze(&run, &gt())
            .unwrap_err()
            .to_string()
            .contains("cannot parse submitted transaction"));
    }
}

#[test]
fn a2_rejects_every_truncated_v4_tail_even_with_matching_bytes_and_rehashed_txid() {
    let bytes = fallback_bytes();
    assert_eq!(bytes.len(), 104);
    // Byte 89 is just past locktime, where the generic prefix parser can finish.
    // The expiry, Sapling value balance, and three empty vectors are still required.
    for end in 89..bytes.len() {
        let mut run = input(&["a", "b", "c"], false);
        exercise_send(&mut run, true);
        replace_captured_fallback(&mut run, &bytes[..end]);
        let error = analyze(&run, &gt()).unwrap_err();
        assert!(
            format!("{error:#}").contains("truncated transaction"),
            "tail ends at {end}: {error:#}"
        );
    }
}

#[test]
fn a2_rejects_extra_tail_wrong_prevout_index_and_signed_input_after_rehashing() {
    let original = fallback_bytes();
    let mut extra = original.clone();
    extra.push(0);
    let mut prevout = original.clone();
    prevout[9] ^= 1;
    let mut index = original.clone();
    index[41] = 1;
    let mut signed = original.clone();
    signed[45] = 1;
    signed.insert(46, 0x51);
    for (bytes, expected) in [
        (extra, "unexpected trailing bytes"),
        (prevout, "known nonexistent lab prevout"),
        (index, "prevout index must be zero"),
        (signed, "empty scriptSig"),
    ] {
        let mut run = input(&["a", "b", "c"], false);
        exercise_send(&mut run, true);
        replace_captured_fallback(&mut run, &bytes);
        let error = analyze(&run, &gt()).unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }
}

#[test]
fn a2_requires_the_declared_expiry_empty_shielded_bundles_and_final_unsigned_model() {
    for (offset, value, expected) in [
        (46, 0xfe, "sequence must be final"),
        (85, 1, "locktime must be zero"),
        (89, 0, "expiry height"),
        (93, 1, "zero Sapling value balance"),
        (101, 1, "no Sapling spends"),
        (102, 1, "no Sapling outputs"),
        (103, 1, "no JoinSplits"),
    ] {
        let mut bytes = fallback_bytes();
        bytes[offset] = value;
        let mut run = input(&["a", "b", "c"], false);
        exercise_send(&mut run, true);
        replace_captured_fallback(&mut run, &bytes);
        let error = analyze(&run, &gt()).unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
    }
    let mut bytes = fallback_bytes();
    bytes.splice(101..102, [253, 0, 0]);
    let mut run = input(&["a", "b", "c"], false);
    exercise_send(&mut run, true);
    replace_captured_fallback(&mut run, &bytes);
    let error = analyze(&run, &gt()).unwrap_err();
    assert!(format!("{error:#}").contains("noncanonical CompactSize"));
}

#[test]
fn three_check_report_uses_captured_a2_data_and_same_send_request() {
    let mut leaky = input(&["b", "a", "b", "c"], true);
    let mut fixed = input(&["a", "b", "c"], false);
    exercise_send(&mut leaky, true);
    exercise_send(&mut fixed, false);
    let expected = vec!["A1".into(), "A3".into(), "A2".into()];
    let report = PairReport::new(
        &gt(),
        analyze(&leaky, &gt()).unwrap(),
        analyze(&fixed, &gt()).unwrap(),
        expected.clone(),
    );
    assert!(report.passed);
    assert_eq!(report.exercised_checks, ["A1", "A3", "A2"]);
    let html = render_report(&report);
    assert!(html.contains("id=\"a2\""));
    assert!(html.contains(FALLBACK_RAW));
    assert!(html.contains("SendTransaction requests: 0"));
    assert!(html.contains("LOCAL SUBJECT EVIDENCE"));
    assert!(html.contains("acceptance is not established"));
    fixed
        .wallet
        .send_attempt
        .as_mut()
        .unwrap()
        .requested_value_zat += 1;
    assert!(
        !PairReport::new(
            &gt(),
            analyze(&leaky, &gt()).unwrap(),
            analyze(&fixed, &gt()).unwrap(),
            expected
        )
        .passed
    );
}
