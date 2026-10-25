use crate::{Finding, PairReport, RunResult};
use std::fmt::Write;

pub fn escape(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            c if c.is_control() && c != '\n' && c != '\t' && c != '\r' => {
                format!("\\u{:04x}", c as u32)
            }
            _ => c.to_string(),
        })
        .collect()
}

fn badge(finding: &Finding) -> &'static str {
    if !finding.exercised {
        "NOT EXERCISED"
    } else if finding.leaked {
        "LEAK OBSERVED"
    } else {
        "NO FINDING"
    }
}

fn finding_state(finding: &Finding) -> &'static str {
    if !finding.exercised {
        "muted"
    } else if finding.leaked {
        "bad"
    } else {
        "good"
    }
}

fn trace(run: &RunResult, report: &PairReport) -> String {
    let mut html = format!("<div class=trace-line><span class=ln>01</span><code>GetBlockRange({}…{})</code><span class=tag>public window</span></div>", run.wallet.scan_start, run.wallet.scan_end);
    for (i, txid) in run.transaction_fetches.iter().enumerate() {
        let short = format!("{}…{}", &txid[..12], &txid[56..]);
        let is_payment = *txid == report.ground_truth.txid;
        let class = if is_payment && !run.full_window_schedule {
            "trace-line marked"
        } else {
            "trace-line"
        };
        let label = if is_payment {
            "fixture payment"
        } else {
            "other transaction"
        };
        let _ = write!(html, "<div class=\"{class}\"><span class=ln>{:02}</span><code title=\"{}\">GetTransaction({})</code><span class=tag>{label}</span></div>", i + 2, escape(txid), escape(&short));
    }
    html
}

fn panel(run: &RunResult, finding: &Finding, report: &PairReport, fixed: bool) -> String {
    let profile = if fixed { "fixed" } else { "leaky" };
    let label = if fixed {
        "AFTER / FIXED"
    } else {
        "BEFORE / LEAKY"
    };
    let trace_html = match finding.id {
        "A1" => format!("<div class=terminal>{}</div><p class=trace-note>Ordered request summary. Payment labels come from ground truth, not the observer.</p>", trace(run, report)),
        "A3" => format!("<pre class=terminal><code>{}</code></pre>", escape(&run.app_log_excerpt)),
        _ => "".into(),
    };
    let download = if finding.id == "A3" {
        "app.log"
    } else {
        "observations.jsonl"
    };
    format!("<section class=\"evidence-panel {}\"><div class=panel-heading><span>{label}</span><strong class=state>{}</strong></div>{trace_html}<div class=explanation><h4>What the observer sees</h4><p>{}</p><h4>What that reveals</h4><p>{}</p><a class=text-link href=\"{profile}-{download}\" download>Download captured {download} <span aria-hidden=true>↗</span></a></div></section>", finding_state(finding), badge(finding), escape(&finding.evidence), escape(&finding.inference))
}

pub fn render_report(report: &PairReport) -> String {
    let mut cards = String::new();
    for before in &report.leaky.findings {
        let after = report
            .fixed
            .findings
            .iter()
            .find(|f| f.id == before.id)
            .expect("matching findings");
        if !before.exercised && !after.exercised {
            let _ = write!(cards, "<article class=unexercised><span class=case-id>{}</span><div><h3>{}</h3><p>{}</p></div><span class=neutral-badge>NOT EXERCISED</span></article>", before.id, escape(before.title), escape(&before.evidence));
            continue;
        }
        let boundary = if before.id == "A1" {
            "WALLET → LIGHT-WALLET SERVER"
        } else {
            "WALLET → MERCHANT LOGS"
        };
        let ground_truth = if before.id == "A1" {
            format!("Known truth: this wallet decrypted the fixture payment at height {} for {} zatoshi. The server-visible query identifies a transaction, not a recipient.", report.ground_truth.height, report.ground_truth.value_zat)
        } else {
            format!("Known truth: the encrypted fixture memo decrypts to “{}”. We compare that exact content against captured application logs.", report.ground_truth.memo_plaintext)
        };
        let _ = write!(cards, "<article class=case id=\"{}\"><header class=case-header><span class=case-id>{}</span><div><p class=eyebrow>{boundary}</p><h3>{}</h3></div></header><p class=ground-truth>{}</p><div class=comparison>{}{}</div><div class=fix><span class=fix-label>THE CHANGE</span><p>{}</p></div></article>", before.id.to_lowercase(), before.id, escape(before.title), escape(&ground_truth), panel(&report.leaky, before, report, false), panel(&report.fixed, after, report, true), escape(before.fix));
    }
    let payment_json =
        serde_json::to_string_pretty(&report.ground_truth).expect("serializable fixture");
    let status = if report.passed {
        "EXPECTED LEAKS REPRODUCED · FIXED RUN CLEAN"
    } else {
        "REGRESSION GATE FAILED · INSPECT EVIDENCE"
    };
    let status_class = if report.passed { "good" } else { "bad" };
    let links = [
        ("result.json", "Machine-readable result"),
        ("leaky-observations.jsonl", "Leaky RPC trace"),
        ("fixed-observations.jsonl", "Fixed RPC trace"),
        ("leaky-app.log", "Leaky application log"),
        ("fixed-app.log", "Fixed application log"),
        ("leaky-wallet.json", "Leaky wallet evidence"),
        ("fixed-wallet.json", "Fixed wallet evidence"),
    ]
    .iter()
    .map(|(url, name)| {
        format!("<a href=\"{url}\" download>{name}<span aria-hidden=true>↗</span></a>")
    })
    .collect::<String>();
    let ci_link = match &report.provenance.ci_run_url {
        Some(url) => format!("<a class=text-link href=\"{}\">Open the producing CI run ↗</a>", escape(url)),
        None => format!("<a class=text-link href=\"{}/actions\">View repository CI ↗</a><span class=kicker>LOCAL CAPTURE</span>", escape(&report.provenance.repository_url)),
    };
    let video_link = if std::path::Path::new("docs/demo.mp4").is_file() {
        "<a class=text-link href=\"demo.mp4\">Watch the 2-minute walkthrough ↗</a>".to_owned()
    } else {
        String::new()
    };
    let values = [
        ("STATUS", status.to_owned()),
        ("STATUS_CLASS", status_class.to_owned()),
        ("BEFORE_COUNT", report.leaky.leaked_ids().len().to_string()),
        ("AFTER_COUNT", report.fixed.leaked_ids().len().to_string()),
        ("CHECK_COUNT", report.exercised_checks.len().to_string()),
        (
            "RPC_COUNT",
            (report.leaky.observation_count + report.fixed.observation_count).to_string(),
        ),
        ("CANDIDATE_COUNT", report.candidate_txids.len().to_string()),
        ("CARDS", cards),
        ("PAYMENT_JSON", escape(&payment_json)),
        ("DOWNLOADS", links),
        ("TRUST", escape(report.trust_boundary)),
        ("REPOSITORY_URL", escape(&report.provenance.repository_url)),
        (
            "FIXTURE_SOURCE_URL",
            escape(&report.provenance.fixture_source_url),
        ),
        ("CI_LINK", ci_link),
        ("VIDEO_LINK", video_link),
    ];
    // Only parse tokens in the trusted template, never tokens inside a fixture memo.
    let mut remaining = include_str!("template.html");
    let mut html = String::new();
    while let Some(start) = remaining.find("{{") {
        html.push_str(&remaining[..start]);
        let token = &remaining[start + 2..];
        let end = token.find("}}").expect("closed template token");
        let value = values
            .iter()
            .find(|(name, _)| *name == &token[..end])
            .expect("known template token");
        html.push_str(&value.1);
        remaining = &token[end + 2..];
    }
    html.push_str(remaining);
    html
}
