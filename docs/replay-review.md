# Replay review

The replay is developed on `report-interactive-replay`. Main remains at `8f2749f8b893e25f32b1338207c8932fb7fab283` during this review.

The baseline screenshots came from the public main report after its three-check video update, produced by [CI run 40279823921](https://github.com/willy264/zcash/actions/runs/40279823921). [Source metadata](replay/before-source.json) records the HTML, result, and screenshot SHA-256 values.

The final local browser run passed in both `file://` and HTTP modes against the report generated from `.runs/20261026T064515470049Z`. The [verification record](replay/local-verification.json) records each checked behavior and the exact report, script, and six evidence-file hashes. Its null CI provenance is intentional: these screenshots show a real local capture, while the baseline comes from the published main CI report. Top screenshots reset the selected profile before capture.

| View | Published main before replay | Replay: leaky | Replay: fixed |
|---|---|---|---|
| Top, 1440 × 1000 | [Before](replay/before-top.png) | [Leaky](replay/after-leaky-top.png) | [Fixed](replay/after-fixed-top.png) |
| A1, 1440 × 1000 | [Before](replay/before-a1.png) | [Leaky](replay/after-leaky-a1.png) | [Fixed](replay/after-fixed-a1.png) |
| A1, 390 × 844 | — | [Leaky](replay/after-leaky-mobile.png) | [Fixed](replay/after-fixed-mobile.png) |

## Reproduce the browser verification

The optional [browser checker](../scripts/check_replay.py) uses Python, Playwright, and either installed Microsoft Edge or Playwright Chromium. It also finds an existing Playwright installation under `.tools/video-tools`. These tools are not needed to generate or open the report.

```sh
python scripts/check_replay.py --report report/index.html \
  --output docs/replay/local-verification.json
```

This runs the same behavior checks against `file://` and a temporary HTTP server bound to a randomly allocated loopback port. It verifies the six embedded evidence files against their original downloads, the report result, the script's CSP hash, and the actual browser downloads.

It exercises keyboard profile selection, focus retention, all three cards, counters, every manual replay step, raw JSON drawers, clipboard copy, polite status announcements, reduced motion, and mobile layout with expanded transaction bytes. It rejects browser errors, CSP violations, or network requests after the initial document. A second browser context disables JavaScript and requires the full static comparison to remain available.

The semantic checks keep the evidence sources distinct. A1 highlights the extra payment lookup while preserving the fixed run's legitimate payment fetch. A3 reads actual application-log lines with separate RPC context and no invented shared timeline. A2 shows local refusal evidence separately from the captured server requests. Stepping through partial traces cannot change the verdict for the complete run.

To verify a deployed preview, use its URL and producing branch commit:

```sh
python scripts/check_replay.py --url PREVIEW_URL \
  --expected-commit BRANCH_COMMIT --screenshot-prefix preview \
  --output docs/replay/preview-verification.json
```

The preview check also compares embedded bytes to the files actually served at that URL. Download verification performed by the Python harness is separate from the page's zero-network replay behavior.
