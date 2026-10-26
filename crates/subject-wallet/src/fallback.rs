//! An executable privacy-policy regression using an unsigned, unfunded lab transaction.
//! Nothing here can spend the shielded fixture or authorize a transparent input.

use anyhow::{Context, Result};
use serde::Serialize;
use std::future::Future;
use zcash_keys::encoding::encode_transparent_address_p;
use zcash_primitives::transaction::{Authorized, TransactionData, TxVersion};
use zcash_protocol::{
    consensus::{BlockHeight, BranchId, Network},
    value::Zatoshis,
};
use zcash_transparent::{
    address::{Script, TransparentAddress},
    bundle::{self, OutPoint, TxIn, TxOut},
};

use crate::ReceivedNote;

pub const INPUT_MODEL: &str = "nonexistent prevout; unsigned input";
pub const REFUSAL_EXIT_CODE: u8 = 3;
pub const NONEXISTENT_PREVOUT: [u8; 32] = [0x44; 32];
const LAB_DESTINATION_HASH: [u8; 20] = [0x55; 20];
const REQUEST_EXCESS_ZAT: u64 = 10_000;

#[derive(Debug, PartialEq, Eq)]
pub struct ShieldedFailure {
    pub code: &'static str,
    pub message: String,
}

/// Performs the subject's actual local shielded-send admission policy.
/// Received note values are only an upper bound, not a claim of spendable balance.
pub fn attempt_shielded_send(
    requested_value_zat: u64,
    available_note_upper_bound_zat: u64,
) -> std::result::Result<(), ShieldedFailure> {
    if requested_value_zat > available_note_upper_bound_zat {
        return Err(ShieldedFailure {
            code: "insufficient_scanned_value",
            message: format!(
                "Shielded request for {requested_value_zat} zatoshis exceeds the scanned-note upper bound of {available_note_upper_bound_zat} zatoshis; no shielded transaction was built."
            ),
        });
    }
    Err(ShieldedFailure {
        code: "shielded_spending_unavailable",
        message: "This receive-only subject has no shielded spend builder or authenticated spendable state.".into(),
    })
}

#[derive(Debug)]
pub struct TransparentTestTransaction {
    pub bytes: Vec<u8>,
    pub txid: String,
    pub destination: String,
}

/// Constructs the entire V4 serialization through librustzcash, including the
/// transparent input/output and the absent Sapling/JoinSplit bundle encodings.
/// `Authorized` is a serialization marker here: the empty scriptSig is deliberately
/// UNSIGNED, and the prevout does not occur in the staged fixture chain.
pub fn build_transparent_test_transaction(
    output_value_zat: u64,
    height: u64,
) -> Result<TransparentTestTransaction> {
    let height = BlockHeight::from(u32::try_from(height)?);
    let expiry = BlockHeight::from(
        u32::from(height)
            .checked_add(20)
            .context("expiry height overflow")?,
    );
    let address = TransparentAddress::PublicKeyHash(LAB_DESTINATION_HASH);
    let transparent = bundle::Bundle {
        vin: vec![TxIn::from_parts(
            OutPoint::new(NONEXISTENT_PREVOUT, 0),
            Script::default(),
            u32::MAX,
        )],
        vout: vec![TxOut::new(
            Zatoshis::from_u64(output_value_zat).context("transparent test amount out of range")?,
            address.script().into(),
        )],
        authorization: bundle::Authorized,
    };
    let transaction = TransactionData::<Authorized>::from_parts(
        TxVersion::V4,
        BranchId::for_height(&Network::TestNetwork, height),
        0,
        expiry,
        Some(transparent),
        None,
        None,
        None,
    )
    .freeze()
    .context("serialize complete transparent test transaction")?;
    let mut bytes = Vec::new();
    transaction.write(&mut bytes)?;
    Ok(TransparentTestTransaction {
        bytes,
        txid: transaction.txid().to_string(),
        destination: encode_transparent_address_p(&Network::TestNetwork, &address),
    })
}

#[derive(Debug)]
pub struct SubmissionReply {
    pub error_code: i32,
    pub error_message: String,
}

#[derive(Debug, Serialize)]
pub struct SubmissionResult {
    pub rpc_completed: bool,
    pub error_code: Option<i32>,
    pub error_message: String,
}

#[derive(Debug, Serialize)]
pub struct SendAttempt {
    pub requested_pool: &'static str,
    pub requested_value_zat: u64,
    pub available_note_upper_bound_zat: u64,
    pub shielded_attempted: bool,
    pub shielded_failure: &'static str,
    pub failure_detail: String,
    pub fallback_policy: &'static str,
    pub outcome: &'static str,
    pub fallback_raw_tx_hex: Option<String>,
    pub submitted_txid: Option<String>,
    pub transparent_destination: Option<String>,
    pub submission_result: Option<SubmissionResult>,
    pub consensus_valid: bool,
    pub funds_moved: bool,
    pub input_model: &'static str,
}

/// Runs the failing shielded request first, then enforces the fallback policy.
/// The provided submission function is the live gRPC client in the CLI; tests
/// capture its real serialized argument to prove the branch actually calls it.
pub async fn exercise_fallback<F, Fut>(
    notes: &[ReceivedNote],
    height: u64,
    refuse_transparent_fallback: bool,
    submit: F,
) -> Result<SendAttempt>
where
    F: FnOnce(Vec<u8>) -> Fut,
    Fut: Future<Output = Result<SubmissionReply>>,
{
    let available_note_upper_bound_zat = notes.iter().try_fold(0u64, |total, note| {
        total
            .checked_add(note.value_zat)
            .context("scanned value overflow")
    })?;
    let requested_value_zat = available_note_upper_bound_zat
        .checked_add(REQUEST_EXCESS_ZAT)
        .context("request value overflow")?;
    let failure = attempt_shielded_send(requested_value_zat, available_note_upper_bound_zat)
        .err()
        .context("overfunded shielded request unexpectedly succeeded")?;
    let mut evidence = SendAttempt {
        requested_pool: "sapling",
        requested_value_zat,
        available_note_upper_bound_zat,
        shielded_attempted: true,
        shielded_failure: failure.code,
        failure_detail: failure.message,
        fallback_policy: if refuse_transparent_fallback {
            "refuse"
        } else {
            "transparent"
        },
        outcome: "refused",
        fallback_raw_tx_hex: None,
        submitted_txid: None,
        transparent_destination: None,
        submission_result: None,
        consensus_valid: false,
        funds_moved: false,
        input_model: INPUT_MODEL,
    };
    if refuse_transparent_fallback {
        return Ok(evidence);
    }
    let transaction = build_transparent_test_transaction(requested_value_zat, height)?;
    evidence.outcome = "transparent_submission_attempted";
    evidence.fallback_raw_tx_hex = Some(hex::encode(&transaction.bytes));
    evidence.submitted_txid = Some(transaction.txid);
    evidence.transparent_destination = Some(transaction.destination);
    evidence.submission_result = Some(match submit(transaction.bytes).await {
        Ok(reply) => SubmissionResult {
            rpc_completed: true,
            error_code: Some(reply.error_code),
            error_message: reply.error_message,
        },
        Err(error) => SubmissionResult {
            rpc_completed: false,
            error_code: None,
            error_message: format!("{error:#}"),
        },
    });
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_transaction;

    fn received_note() -> ReceivedNote {
        ReceivedNote {
            txid: "lab-received-note".into(),
            height: 1_028_505,
            output_index: 0,
            value_zat: 70_000_000,
            recipient_addr: "public-test-address".into(),
            memo: None,
        }
    }

    #[test]
    fn real_shielded_policy_fails_before_a_fallback_is_selected() {
        let failure = attempt_shielded_send(70_010_000, 70_000_000).unwrap_err();
        assert_eq!(failure.code, "insufficient_scanned_value");
        assert!(failure.message.contains("exceeds"));
        assert_eq!(
            attempt_shielded_send(1, 70_000_000).unwrap_err().code,
            "shielded_spending_unavailable"
        );
    }

    #[test]
    fn constructed_fallback_is_a_complete_unsigned_transparent_v4_transaction() {
        let built = build_transparent_test_transaction(70_010_000, 1_028_510).unwrap();
        let tx = parse_transaction(&built.bytes, 1_028_510).unwrap();
        assert_eq!(tx.version(), TxVersion::V4);
        assert_eq!(tx.txid().to_string(), built.txid);
        assert!(tx.sapling_bundle().is_none());
        assert!(tx.sprout_bundle().is_none());
        assert!(tx.orchard_bundle().is_none());
        let transparent = tx.transparent_bundle().unwrap();
        assert_eq!(transparent.vin.len(), 1);
        assert_eq!(transparent.vout.len(), 1);
        assert_eq!(
            transparent.vin[0].prevout(),
            &OutPoint::new(NONEXISTENT_PREVOUT, 0)
        );
        assert_eq!(transparent.vin[0].script_sig(), &Script::default());
        assert_eq!(transparent.vout[0].value().into_u64(), 70_010_000);
        assert_eq!(
            transparent.vout[0].recipient_address(),
            Some(TransparentAddress::PublicKeyHash(LAB_DESTINATION_HASH))
        );
        assert!(
            parse_transaction(&built.bytes[..built.bytes.len() - 1], 1_028_510).is_err(),
            "removing the trailing bundle encoding must make the transaction incomplete"
        );
    }

    #[tokio::test]
    async fn leaky_branch_really_submits_the_serialized_transparent_bytes() {
        let mut submitted = Vec::new();
        let evidence = exercise_fallback(&[received_note()], 1_028_510, false, |raw| {
            submitted.push(raw);
            std::future::ready(Ok(SubmissionReply {
                error_code: 19,
                error_message: "unsigned test transaction rejected".into(),
            }))
        })
        .await
        .unwrap();
        assert_eq!(submitted.len(), 1);
        assert_eq!(
            evidence.fallback_raw_tx_hex,
            Some(hex::encode(&submitted[0]))
        );
        assert_eq!(evidence.shielded_failure, "insufficient_scanned_value");
        assert_eq!(evidence.requested_value_zat, 70_010_000);
        assert_eq!(evidence.outcome, "transparent_submission_attempted");
        assert_eq!(evidence.submission_result.unwrap().error_code, Some(19));
        assert!(!evidence.consensus_valid && !evidence.funds_moved);
    }

    #[tokio::test]
    async fn fixed_branch_refuses_without_calling_submission_or_creating_raw_bytes() {
        let evidence = exercise_fallback(&[received_note()], 1_028_510, true, |_| {
            panic!("fixed policy must never call submission");
            #[allow(unreachable_code)]
            std::future::ready(Ok(SubmissionReply {
                error_code: 0,
                error_message: String::new(),
            }))
        })
        .await
        .unwrap();
        assert!(evidence.shielded_attempted);
        assert_eq!(evidence.outcome, "refused");
        assert!(evidence.fallback_raw_tx_hex.is_none());
        assert!(evidence.submitted_txid.is_none());
        assert!(evidence.transparent_destination.is_none());
        assert!(evidence.submission_result.is_none());
    }

    #[tokio::test]
    async fn transport_failure_is_recorded_without_a_false_acceptance_claim() {
        let evidence = exercise_fallback(&[received_note()], 1_028_510, false, |_| {
            std::future::ready(Err(anyhow::anyhow!("proxy unavailable")))
        })
        .await
        .unwrap();
        let result = evidence.submission_result.unwrap();
        assert!(!result.rpc_completed);
        assert_eq!(result.error_code, None);
        assert!(result.error_message.contains("proxy unavailable"));
        assert!(!evidence.consensus_valid && !evidence.funds_moved);
    }
}
