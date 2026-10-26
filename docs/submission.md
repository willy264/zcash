# Darkroom submission fields

- **Project:** Darkroom
- **Track:** Core & Tooling
- **Repository:** https://github.com/willy264/zcash
- **Working demo:** https://willy264.github.io/zcash/
- **Reproduce:** `make demo`
- **CI:** https://github.com/willy264/zcash/actions/workflows/ci.yml (the report links its exact producing run)
- **Video:** https://willy264.github.io/zcash/demo.mp4 (two-minute capture of the verified public three-check report; conversational synthetic neural narration)

## Description

Darkroom is a privacy regression lab for Zcash wallets. A shielded payment can still leak through the requests a wallet makes and the logs an application writes.

We exercise three named leaks with a Rust subject that really trial-decrypts Sapling notes. A1 is an extra payment-specific GetTransaction request that reveals session interest in a shielded transaction. A3 is a merchant's plaintext memo/address in app.log. A2 is a failed shielded-send admission check that silently falls back to constructing and submitting transparent transaction bytes. A tonic proxy captures the subject's real requests to upstream lightwalletd in Darkside mode. The static report shows the captured evidence, the known fixture facts, and the same view after each fix.

The fixed client scans locally and fetches every public candidate transaction once, in chain order, when memos are needed; compact blocks do not contain memos. The merchant redacts its private log fields. The send policy refuses explicitly instead of submitting a transparent fallback. CI passes only when the leaky run reproduces exactly A1, A3, and A2 and all three checks are exercised with zero findings in the fixed run. Missing evidence, skipped send attempts, and wrong decrypted facts fail the gate.

Real: wallet cryptography, encrypted transaction bytes, memo decryption, gRPC/HTTP traffic, captured logs, the failing shielded-send admission check, transparent transaction serialization/submission, and assertions. Controlled: public upstream test keys, staged blocks, known ground truth, and intentionally vulnerable policies. A2 requests more than the scanned-note value upper bound; this is not an authenticated spendable balance. Its transparent fallback is unsigned and unfunded, with a nonexistent prevout. It is constructed and submitted to Darkside, not mined or consensus-valid. No funds move; transaction acceptance is not established. Darkside does not validate consensus.

A targeted lookup shows interest, not proof of receipt or identity. We test named leaks under a stated observer: the light-wallet server's captured requests and the merchant's application logs. We do not claim mainnet execution, global anonymity, or the absence of all privacy leaks. The video scrolls the generated evidence report; it does not depict a live transaction execution.

## Before submitting

Confirm the public video, live report, and report-linked successful CI run all show A1, A3, and A2 exercised: three findings before, zero after. The video source report and CI provenance are recorded in `docs/demo.json`; word-aligned captions are in `docs/demo.vtt`. The submission form itself must be completed by the entrant; this file contains the prepared field values.
