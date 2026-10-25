# Darkroom submission fields

- **Project:** Darkroom
- **Track:** Core & Tooling
- **Repository:** https://github.com/willy264/zcash
- **Working demo:** https://willy264.github.io/zcash/
- **Reproduce:** `make demo`
- **CI:** https://github.com/willy264/zcash/actions/workflows/ci.yml
- **Video:** https://willy264.github.io/zcash/demo.mp4 (two-minute capture of the verified report with synthetic narration)

## Description

Darkroom is a privacy regression lab for Zcash wallets. A shielded payment can still leak through the requests a wallet makes and the logs an application writes.

We reproduce two leaks using a Rust client that really trial-decrypts Sapling notes: a payment-specific GetTransaction call links the session to a transaction of interest, and a merchant service logs a decrypted memo/address. A tonic proxy captures real wallet requests to upstream lightwalletd in Darkside mode. The report shows the observer's evidence, the known fixture facts, and the same view after each fix.

The fixed client scans locally and retrieves full transactions for the entire public window when memos are needed; compact blocks do not contain memos. The merchant redacts its private fields. CI passes only when the leaky run reproduces A1 and A3 and the fixed run has neither. Missing evidence and wrong decrypted facts fail the build. The one-command demo regenerates the report and original traces.

Real: wallet cryptography, gRPC/HTTP traffic, encrypted transaction bytes, memo decryption, logs, and assertions. Controlled: public upstream test keys, staged blocks, and intentionally vulnerable policies. Darkside does not validate consensus. Transparent fallback (A2) is explicitly not exercised. A targeted lookup shows interest, not proof of receipt or identity. We test named leaks under a stated observer; we do not claim global anonymity or the absence of all leaks.

## Before submitting

Use the report's actual successful CI run link. Confirm the repository, Pages report, and video open publicly. The submission form itself must be completed by the entrant; this file contains the prepared field values.
