# Two-minute walkthrough

This script describes the implemented A1 and A3 experiments. It deliberately does not claim a transparent-fallback experiment or infer receipt solely from a lookup.

**0:00–0:13 — The boundary**

This payment is shielded. But the wallet's requests and the merchant's logs can still expose private facts. Darkroom tests that boundary with real encrypted transactions and captured traffic.

**0:13–0:31 — The setup**

A small Rust client uses Zcash wallet cryptography to decrypt a known test payment. Its requests pass through an observer to a controlled light-wallet server. The report keeps what the observer saw separate from the ground truth used to check the result.

**0:31–1:01 — Leak one**

Here is the extra GetTransaction request. It appears because the client recognized its payment. The request links this session to that transaction of interest. It does not, by itself, prove who received it. The fixed client scans locally. Because compact blocks do not contain memos, the memo demo retrieves every transaction in the same public window, once and in order. The selective extra request disappears. The detector checks the captured sequence, not a mode label.

**1:01–1:24 — The application leak**

Now follow the decrypted memo to the merchant. The leaky service writes the memo and address into its application log. Encryption on chain cannot protect that plaintext copy. We enable field redaction and run the same payment again. The merchant still receives the memo, but its log no longer exposes those fields.

**1:24–1:49 — The regression gate**

The build passes only when the leaky run reproduces both findings and the fixed run removes them. Empty logs, incomplete scans, or incorrect decrypted facts fail. These are real wallet requests and real decryption, replayed on a labeled test chain. Darkside does not check consensus. Transparent fallback is marked untested, and there is no anonymity percentage.

**1:49–2:00 — Close**

Run make demo to reproduce the evidence. Darkroom tests specific privacy leaks beyond the shielded transaction. Open the traces, inspect the fixes, and put those checks in CI.
