use crate::{
    encode_hex, parse_transparent, valid_txid, Cursor, FallbackPolicy, Finding, RunInput,
    SendAttempt, SendOutcome, SubmittedTransaction,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

const INPUT_MODEL: &str = "nonexistent prevout; unsigned input";
const FIX: &str = "Refuse a shielded send when shielded funds or support are insufficient; never silently construct a transparent replacement.";

/// A2's model is a complete transparent-only V4 transaction, not just a prefix.
/// This narrow validator complements (and does not relax) the generic parser.
/// All CompactSize values still pass through its canonical, bounded decoder.
fn validate_a2_v4(bytes: &[u8], scan_end: u64, output_value_zat: u64) -> Result<()> {
    let mut cursor = Cursor { bytes, pos: 0 };
    ensure!(
        cursor.u32()? == 0x8000_0004,
        "A2 requires a V4 overwintered transaction"
    );
    ensure!(
        cursor.u32()? == 0x892f_2085,
        "A2 requires the Sapling V4 version group"
    );
    ensure!(cursor.count()? == 1, "A2 requires one transparent input");
    ensure!(
        cursor.take(32)? == [0x44; 32],
        "A2 input must reference the known nonexistent lab prevout"
    );
    ensure!(cursor.u32()? == 0, "A2 lab prevout index must be zero");
    ensure!(
        cursor.count()? == 0,
        "A2 lab input must remain unsigned with an empty scriptSig"
    );
    ensure!(
        cursor.u32()? == u32::MAX,
        "A2 lab input sequence must be final"
    );
    ensure!(cursor.count()? == 1, "A2 requires one transparent output");
    ensure!(
        u64::from_le_bytes(cursor.take(8)?.try_into()?) == output_value_zat,
        "A2 output amount disagrees with the shielded request"
    );
    let script_size = cursor.count()?;
    cursor.skip(script_size)?; // Independently matched to the Base58Check destination below.
    ensure!(
        cursor.u32()? == 0,
        "A2 lab transaction locktime must be zero"
    );
    let expected_expiry = u32::try_from(scan_end)?
        .checked_add(20)
        .context("A2 expiry height overflow")?;
    ensure!(
        cursor.u32()? == expected_expiry,
        "A2 expiry height must equal the scan tip plus 20"
    );
    ensure!(
        i64::from_le_bytes(cursor.take(8)?.try_into()?) == 0,
        "A2 transparent-only transaction must have zero Sapling value balance"
    );
    ensure!(
        cursor.count()? == 0,
        "A2 transparent-only transaction must have no Sapling spends"
    );
    ensure!(
        cursor.count()? == 0,
        "A2 transparent-only transaction must have no Sapling outputs"
    );
    ensure!(
        cursor.count()? == 0,
        "A2 transparent-only transaction must have no JoinSplits"
    );
    ensure!(
        cursor.pos == bytes.len(),
        "A2 transaction has unexpected trailing bytes"
    );
    Ok(())
}

pub(super) fn same_send_scenario(
    before: &Option<SendAttempt>,
    after: &Option<SendAttempt>,
) -> bool {
    match (before, after) {
        (None, None) => true,
        (Some(before), Some(after)) => {
            before.requested_pool == after.requested_pool
                && before.requested_value_zat == after.requested_value_zat
                && before.available_note_upper_bound_zat == after.available_note_upper_bound_zat
                && before.shielded_failure == after.shielded_failure
                && before.input_model == after.input_model
        }
        _ => false,
    }
}

pub(super) fn analyze_send(input: &RunInput) -> Result<(Finding, Vec<SubmittedTransaction>)> {
    let sends: Vec<_> = input
        .observations
        .iter()
        .filter(|observation| observation.method == "SendTransaction")
        .collect();
    // Parse every observed send first. Missing attempt metadata must never bypass
    // malformed transaction/CompactSize checks, even in the optional two-check scope.
    let parsed = sends
        .iter()
        .map(|send| {
            let raw = send
                .args
                .get("raw_tx_hex")
                .and_then(Value::as_str)
                .context("SendTransaction lacks raw transaction evidence")?;
            parse_transparent(raw).context("cannot parse submitted transaction transparency")
        })
        .collect::<Result<Vec<_>>>()?;
    let Some(attempt) = &input.wallet.send_attempt else {
        ensure!(
            sends.is_empty(),
            "SendTransaction requires typed shielded send_attempt evidence"
        );
        return Ok((
            Finding {
                id: "A2",
                title: "Silent transparent fallback",
                exercised: false,
                leaked: false,
                evidence: "Not exercised: no typed shielded-send attempt was recorded.".into(),
                inference: "No claim is made about transparent-fallback prevention.".into(),
                fix: FIX,
                observation_sequences: vec![],
            },
            vec![],
        ));
    };
    ensure!(
        attempt.shielded_attempted,
        "A2 shielded send was skipped, not exercised"
    );
    ensure!(
        !attempt.failure_detail.trim().is_empty(),
        "A2 shielded failure detail is missing"
    );
    let available = input.wallet.notes.iter().try_fold(0u64, |total, note| {
        total
            .checked_add(note.value_zat)
            .context("scanned note value overflow")
    })?;
    ensure!(
        attempt.available_note_upper_bound_zat == available,
        "A2 claimed available note upper bound disagrees with decrypted wallet notes"
    );
    ensure!(
        attempt.requested_value_zat > available,
        "A2 request does not exceed the scanned shielded value; failure is not established"
    );
    ensure!(
        !attempt.consensus_valid && !attempt.funds_moved,
        "A2 must not claim consensus validity or funds moved for the unsigned test transaction"
    );
    ensure!(
        attempt.input_model == INPUT_MODEL,
        "A2 input model is unknown or mislabeled"
    );

    match attempt.outcome {
        SendOutcome::Refused => {
            ensure!(
                input.wallet.mode == "good" && attempt.fallback_policy == FallbackPolicy::Refuse,
                "A2 refusal outcome disagrees with wallet mode or fallback policy"
            );
            ensure!(
                sends.is_empty(),
                "A2 refusal claimed but SendTransaction was observed"
            );
            ensure!(
                attempt.fallback_raw_tx_hex.is_none()
                    && attempt.submitted_txid.is_none()
                    && attempt.transparent_destination.is_none()
                    && attempt.submission_result.is_none(),
                "A2 refusal must not include a constructed or submitted fallback transaction"
            );
            Ok((Finding {
                id: "A2", title: "Silent transparent fallback", exercised: true, leaked: false,
                evidence: format!("Zero SendTransaction requests were observed. Subject evidence records a shielded-send attempt for {} zatoshi against a {}-zatoshi scanned-note upper bound, followed by explicit refusal.", attempt.requested_value_zat, available),
                inference: "The refused send produces no transparent submission in this captured run. Local refusal evidence proves this path ran; absence of a request alone would not establish that.".into(),
                fix: FIX, observation_sequences: vec![],
            }, vec![]))
        }
        SendOutcome::TransparentSubmissionAttempted => {
            ensure!(
                input.wallet.mode == "leaky"
                    && attempt.fallback_policy == FallbackPolicy::Transparent,
                "A2 submission outcome disagrees with wallet mode or fallback policy"
            );
            ensure!(
                sends.len() == 1,
                "A2 transparent fallback requires exactly one captured SendTransaction"
            );
            let constructed = attempt
                .fallback_raw_tx_hex
                .as_deref()
                .context("A2 submission lacks constructed fallback bytes")?;
            let constructed = parse_transparent(constructed)
                .context("invalid A2 constructed fallback transaction")?;
            let captured = &parsed[0];
            ensure!(
                constructed.bytes == captured.bytes,
                "A2 constructed fallback bytes differ from the captured SendTransaction bytes"
            );
            validate_a2_v4(
                &captured.bytes,
                input.wallet.scan_end,
                attempt.requested_value_zat,
            )
            .context("A2 fallback does not match the complete unsigned V4 test model")?;
            ensure!(
                captured.inputs > 0 || captured.outputs > 0,
                "A2 fallback bytes have no transparent component"
            );
            ensure!(
                captured.version == 4
                    && captured.inputs == 1
                    && captured.outputs == 1
                    && captured.input_script_sizes == [0],
                "A2 test input model requires one unsigned V4 input and one transparent output"
            );
            ensure!(
                captured.output_values_zat == [attempt.requested_value_zat],
                "A2 submitted output amount does not match the failed shielded-send request"
            );
            let destination = attempt
                .transparent_destination
                .as_deref()
                .context("A2 fallback destination is missing")?;
            ensure!(
                !destination.trim().is_empty(),
                "A2 fallback destination is empty"
            );
            let address = bs58::decode(destination)
                .with_check(None)
                .into_vec()
                .context("A2 fallback destination has invalid Base58Check encoding")?;
            // Zcash testnet P2PKH prefix (zcash_address/constants/testnet.rs).
            ensure!(
                address.len() == 22 && address[..2] == [0x1d, 0x25],
                "A2 fixture destination must be a testnet P2PKH address"
            );
            let expected_script = format!("76a914{}88ac", encode_hex(&address[2..]));
            ensure!(
                captured.output_scripts_hex == [expected_script],
                "A2 submitted output script does not match the fallback destination"
            );
            let digest = Sha256::digest(Sha256::digest(&captured.bytes));
            let txid = encode_hex(&digest.iter().rev().copied().collect::<Vec<_>>());
            let claimed_txid = attempt
                .submitted_txid
                .as_deref()
                .context("A2 submitted txid is missing")?;
            ensure!(
                valid_txid(claimed_txid) && claimed_txid == txid,
                "A2 submitted txid does not match the captured V4 transaction bytes"
            );
            let result = attempt
                .submission_result
                .as_ref()
                .context("A2 submission result is missing")?;
            ensure!(
                result.rpc_completed == result.error_code.is_some(),
                "A2 RPC completion and SendResponse error code disagree"
            );
            ensure!(
                result.rpc_completed || !result.error_message.trim().is_empty(),
                "A2 incomplete RPC lacks its transport failure detail"
            );
            let sequence = sends[0].seq;
            Ok((Finding {
                id: "A2", title: "Silent transparent fallback", exercised: true, leaked: true,
                evidence: format!("SendTransaction at observation #{sequence} carries a transparent V4 transaction with one unsigned input and an output of {} zatoshi. Captured bytes and txid {txid} match the subject's constructed fallback after its shielded send was refused.", attempt.requested_value_zat),
                inference: "The submitted bytes disclose a transparent output amount and destination script to the server. The input references a nonexistent prevout and is unsigned. Transaction acceptance is not established; mining and public-chain acceptance are not established by this request.".into(),
                fix: FIX, observation_sequences: vec![sequence],
            }, vec![SubmittedTransaction {
                observation_sequence: sequence, txid, raw_tx_hex: encode_hex(&captured.bytes),
                transparent_inputs: captured.inputs, transparent_outputs: captured.outputs,
                output_values_zat: captured.output_values_zat.clone(),
                output_scripts_hex: captured.output_scripts_hex.clone(),
            }]))
        }
    }
}
