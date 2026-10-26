# Captured-run replay

This feature was reviewed on `report-interactive-replay` and is included in the main report. It adds no wallet execution, backend, analytics, or network request to the report. The label **Replay of the captured run** identifies the interaction as a view of saved evidence.

The leaky profile is selected first. Changing profiles switches the displayed counts and evidence panels together. The complete captured run is visible initially; the step controls let a reader restart and reveal records in their recorded order. Findings still describe the complete validated run, not just the currently visible prefix. Turning JavaScript off preserves the original static before/after comparison, downloads, methodology, and trust boundary.

## Exact evidence

The analyzer reads each profile's `observations.jsonl`, `app.log`, and `wallet.json` once. It validates those buffers, writes the same bytes to the downloadable files, and embeds their Base64 representation, byte lengths, and SHA-256 hashes in an inert JSON block. It does not reconstruct raw evidence from parsed summaries. This preserves metadata, whitespace, Unicode, and line endings that would otherwise disappear during JSON serialization.

The browser decodes the embedded files for display and uses `textContent` for untrusted content. No fetch, external library, or JavaScript build tool is used. Evidence downloads remain available with JavaScript disabled.

## Evidence boundaries

- **A1:** the leaky run has an extra payment-specific lookup. The fixed run still includes one `GetTransaction` for that payment in the complete public batch. Its legitimate lookup is retained and is not labeled a leak.
- **A3:** application log records are shown separately from the server's RPC trace. The report does not invent timestamps or an interleaving between those streams.
- **A2:** `SendTransaction` is a captured server event. The fixed refusal is local `wallet.json` evidence, displayed as such; an absent send alone does not prove the refusal path ran. The constructed transaction remains unsigned and unfunded. Acceptance is not established.

## Content Security Policy

The existing policy retains `default-src 'none'`, its existing inline-style allowance, data-only images, and disabled base/form destinations. The only added execution permission is `script-src 'sha256-…'` for the exact trusted inline replay script. No `unsafe-inline` script permission, nonce, external origin, or `eval` is added. The script is normalized to LF before both hashing and embedding so Windows checkouts cannot produce a hash that differs from HTML parsing. The inert evidence JSON escapes characters that could terminate its script element.

Controls use native keyboard-operable elements and concise live status announcements. Stepping is manual; no animation or automatic playback is introduced. Existing reduced-motion styling remains in effect.

## Historical isolated Pages preview

The following workflow was used to review the branch before its merge. Main publication replaces that preview site; use https://willy264.github.io/zcash/ for the current replay. The historical workflow refuses to redeploy its old baseline after main advances.

Normal branch pushes run the same proof and upload an artifact. A manual dispatch with `publish_preview=true` on this exact branch additionally composes a site from two CI artifacts: the pinned successful main site, unchanged at its original paths, and the branch report under `previews/report-interactive-replay/`.

The main baseline is pinned in `.github/preview-base.json`. `scripts/preview_pages.py` verifies the main commit has not advanced, checks the three-check gate in both artifacts, compares the main artifact with the live site, preserves every original file, and verifies both sets of published hashes afterward. The preview job uses its own branch-restricted `github-pages-preview` environment and serializes with main publication. The existing `github-pages` environment and `main` branch are unchanged.

This is a review path on the same static Pages site, not GitHub's unavailable native PR-preview feature. Deployment metadata identifies the preview workflow; the root report retains its original content and producing CI provenance. The next normal main deployment removes the review path. The pinned main Pages artifact must still be available; this workflow refuses to substitute a locally rebuilt root site if it has expired.
