# Darkroom

A privacy regression lab for Zcash wallet access patterns and application logs.

[Live report](https://willy264.github.io/zcash/) · [CI runs](https://github.com/willy264/zcash/actions/workflows/ci.yml) · [Research](ZECATHON-RESEARCH.md)

Darkroom runs a small Rust wallet client against a staged chain, forwards its actual gRPC requests through an observer, and compares the observer's evidence with independently authored ground truth. The same encrypted payment runs through a deliberately leaky policy and a fixed policy. The analyzer fails unless the leaky run exposes exactly the expected facts and the fixed run removes those specific signals.

## Run it

Requirements: stable Rust, Go 1.27.3 or newer, Python 3.12 or newer, and GNU Make. The first run downloads Cargo dependencies and builds the pinned lightwalletd source. No funded wallet, running full node, Docker, system `protoc`, or JavaScript build is required.

```sh
make demo
# Open report/index.html
```

On Windows, use `python scripts/demo.py` from PowerShell. This installs a checksum-verified portable Go toolchain under `.tools/` when Go is absent and applies a [small Windows cache-truncation fix](docs/windows-backend.md) to the backend build. Rust must already be installed. Linux and macOS use the unmodified pinned backend source.

```sh
make fixture  # verify the committed transaction bytes with real wallet cryptography
make test     # detector, malformed-input, and redaction tests
make check    # formatting and Clippy
make report   # regenerate the report from the latest successful captured run
```

The runner starts the backend, stages local fixture bytes, then starts an independent proxy and merchant process for each profile. It waits for readiness, checks exit statuses, preserves each run under `.runs/`, and terminates its services even on failure. Loopback ports 9067, 9068, 19067, and 18080 must be free. An attempted rerun removes the old report entry points so stale green output cannot stand in for a failed run.

## What the experiments prove

| Assertion | Leaky observation | Fixed behavior | Scope |
|---|---|---|---|
| A1: targeted fetch | A `GetTransaction` request singles out a locally decrypted payment | Scan locally; when memos are needed, fetch every transaction in the fixed public window once, in public order | Session interest in a particular transaction |
| A3: memo in logs | The merchant writes a decrypted memo/address to `app.log` | `DARKROOM_FIX=1` redacts both fields | Disclosure through application logs |
| A2: transparent fallback | A failed shielded-send admission check silently constructs an unsigned transparent transaction and calls `SendTransaction` | `DARKROOM_FIX=1` refuses, writes explicit evidence, exits with error code 3, and makes no send RPC | Transparent components disclosed in submitted bytes; mining and acceptance are not established |

**A targeted lookup establishes a session's interest in a transaction. It does not, by itself, prove receipt, spending authority, or the user's identity.** The fixture gives the lab ground truth about receipt; that extra knowledge is kept separate from the adversary's observation.

Compact blocks contain enough ciphertext for note recognition and value decryption, but **they do not contain memos**. The scan-only GOOD path therefore makes no targeted full-transaction request. The combined memo demo retrieves full encrypted transactions for the entire fixed public window, independently of which notes decrypt. The leaky profile makes an additional payment-specific request. The analyzer checks the actual request sequence; it does not accept a client-provided “private mode” label as proof. This costs extra bandwidth and only removes the tested selective-fetch signal within this window.

For A2, both profiles request 10,000 zatoshis more than the total value actually scanned. That total is only an upper bound, not an authenticated spendable balance. The subject's local shielded-send policy rejects the request before any shielded transaction can be built. The leaky path then constructs a complete V4 transaction using librustzcash serialization: one nonexistent transparent input, an empty signature script, and one transparent output. Its bytes pass through the actual proxy to Darkside. The fixed path refuses and emits an explicit error; the runner expects exit code 3 and the analyzer requires refusal evidence plus zero `SendTransaction` calls. Merely skipping the attempt cannot pass A2.

## Trust boundary and provenance

The modeled adversary sees the proxy's logged RPC requests and all merchant application logs. Ground truth, wallet decryption receipts, and the deliberately published test key are excluded from that observer's inputs. This controlled experiment does not claim secrecy against someone who downloads the fixture key. **Never send real funds to these keys.**

Real components: upstream lightwalletd in Darkside mode, tonic gRPC forwarding, transaction parsing and serialization, Sapling trial decryption, full encrypted-memo decryption, wallet-originated RPC traces, the failing shielded-send policy and transparent submission, HTTP delivery to the merchant, log redaction, and the analyzer. The subject is a small research client using librustzcash components; it is not a production wallet or a complete sync engine.

Controlled components: the staged block sequence, intentionally vulnerable policies, public test keys, and known payment ground truth. Darkside replays transactions without consensus validation. This is not a new mainnet payment, a consensus proof, a production-wallet audit, or an anonymity score. Darkroom demonstrates named leaks under the modeled observer; a clean report does not prove the absence of all privacy leaks. IP correlation, timing attacks beyond the recorded trace, malicious servers, and other observers remain outside that claim.

The A2 fallback is **constructed and submitted, not mined or consensus-valid**. Its input has no funding and no signature. Darkside recording the bytes, or returning a success code, does not establish transaction acceptance. No funds move. The tested disclosure is to the server that receives those transparent bytes; this lab does not claim they appeared on a public explorer.

Fixture source, license, byte hashes, and the actual decrypted facts are recorded in `fixtures/`. The backend release and archive checksum are pinned in [scripts/backend.json](scripts/backend.json); vendored protocol provenance is in [proto/README.md](proto/README.md). Rust dependencies were selected with `cargo add` and are locked in `Cargo.lock`.

## Layout

```text
crates/walletrpc       Generated upstream gRPC client and server types
crates/stager         Local fixture → Darkside administrative RPCs
crates/observer-proxy  Wallet-facing forwarding service → observations.jsonl
crates/subject-wallet Real compact/full note decryption → wallet.json
crates/merchant-app   Local checkout HTTP endpoint → app.log
crates/analyzer       Strict assertions → JSON and standalone HTML
fixtures/             Public test bytes, wallet-only configuration, ground truth
scripts/              Reproducible backend setup and process orchestration
report/               Generated report and evidence downloads; ignored by Git
```

The subject's configuration contains its public test key and public scan range. It does not read expected payment IDs, amounts, or memos from the analyzer's ground-truth file during a network run.

## CI and publishing

GitHub Actions runs format/lint checks, tests, the fixture gate, both captured profiles, and strict expectations. Only a successful run on `main` publishes `report/` to GitHub Pages. Branches and pull requests execute the same proof and upload a report artifact for review without deploying. The report includes the CI run/commit when generated in Actions. A walkthrough is linked and copied into the report only when its metadata lists the same exercised leaks, preventing an old two-leak video from being presented as the current three-leak walkthrough.

The expected leaky findings are **A1, A3, and A2**. All three must be exercised in the fixed profile with zero findings. Empty traces, missing application events, wrong decrypted facts, missing scan coverage, absent or contradictory send/refusal evidence, unexpected findings, or leaks in the fixed run fail the build.

To gate one captured run directly:

```sh
cargo run --locked -p analyzer -- --expect clean \
  --observations path/to/observations.jsonl --app-log path/to/app.log \
  --wallet path/to/wallet.json --ground-truth fixtures/ground_truth.json \
  --output report
```

Use `--expect leaky` for the known vulnerable control. Adapting another wallet requires capturing its requests through the proxy and emitting the same decryption-receipt schema; changing a mode label cannot make the fixed expectation pass.

MIT licensed. Vendored upstream material retains its own notices.
