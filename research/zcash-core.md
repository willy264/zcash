# Zcash technology research and Core & Tooling opportunities

Research date: 24 October 2026. Primary sources were fetched live from the Zcash ZIP repository, current Rust crates, Foundation/ECC/ZODL/Zingo repositories, release records, and protocol documentation. This report separates released code, activated consensus changes, draft specifications, and proposed products. The product concepts are recommendations, not claims about existing implementations.

## Executive findings that materially change the build decision

1. **Do not build from a 2024/2025 mental model of Zcash.** NU6.3 introduced the **Ironwood pool**, using the Orchard protocol. ZIP 258 is Final, sets mainnet activation at **3,428,143**, and prohibits cross-address payments and new value entering the old Orchard pool after activation. After this activation, new payments using **Orchard-protocol receivers** must route through **Ironwood**, rather than the old Orchard pool. This is not a claim that all Zcash payments use Ironwood: Sapling and transparent transfers also remain possible. The Orchard protocol and Orchard pool are no longer interchangeable terms. [ZIP 258, “Consensus rules from NU6.3 activation”](https://github.com/zcash/zips/blob/main/zips/zip-0258.md)
2. **Published current tooling exists.** PCZT 0.10.0 and `zcash_client_backend` 0.25.0 were published on 21 October 2026. Zebra 7.0.0 was released on 22 October, including support for the scheduled NU7 activation. Zaino 0.10.1 is a September stable release. Pin releases and verify compatibility rather than copying old tutorials. [PCZT changelog](https://github.com/zcash/librustzcash/blob/main/pczt/CHANGELOG.md), [backend crate](https://docs.rs/zcash_client_backend/0.25.0/zcash_client_backend/), [Zebra 7.0.0](https://github.com/ZcashFoundation/zebra/releases/tag/v7.0.0), [Zaino 0.10.1](https://github.com/zingolabs/zaino/releases/tag/0.10.1)
3. **The original ECC mobile SDKs are ending support.** Their READMEs say 4.0.0 is the final release, with support ending at NU7, expected November 2026. Successors are ZODL's Swift/Android SDKs and Slipstream. These use AGPLv3 with separate commercial licensing; upstream Rust libraries remain separately licensed MIT/Apache. This is an immediate architecture and licensing decision, not a distant maintenance concern. [Swift SDK](https://github.com/zcash/zcash-swift-wallet-sdk), [Android SDK](https://github.com/zcash/zcash-android-wallet-sdk), [Slipstream](https://github.com/zodl-inc/slipstream)
4. **Zcash privacy does not provide arbitrary private smart contracts.** Current shielded transfer proofs validate their fixed consensus statements. They cannot be programmed with an auction rule, revenue split, lending liquidation, subscription schedule, or arbitrary application condition simply by placing data in a memo. A separate application can coordinate such rules, but its trust and enforcement must be stated. [ZIP 224](https://github.com/zcash/zips/blob/main/zips/zip-0224.rst), [ZIP 222, motivation; status Draft](https://github.com/zcash/zips/blob/main/zips/zip-0222.rst)
5. **A shielded transaction is not an end-to-end privacy guarantee.** Indexer requests, IP addresses, payment URI distribution, application accounts, refund addresses, browser telemetry, and off-chain matching can expose the relationship that the chain conceals. The Zingolib README explicitly acknowledges server inference of transaction-containing blocks and IP metadata. [Zingolib privacy section](https://github.com/zingolabs/zingolib#privacy), [Zaino motivation](https://github.com/zingolabs/zaino#motivations)

## The technology, explained through what a product can actually use

### Notes, commitments, nullifiers, and proofs

A shielded balance is represented by notes, rather than a public account balance. A note's owner can spend it using the appropriate keys and a witness tying its commitment to a recognized commitment-tree root. The public chain contains commitments, encrypted note data, nullifiers that prevent double spending, and proofs that enforce validity without publishing all the private inputs. An observer does not simply look up a shielded address's balance in an explorer.

Orchard uses Halo 2 with PLONKish arithmetization and the Pasta curve cycle, instead of Sapling's Groth16 arrangement. The original Orchard deployment did **not** use Halo 2 recursive proofs; recursion friendliness is not evidence that any desired recursive application already exists. A transaction's Orchard-protocol bundle contains actions and an aggregate proof. The user-level implications are private transfer and useful spending/viewing-key separation, not an EVM-like contract runtime. [ZIP 224, proving system, bundle structure and security considerations](https://github.com/zcash/zips/blob/main/zips/zip-0224.rst), [Orchard commitments](https://zcash.github.io/orchard/design/commitments.html), [Orchard keys](https://zcash.github.io/orchard/design/keys.html)

Ironwood reuses the Orchard protocol with changes deployed by NU6.3. Its introduction followed Orchard circuit soundness mitigation and adds quantum **recoverability**. Do not describe it as making all Zcash transaction confidentiality quantum-proof: recovery of spending authority and permanent privacy are different properties. The exact cryptographic claims belong to ZIP 2005 and its assumptions. [ZIP 258](https://github.com/zcash/zips/blob/main/zips/zip-0258.md), [ZIP 2005](https://github.com/zcash/zips/blob/main/zips/zip-2005.md)

**What remains public:** transaction existence and timing, public transaction shape, transparent components when present, and net value crossing between pools. Same-pool shielded settlement is a substantially stronger privacy foundation than shielding/unshielding around every payment. Pool migration does not require a transparent address, but it still reveals pool-crossing amounts. Therefore “never unshielded” and “no amount leakage” are distinct requirements. [ZIP 318, terminology and motivation](https://github.com/zcash/zips/blob/main/zips/zip-0318.md)

### Unified Addresses and viewing keys

ZIP 316 lets a Unified Address carry several receiver types. Supporting wallets choose the most preferred receiver they support. A UA string beginning with `u` is **not by itself proof that a proposed transaction is fully shielded**: revision-0 UAs can contain a transparent receiver. Inspect the actual proposal, receiver selection, source pool, and change outputs. An Orchard-protocol receiver can receive in Ironwood; it is scoped to the protocol rather than exclusively to the old pool. [ZIP 316](https://github.com/zcash/zips/blob/main/zips/zip-0316.rst), [ZIP 326, “Receiving funds into the Ironwood pool”](https://github.com/zcash/zips/blob/main/zips/zip-0326.md)

Current ZIP 316 status is **Revision 0 Active; Revision 1 Withdrawn; Revision 2 Draft**. Do not assume revision-2 `zu`/`tu` prefixes, expiration metadata, or new UVK encodings are supported everywhere merely because they appear in the current specification.

Viewing keys are meaningful data-access capabilities. An incoming viewing key enables discovery of incoming notes for its scope. A full viewing key gives broader transaction visibility, including spend-related information, without spending authority. Sending an account's viewing key to a SaaS backend is still a financial-privacy disclosure. Generating fresh diversified addresses from that account does not turn the viewing key into a narrowly scoped per-invoice capability.

**Product pattern:** keep an account-level viewer on a merchant-owned/local service, and export narrowly defined application events, receipts, or user-selected records. Isolate accounts when organizational disclosure boundaries differ. Do not market a full-viewing-key export as a one-transaction receipt.

### ZIP 321 payment requests and encrypted memos

ZIP 321 is Active and standardizes `zcash:` payment URIs with recipient address, amount, optional label/message, and base64url memo. It also supports indexed multiple payments; actual wallet interoperability must still be tested. Memos are at most **512 bytes**. They are good for an opaque invoice ID, an application nonce, a short structured payload, or a hash of a larger encrypted off-chain record. They are not a sensible bulk data store. [ZIP 321, “Other parameters” and examples](https://github.com/zcash/zips/blob/main/zips/zip-0321.rst)

Three distinctions matter:

- The memo is encrypted **on chain**, but the payment URI carrying its plaintext/base64url representation is visible to whoever sees the QR code, web page, browser history, application logs, or analytics. Never put the sensitive purchase description or real identity in the public-facing URI just because the eventual memo will be encrypted.
- A memo can be authored by someone other than the claimed sender. A refund/reply address in a memo is not authenticated proof that it belongs to a spending party. ZIP 231 explicitly explains why authenticated Orchard reply addresses require zero-knowledge proofs too large for the current 512-byte field.
- Payment confirmation requires locally recognizing an actual output for the expected receiver and amount and checking chain status. A txid supplied by a customer, a screenshot, a memo string, or an HTTP callback alone is not payment verification.

ZIP 302 gives memo formatting guidance (`0xFF` for agreed arbitrary data, with the remaining 511 bytes unconstrained); its status remains Draft. ZIP 231's larger memo bundles are also Draft, not an available mainnet storage upgrade. [ZIP 302](https://github.com/zcash/zips/blob/main/zips/zip-0302.rst), [ZIP 231, motivation](https://github.com/zcash/zips/blob/main/zips/zip-0231.md)

### TEX is an unshielding route

ZIP 320 is Active and implements transparent-source-only TEX addresses. If shielded funds are to be sent to a TEX address, a conforming sender creates two transactions: first unshield to an ephemeral transparent address, then pay the TEX destination. A flow involving a TEX deposit address does **not** qualify as “cross-chain without unshielding.” [ZIP 320, “Design considerations for Senders”](https://github.com/zcash/zips/blob/main/zips/zip-0320.rst)

### PCZT: split transaction work, with explicit confidentiality boundaries

Partially Created Zcash Transactions separate creation, updating, proving, signing, combining, redaction, finalization, and extraction among distinct entities. This is useful for cold/hardware signing, local policy checks, and coordinated transaction assembly. The current crate's role module explicitly says its **Constructor** role still lacks an implementation; do not promise generic multiparty input/output contribution as a trivial off-the-shelf flow. [PCZT README](https://github.com/zcash/librustzcash/tree/main/pczt), [roles](https://github.com/zcash/librustzcash/blob/main/pczt/src/roles.rs)

The redactor can remove recipients, values, full viewing keys, witnesses, derivation paths, proprietary fields, and other information. The existence of a redactor demonstrates why the intermediate transaction format needs a privacy policy: a PCZT can contain data that the final chain transaction deliberately hides. It is not safe to upload an arbitrary PCZT to a public “helpful prover” or log it. A prover can need sensitive witness material; redaction does not magically permit a zero-disclosure remote proving service. [Orchard-protocol redactor](https://github.com/zcash/librustzcash/blob/main/pczt/src/roles/redactor/orchard.rs)

Use a current released version. PCZT 0.10.0 adds explicit randomness parameters, zeroization, transparent input consistency checks, and restrictive default transparent sighash policies. Its NU-Tachyon/V7 support is explicitly experimental behind a flag. That is not evidence of activated V7 consensus. [PCZT 0.10.0 release details](https://github.com/zcash/librustzcash/blob/main/pczt/CHANGELOG.md)

### Node, indexer, wallet, and UI are distinct layers

| Component | Job | Consequence for this hackathon |
|---|---|---|
| Zebra | Full consensus validation and chain state | Use it when the product needs a trustworthy chain view. It is not itself a spending wallet. |
| Zaino | Rust indexer serving wallet/explorer data from Zebra, including lightwallet APIs | Strong base for notification, reliability, indexing, and protocol conformance tooling. Stable 0.10.1 has indexed-tip subscriptions including same-height reorgs. |
| lightwalletd | Compact-block and transaction interface for light wallets | Reduces bandwidth; the server remains a metadata observer. It can use Zebra as its backend. |
| `librustzcash` | Crates for keys, addresses, transactions, scanning, wallet DB abstraction, SQLite backend, payment URIs, PCZT | The primary implementation toolkit. Parsing a transaction is explicitly **not** complete consensus validation. |
| Zallet | Rust full-node wallet replacing zcashd's wallet | Currently beta; check implemented RPCs rather than assuming zcashd API parity. Useful for server/local merchant integrations. |
| ZODL SDKs / Slipstream | Mobile-facing SDKs and a newer Rust wallet synchronization engine | Strong mobile route, but time, licensing, build environment, and current feature support must be checked. |
| Browser/Expo UI | Product interface and wallet handoff | Does not automatically provide native wallet proving/scanning. A working wallet handoff or local backend is a safer four-day scope than a new browser wallet. |

Sources: [Zebra](https://github.com/ZcashFoundation/zebra), [Zaino stable release](https://github.com/zingolabs/zaino/releases/tag/0.10.1), [lightwalletd](https://github.com/zcash/lightwalletd), [librustzcash README, warning and crate map](https://github.com/zcash/librustzcash), [Zallet](https://github.com/zcash/wallet), [Zebra's zcashd API migration guide](https://zebra.zfnd.org/user/zcashd-api-migration-guide.html), [Slipstream](https://github.com/zodl-inc/slipstream).

**Stack compatibility correction:** “latest Zallet + latest Zebra” is not a verified working configuration. Zebra 7 changes its state database to v29; released Zallet beta.3 uses an older dependency cohort, and its Zaino RPC backend was subsequently removed from development `main`. Current development platform documentation states that no chain backend runs on native Windows/macOS; the remaining direct Zebra-state backend requires Linux and an appropriate indexer-enabled Zebra build. This Windows workspace therefore needs a tested Linux/WSL2/container setup for that Zallet path, or a different already-working wallet/SDK integration. Do not infer a synchronizing wallet from passing launcher/unit tests. See the [cross-chain report's compatibility addendum](./crosschain.md#zallet-is-useful-but-install-latest-zallet--latest-zebra-is-unsafe-planning), [Zallet beta.3 backend manifest](https://github.com/zcash/zallet/blob/v0.1.0-beta.3/backends/zebra/Cargo.toml), and [current platform documentation](https://github.com/zcash/zallet/blob/main/book/src/guide/platform-support.md). The addendum distinguishes source-verified mismatch risks from configurations actually executed; this research did not run this stack.

Zaino 0.10.1's `IndexedTipService/SubscribeIndexedTips` reports the current indexed tip and subsequent canonical tip changes, including a same-height reorg. A slow subscriber receives only the latest tip, so a consumer must compare block hashes and reconcile missing intervals; it cannot assume every intermediate event was delivered. This is especially useful for payment confirmation tooling.

The lightwallet protocol provides block ranges, individual full transactions, broadcasts, tree states, subtree roots, and mempool streams. Requesting a specific full transaction can reveal interest in it. Standard RPC API presence does not imply that querying it is privacy-neutral. [Current lightwallet service definitions](https://github.com/zcash/librustzcash/blob/main/zcash_client_backend/lightwallet-protocol/walletrpc/service.proto)

The Rust backend has Tor support and isolated client handles. A useful product can apply separate circuits to different activity classes, but Tor is not a guarantee against a global traffic-correlation adversary. Multiple indexers can help detect disagreement or outages; they do not turn a light wallet into a consensus-validating node. [Tor client](https://github.com/zcash/librustzcash/blob/main/zcash_client_backend/src/tor.rs)

## Four Core & Tooling ideas with distinct wedges

The strongest shortlist is **C1 Sieve, C3 Darkroom, and C4 Epoch Rehearsal**. C2 is useful infrastructure but is downgraded after finding substantial existing implementations; it is better consumed as a building block for another product. Each selected idea needs a concrete distinction from the existing tooling below.

### C1. Sieve — a privacy firewall for partially created transactions

**User and problem:** Wallet integrators, hardware-signing teams, and any application coordinating transactions across devices. Their finished transaction can be shielded while an intermediate PCZT sent to a coordinator leaks recipients, values, full viewing keys, or derivation paths. Approval UIs also often fail to explain hidden data sharing.

**Product:** A local Rust library and inspector that enforces a declared privacy policy at every transaction handoff. The policy says which actor may receive which fields and what the final spend is allowed to do. It produces a human-readable exposure report and a minimal redacted PCZT; the signer checks payment semantics against the user's approved intent before signing.

**Creative composition:** PCZT's separable roles + its actual redaction capabilities + an address/pool-aware transaction policy + executable adversarial fixtures. The innovation is making off-chain privacy boundaries auditable, not inventing new cryptography or a new signing protocol.

**Architecture:** App creates intent locally → wallet proposes PCZT → policy inspector verifies recipients, amounts, network, branch/version, fees, pool crossing, and transparent outputs → per-role redaction profile → separate signer/prover as explicitly trusted → final transaction extraction → broadcast. Reports contain rule IDs and aggregate field classes, not raw secrets.

**Four-day MVP:** Ironwood-only single-payment policy; one coordinator/signer handoff; three injection cases (unexpected transparent output, changed recipient, accidental FVK/proprietary-field export); export redacted PCZT; demonstrate the intended payment on testnet with a supported wallet/backend. Use the released crate's own verification paths instead of attempting full consensus validation.

**First-hour kill test:** Compile a pinned PCZT 0.10.0 fixture generator and inspector; create or load one legitimate Ironwood PCZT; enumerate sensitive fields; prove that the chosen role can still complete after the proposed redaction; demonstrate rejection of one unexpected FVK/proprietary-field export. If the team cannot obtain a real compatible PCZT or the intended privacy boundary requires removing fields essential to the next role, narrow the policy to an inspect-only handoff auditor instead of promising a private remote prover. Prerequisites are the crate's actual [Verifier](https://github.com/zcash/librustzcash/blob/main/pczt/src/roles/verifier/mod.rs), [Ironwood verification adapter](https://github.com/zcash/librustzcash/blob/main/pczt/src/roles/verifier/orchard.rs), [redactor](https://github.com/zcash/librustzcash/blob/main/pczt/src/roles/redactor/orchard.rs), and [end-to-end fixtures](https://github.com/zcash/librustzcash/blob/main/pczt/tests/end_to_end.rs).

**Demo moment:** Two proposals produce the same innocent-looking “Pay 0.01 ZEC” screen. Sieve exposes that one exports an account viewing key or adds an unintended transparent output, blocks it, then completes the correctly redacted transaction.

**What to measure:** Number/types of sensitive fields transmitted, successful transaction extraction after allowed redaction, tampering cases rejected, amount/pool/recipient approval consistency. Do not use a vague numerical “privacy score.”

**Limits:** A remote prover may require sensitive witnesses; Sieve reports that trust boundary rather than claiming redaction removes it. It cannot force an independently malicious app to use the library, and it is not an audited wallet. No blind signing if necessary display/verification fields were redacted.

**Sponsor integration:** Useful as an adapter safety layer around any verified wallet/settlement partner, but sponsor credit requires that partner's actual supported signing/transaction interfaces. Do not invent PCZT support in an embedded-wallet sponsor. A demonstration that blocks a transparent route is a useful safety feature, but does not mean the route qualifies for a shielded sponsor track.

**Why this beats generic multisig:** Existing public projects already implement FROST treasuries and escrow. This targets the less visible problem of what collaborators learn before the transaction reaches the chain.

**Originality boundary:** [Zcash Durable Payment Outbox](https://github.com/devfola/zcash-durable-outbox) already advertises an independent PCZT/transaction verifier checking recipient, change, fees, memo, expiry and proofs. Sieve must therefore emphasize **per-role confidentiality and auditable redaction**, not claim the first transaction intent verifier or durable payment outbox.

### C2. Receipt Relay — private payment events that survive reorgs

**Research disposition: do not select as a standalone hackathon idea without an additional substantial mechanism.** [zcash-payment-service](https://github.com/nerdcash/zcash-payment-service) already advertises UIVK allocation, scanning, short-reorg overlap, receipt maturity, HMAC webhooks and retries. [Zcash Payment Sessions](https://github.com/Iniwura/zcash-payment-sessions) advertises exact amount/memo matching, replay prevention, confirmation states and reorg safety. These are primary-source project claims, not audited guarantees, but they significantly reduce novelty. The architecture below is still useful as infrastructure for a differentiated product.

**User and problem:** Merchants, marketplaces, donation programs, or access-gated services want Stripe-like “paid” events. Shielded payments cannot be verified by looking up a public receiving address, and sending an account viewing key to a hosted service compromises unrelated financial activity. Simple webhooks often double-fulfill after retries or mishandle reorganizations.

**Product:** A merchant-owned/local receive-only payment observer that turns locally decrypted notes into signed, narrowly scoped invoice events. The commerce backend learns the invoice ID and status; the external event relay does not get the account viewing key or the entire wallet ledger.

**Creative composition:** ZIP 321 opaque invoice nonces + shielded note scanning + Zaino's indexed-tip/reorg stream + an idempotent invoice state machine + minimal signed events. This turns native privacy into practical commerce infrastructure.

**Architecture:** Merchant generates invoice and shielded receiver; URI contains only opaque invoice reference → buyer pays using current wallet → local observer scans with merchant-held viewing material and verifies exact output/amount → trusted Zebra-backed chain view supplies confirmation state → outbox emits authenticated events (`observed`, `confirmed`, `reorged`, `expired`, `reconfirmed`) keyed by invoice and actual note/transaction identity → app fulfills once according to its explicit policy. Redelivered events are idempotent.

**Four-day MVP:** One product checkout and one merchant account; one normal payment; duplicate callback; one underpayment; one synthetic reorg using darkside fixtures; local signer for receipts; a tiny TypeScript consumer. Testnet demonstrates real receipt scanning, while adversarial/reorg behavior is demonstrated with clearly labeled local fixtures.

**Demo moment:** A customer pays shielded ZEC, the shop grants access, a replayed event does nothing, then a simulated chain reorg correctly downgrades payment status instead of silently continuing to call it final.

**What to measure:** No viewer key leaves the local boundary; duplicate fulfillments remain zero; each reorg state transition reconciles; amount and receiver verified; event payload omits real identity and wallet balances. Show the public transaction view alongside the private merchant view to explain who knows what.

**Limits:** The merchant still knows its own sale and may know customer identity from delivery. A merchant-signed receipt is an attestation by that merchant, **not** a trustless public proof of payment. Automatic refunds cannot assume a memo reply address is the original spender; MVP refunds require explicit authenticated customer confirmation. The consumer must rescan gaps because Zaino's slow subscribers receive the latest tip rather than every intermediate tip.

**Sponsor integration:** Natural infrastructure for partner checkout/onramp/swap paths **if** they truly deliver into the shielded pool. Payment route privacy must be evaluated separately from the receipt system. Also reusable by private-market products after their off-chain matching settles.

### C3. Darkroom — an adversarial privacy regression lab for wallets

**User and problem:** A wallet team can pass balance and transaction tests while leaking which payment belongs to a user through transaction lookups, timing, transparent fallback, memos, or analytics. The brief's privacy gate makes this a direct developer need.

**Product:** A deterministic lab with an observer's view of controlled wallet activity. It checks whether a proposed wallet flow reveals specified secrets to a lightwallet operator or application backend and reports precisely how the inference was made.

**Creative composition:** Existing darksidewalletd fixtures + instrumented gRPC proxy + known synthetic identities and transactions + user-defined privacy invariants + side-by-side observer traces before/after a mitigation. The novelty is a reproducible **privacy** test suite, not merely recreating darksidewalletd.

**Architecture:** Synthetic wallets execute a scenario against a local instrumented server; ground truth stays in the test runner; observer receives only request method, requested txid/range, timing, and application event data; attack rules flag direct linkage and overdisclosure. A patched client profile uses only justified mitigations such as isolated transport contexts, consistent request patterns, local verification, or reduced app logging.

**Four-day MVP:** Three cases: (1) one full-transaction fetch immediately following trial decryption, linking the wallet to a transaction; (2) checkout or telemetry logs exposing a supposedly private memo; (3) an unsupported shielded route silently falling back to transparent. Include one successful mitigation and one honestly unresolved limitation. Export CI-friendly JSON and an HTML trace visualizer.

**First-hour kill test:** Run a local lightwallet-compatible client through an instrumented proxy and capture one synthetic wallet's actual `GetTransaction` or transaction-broadcast request. Replay the same fixture and obtain the same inferred linkage. If the client cannot be instrumented or the attack is only a cartoon disconnected from its real request behavior, narrow to the deterministic checkout/URI/log disclosure analyzer; do not spend the deadline building a full anonymity simulator. Prerequisites are the real [RPC service definitions](https://github.com/zcash/librustzcash/blob/main/zcash_client_backend/lightwallet-protocol/walletrpc/service.proto) and [darksidewalletd control interface](https://github.com/zcash/lightwalletd/blob/master/docs/darksidewalletd.md).

**Demo moment:** From server-visible traces alone, the adversary identifies the buyer's transaction. After a specific change, the direct identifying signal disappears and the tool explains what metadata remains visible.

**What to measure:** Exact observable fields, candidate transaction set under a stated local adversary, deterministic pass/fail invariants, overhead of mitigation. Do not claim global anonymity from a toy anonymity set or assume adding arbitrary decoy requests proves privacy.

**Limits:** Darksidewalletd accepts artificial blocks and is for testing only; it cannot certify consensus correctness. Network anonymity results depend on the modeled adversary. Do not collect real users' transaction metadata to populate the demo.

**Sponsor integration:** A particularly credible way to compare privacy of sponsor-supported routes. Integration means running real documented partner interfaces in a local/test environment and displaying the actual boundary, not awarding a false “private” badge because the final asset is ZEC.

### C4. Epoch Rehearsal — wallet migration and recovery compatibility tests

**User and problem:** The ecosystem is moving from old Orchard balances to Ironwood while NU7 changes block spacing and invalidates old transaction formats. A seed restore or delayed/offline transaction can fail in ways that a green “synced” indicator misses.

**Product:** A conformance suite and explanatory local dashboard that rehearses wallet restore, transaction expiry, branch changes, and pool migration before upgrades. It validates wallet behavior against published released APIs plus explicitly labeled draft guidance.

**Creative composition:** The current migration engine + cross-wallet recovery fixtures + simulated activation boundaries + semantic comparison of recovered notes and balances. This solves a timely ecosystem transition while producing a reusable framework for future upgrades.

**Four-day MVP:** Three scripted fixtures: restore a post-NU6.3 account with a conservatively early birthday; resume a prepared payment across a branch-ID boundary; compare a naive full-balance pool migration with denomination/cohort scheduling. Show incorrect behavior and a failing invariant, then the current correct library path. Keep migration-policy simulation separate from broadcasting real customer funds.

**First-hour kill test:** Build one current wallet-backend fixture that creates an Ironwood receipt and restores it from the same seed with an early birthday; compare the recovered note identity/value. Then set a synthetic activation map and show a saved transaction's branch/expiry mismatch is detected. If the team cannot exercise real current wallet APIs and only has a mock balance display, reduce the MVP to a release/version conformance runner for existing wallet crates and document unimplemented recovery cases. Prerequisites are [released backend APIs](https://docs.rs/zcash_client_backend/0.25.0/zcash_client_backend/), [network consensus parameters](https://github.com/zcash/librustzcash/blob/main/components/zcash_protocol/src/consensus.rs), [ZIP 326 recovery rules](https://github.com/zcash/zips/blob/main/zips/zip-0326.md), and the existing [pool migration crate](https://github.com/zcash/librustzcash/tree/main/zcash_pool_migration).

**Technical reason this is real:** ZIP 326 says incoming receiver/key scope is Orchard-protocol-wide and recovery may require considering both `use_qsk` variants until evidence narrows them. An early birthday must not cause Ironwood notes to be skipped. ZIP 318 warns that naively migrating an entire balance exposes it and proposes denomination/cohort scheduling; the current Rust repository already contains a pool-migration engine, so the project should test and explain integration rather than rewrite that engine. NU7 changes target spacing from 75 to 25 seconds, so block counts must not be treated as a fixed wall-clock duration. [ZIP 326](https://github.com/zcash/zips/blob/main/zips/zip-0326.md), [ZIP 318](https://github.com/zcash/zips/blob/main/zips/zip-0318.md), [ZIP 259](https://github.com/zcash/zips/blob/main/zips/zip-0259.md)

**Limits:** ZIP 318 and 326 are Draft guidance even though the relevant NU6.3 consensus changes are Final. A test result must label the exact policy/version tested. This is not a promise that quantum recovery is currently an ordinary wallet operation or that a successful restore test guarantees all future recovery situations.

## Feasibility rules for ideas in the other tracks

### Shielded Payments

- A web invoice/QR flow plus an existing wallet and local merchant watcher is feasible in days. A fully new browser wallet with safe key handling, scanning, proving, hardware support, and production recovery is a much larger undertaking.
- A native Expo application does not get a Zcash SDK merely by adding a JavaScript package. Budget native bridging/build work if integrating Rust/Kotlin/Swift. The research supports wallet handoff or a narrow backend/local service as the fastest credible first demo.
- Periodic transfers/subscriptions can be scheduled by a wallet or service, but Zcash does not thereby provide an autonomous on-chain subscription contract. State who holds signing authority, who must be online, and how cancellation works.
- Sponsor-funded fees or batched disbursements must be demonstrated using the actual wallet construction API; not all wallet payment-URI implementations support multipay or arbitrary note selection.
- Avoid pay-to-unlock ideas that make the final public action immediately identify the buyer. Use scoped opaque access capabilities and explain service-side visibility.

### Private Markets

- A private RFQ, auction, orderbook, or recommendation engine can keep orders off chain, perform computation in a separately protected environment, and settle in shielded ZEC. It must explicitly state who sees plaintext orders and who can steal, censor, or mis-settle.
- The encrypted memo is an application payload; it does not enforce fair exchange, escrow release, auction correctness, or cross-chain atomicity.
- An MPC/FHE/TEE match engine is valuable only if the threat model and the output disclosures match the real market need. Zcash settlement and private matching protect different information at different times.
- FROST threshold signing can distribute custody, but does not itself prove that signers obeyed an auction or lending rule. Generic multisig treasury/escrow is already crowded.
- A zero-knowledge statement “I own a note with value at least X” is not automatically proof of total solvency, exclusive collateral, or absence of debts. A note can be reused as evidence across applications unless the application enforces domain-bound uniqueness and spend tracking, and the proof must bind to a trustworthy chain root.

### Cross-chain without unshielding

- Draw every custody/asset leg. If any step spends to a transparent Zcash address, a TEX address, or a transparent deposit, the end-to-end flow has unshielded even if the final output is private.
- “Bridge to EVM then reship into a shielded pool” is not equivalent to preserving shielded ZEC throughout. A public source/destination chain may reveal its own transfer details independently of Zcash.
- If a relayer watches a shielded deposit, it needs appropriate viewing/verification capability. That introduces a disclosure or a specialized proof-design problem; public RPC cannot simply detect arbitrary shielded deposits by address.
- Liquidity networks and threshold custodians can implement useful products with explicit trust, but should not be called trustless atomic swaps without an actual verified protocol covering refunds, timeouts, griefing, chain reorgs, and failure recovery.
- Native shielded assets/stablecoins are not assumed: ZIP 226/227 remain proposals/drafts. A memo naming USDC does not create private USDC on Zcash.

### Wildcard

- Strong directions combine shielded value with a scarce access right, sensitive relationship, or verifiable service result; they should do more than attach a donation button to an unrelated app.
- Refund/reputation/identity systems need independent authentication. Shielded recipients do not inherently learn a sender address, and memo claims should be treated as untrusted input.
- Narrow selective disclosure is valuable, but a viewer key shares a scope of history, not an arbitrary SQL query result. A custom cryptographic proof needs its own correctness assumptions and implementation budget.

## Existing work and originality checks

Public repository search already finds projects described as private FROST treasury/escrow/recovery systems, including [Konclave](https://github.com/deegalabs/konclave), [Rime](https://github.com/Giri-Aayush/rime), [Steward](https://github.com/abdulhammedabdulazeez/Steward), and [Aegis Gate](https://github.com/angelraph/aegis-gate). These descriptions are evidence of existing positioning, not a security audit or confirmation of their functionality. Do not pitch “a private 2-of-3 Zcash treasury” as uniquely original without a materially different problem and mechanism.

Similarly, darksidewalletd already exists for reorg/security fixtures, Zaino already provides indexing, Slipstream already optimizes wallet sync, and the Rust repository already contains pool migration orchestration. The most credible tooling project builds on these components and solves a specific missing integration, privacy, or correctness problem.

Payment infrastructure is also crowded: [zcash-payment-service](https://github.com/nerdcash/zcash-payment-service), [Zcash Payment Sessions](https://github.com/Iniwura/zcash-payment-sessions), and [Zcash Durable Payment Outbox](https://github.com/devfola/zcash-durable-outbox) already cover substantial parts of the obvious checkout, callback, replay, and durable-send surface. The last project's README specifically lists NU7 compatibility and verified backup/restore as remaining release gates, strengthening the motivation for a focused upgrade/recovery rehearsal tool without establishing that any one project is currently unsafe.

## Suggested four-day technical gate

Before committing to an idea, prove a single complete vertical slice with pinned current releases: generate a compatible receive address, obtain testnet funds, receive and recognize an Ironwood payment, construct/hand off the next required transaction, confirm it through the intended chain backend, and inspect what each actor learned. If that slice fails, change the architecture early. After it works, add only the mechanism that differentiates the idea.

A demo should distinguish mainnet/testnet transactions from simulated adversarial states, and disclose exactly which party holds spending/viewing authority. A public explorer alone cannot demonstrate that a shielded payment went to the intended recipient and amount; pair it with the recipient's local verified view and the relevant code path.

## Source verification notes

- Live primary-source repository reads were completed on 24 October 2026. Referenced `main`/`dev` content can change; released version links are provided for current tool versions.
- The legacy readthedocs wallet threat-model page returned HTTP 403 in this research environment. No unavailable text from that page is quoted as verified; its relevant network/privacy issues were independently confirmed in current Zingolib, Zaino, Tor, lightwallet protocol, and darkside documentation.
- ZIP 258 is Final; ZIP 259 remains Proposed even though current Zebra releases contain its scheduled activation support. ZIP 316's revisions have different statuses. ZIP 318/326 guidance is Draft. ZIP 227 assets and ZIP 231 memo bundles are Draft. Avoid flattening these into one “supported Zcash feature” list.
- Consensus activation at a particular chain height is separate from a library having a code path or a ZIP becoming Final. A subsequent read of the [Blockchair Zcash statistics API](https://api.blockchair.com/zcash/stats) on 24 October 2026 returned `best_block_height: 3529220` and `best_block_time: 2026-10-24 17:37:56`, beyond NU6.3's 3,428,143 and below NU7's 3,543,405. This is third-party explorer corroboration, not independent consensus validation by our own node. Only the reported height/time are used here; unrelated supply/value statistics from that response were not validated. An implementation must still query its actual network, tip, recent block hash, and consensus branch and prove its chosen wallet/server combination works.
