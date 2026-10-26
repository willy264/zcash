# Two-minute live-report walkthrough

This narration matches the published three-check report and its successful main-branch CI run. The recorder independently verifies that evidence before capture. Timecodes and pause markers are direction only; they are not spoken. Narration uses a clearly disclosed synthetic neural voice at +6% rate, with varied explicit pauses.

<!-- evidence: {"checks":3,"calls":10,"candidates":3,"leaky":3,"fixed":0} -->

**0:00–0:13 — The boundary**

The payment is shielded. The extra lookup still leaks.
[pause 0.5s]
Across three checks, ten calls, and three candidate transactions:
three findings before the fixes. Zero after.

**0:13–0:31 — The setup**

For A1, start with GetBlockRange. Both runs scan the same window.
Now look at GetTransaction on the left.
The payment appears twice.
On the right, each candidate gets one lookup.
[pause 0.3s]
That extra question is the leak.

**0:31–1:01 — The extra request**

The extra lookup tells this server that the session is interested in that transaction.
That's the limit of the claim. It doesn't identify the recipient.
[pause 0.6s]
The fixed trace keeps GetBlockRange.
For the memo, it fetches all three candidates once, in chain order.
The extra GetTransaction is gone.
[pause 0.3s]
That's the request I'd want CI to catch.

**1:01–1:24 — The application log**

A3 is app.log.
The left side contains the decrypted memo and the address.
My app wrote a plaintext copy into its log.
That's a copy I didn't need.
[pause 0.6s]
On the right, those fields say redacted.
Same payment. Same check. Less private data in the log.
That's a change I can actually review.

**1:24–1:34 — The fallback**

A2. Here's SendTransaction.
Unsigned transparent test bytes on the left.
The fixed run refuses the send.
No funds move.

**1:34–1:49 — The boundary**

This is a staged chain. Darkside doesn't check consensus.
[pause 0.4s]
I'm checking this server's view and these application logs.
The leaky run shows three findings. The fixed run shows none.
Other privacy leaks can still exist.

**1:49–2:00 — Close**

make demo reruns the checks.
The successful CI run and raw traces are linked here.
I'd check those before trusting the green result.
