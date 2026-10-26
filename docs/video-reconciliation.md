# Video reconciliation

The original two-check narration is compared with the final script below. The new recording shows the published three-check report from commit `8e3302ad115a476c2a3881b062d60d84f36edcdb`, produced by [successful CI run 40277992851](https://github.com/willy264/zcash/actions/runs/40277992851). Its measured facts are **3 checks, 10 calls, 3 candidates, 3 leaky findings, and 0 fixed findings**. Source downloads and their hashes are recorded in `docs/demo.json`.

Minus lines quote the original narration. Plus lines quote the final spoken wording, including its punctuation. Bracketed notes describe what is visible or remains offscreen.

```diff
00:00-00:13 | docs/video-script.md | hero
- This payment is shielded. But the wallet's requests and the merchant's logs can still expose private facts.
+ The payment is shielded. The extra lookup still leaks.
+ Across three checks, ten calls, and three candidate transactions:
+ three findings before the fixes. Zero after.
[Leads with the point; all five counts match the visible published result.]

00:13-00:31 | docs/video-script.md | A1 trace
- A small Rust client uses Zcash wallet cryptography to decrypt a known test payment. Its requests pass through an observer to a controlled light-wallet server.
+ For A1, start with GetBlockRange. Both runs scan the same window.
+ Now look at GetTransaction on the left.
+ The payment appears twice.
+ On the right, each candidate gets one lookup.
[The old client/proxy setup is not being constructed on screen. The replacement describes the visible captured requests.]

00:31-01:01 | docs/video-script.md | A1 comparison
- The fixed client scans locally.
+ The fixed trace keeps GetBlockRange.
+ For the memo, it fetches all three candidates once, in chain order.
+ The extra GetTransaction is gone.
[The local scan is offscreen; its trace is visible. The existing interest-not-identity limit remains explicit:]
+ The extra lookup tells this server that the session is interested in that transaction.
+ That's the limit of the claim. It doesn't identify the recipient.

01:01-01:24 | docs/video-script.md | A3 app.log
- Now follow the decrypted memo to the merchant.
- The merchant still receives the memo, but its log no longer exposes those fields.
+ A3 is app.log.
+ The left side contains the decrypted memo and the address.
+ My app wrote a plaintext copy into its log.
+ On the right, those fields say redacted.
[The report shows captured log contents. Merchant delivery is supported by the run evidence but is not shown as a live network action.]

01:24-01:34 | docs/video-script.md | A2 comparison
- Transparent fallback is marked untested, and there is no anonymity percentage.
+ A2. Here's SendTransaction.
+ Unsigned transparent test bytes on the left.
+ The fixed run refuses the send.
+ No funds move.
[A2 now runs. The bytes are unsigned and the input references a nonexistent prevout. This demonstrates a submission attempt after local insufficient funds, not an accepted or mined spend.]

01:34-01:49 | docs/video-script.md | methodology
- The build passes only when the leaky run reproduces both findings and the fixed run removes them.
+ This is a staged chain. Darkside doesn't check consensus.
+ I'm checking this server's view and these application logs.
+ The leaky run shows three findings. The fixed run shows none.
+ Other privacy leaks can still exist.
[The count changes from two to three. The observer and staged-chain limits are stated while their methodology is visible; no global anonymity claim is made.]

01:49-02:00 | docs/video-script.md | reproduction links, then hero
- Run make demo to reproduce the evidence.
+ make demo reruns the checks.
+ The successful CI run and raw traces are linked here.
+ I'd check those before trusting the green result.
[make demo and CI are not executed on screen. The completed run and raw traces are visibly linked during the spoken claim at 01:51-01:54, before the hero returns at 01:56.]
```

The synthetic voice changed from Microsoft Zira Desktop to `en-US-AndrewMultilingualNeural` at `+6%`, with explicit short pauses. It is not presented as a human recording. `docs/demo.vtt` now has **52 short verbatim cues**, aligned to **246 actual TTS word-boundary events**, replacing the original six long segment captions. Stage directions are not spoken or captioned.

The video is a fresh capture of the public report in the original viewing sequence. It runs **120.0 seconds** and is **2,436,169 bytes**. The recorder measured the first hero-to-A1 cut at 14.24 seconds in the raw recording and trimmed the 1.24-second browser lead-in so that the cut lands at 00:13. Decoded final frames were reviewed at the A1, A3, A2, methodology, link, and closing-hero cues. In particular, the A2 frame shows both `SendTransaction = 1` and the fixed refusal with `SendTransaction = 0`; the closing frame includes the successful CI link and downloadable raw traces while they are named.

The recorder checks the live result, producing CI success, source hashes, counts, caption wording, and claim timing before replacing the artifacts. It cannot relabel arbitrary old footage as a new live capture. Script and caption files use LF line endings so their recorded hashes also match the committed source bytes.
