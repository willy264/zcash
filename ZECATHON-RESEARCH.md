# ZECATHON: technology research and 15 differentiated build ideas

**Researched 24 October 2026 · Three concepts for each of the five official tracks.**

The most promising direction is to solve a privacy failure that survives an ordinary shielded payment: a refund that demands identity, an API key that links every purchase to usage, a market that exposes who needs a product, an intermediate transaction that leaks viewing keys, or a public-chain service whose gas funding reveals its customer's treasury.

My strongest shortlist is **Aftercare** for a practical product, **PatchPool** for a distinctive market, **Darkroom** for direct alignment with the privacy judging criterion, **QuietPass** for a strong composition of established cryptography, and **CommonCart** for a useful cross-chain demonstration. Which one is best depends on whether the team is strongest in product engineering, Rust/wallet internals, or cryptographic integration. The remaining build window favors a narrow working slice over a new wallet, bridge, or proof system.

This document is the decision guide. The supporting research contains more implementation detail, source links, adversary models, prior-art comparisons, and demo plans:

- [Official brief, rules, dates and sponsor evidence](research/brief-audit.md)
- [Shielded Payments concepts and mechanisms](research/shielded-payments.md)
- [Zcash protocol, wallet stack and Core & Tooling](research/zcash-core.md)
- [NEAR Intents, cross-chain constraints and concepts](research/crosschain.md)
- [Private Markets and Wildcard concepts](research/markets-wildcard.md)

## 1. What the brief actually rewards

The official [site content](https://thezecathon.com/darkpool/data.js) lists:

| Track | Published focus |
|---|---|
| Shielded Payments | POS, payroll, remittance, subscriptions; money moving without publishing who paid whom. |
| Private Markets | Sealed bids, order flow and settlement; public prices with private participants. |
| Cross-chain | Swaps, bridges and intents reaching other chains **without unshielding on the way through**. |
| Core & Tooling | Zebra, librustzcash, indexers, wallet SDKs and provers. |
| Wildcard | Art, games, social and hardware that make a shielded transaction worth making. |

The four judging criteria are **Privacy, Usefulness, Execution, Originality**. There are no published numerical weights. The important language is “**Leaks are disqualifying, not deductions**,” “Would someone use this on Monday,” and “Working beats ambitious.” Originality also means code authored during the event rather than an existing product repackaged for submission.

Submit in **exactly one track**. A product may use technologies from several ecosystems, but that does not create multiple main-track entries.

The published deadline is **28 October 2026 at 23:59 UTC**, which is **29 October at 00:59 in Lagos**. On the research date roughly four days remain. Code must be open source by submission. The current [account-backed form](https://thezecathon.com/darkpool/forms.js) requires a working demo **and** a two-minute video, repository URL, one track, and a description capped at 2,000 characters. Its prompt asks what works, why it does not leak, and what is real versus mocked. Prepare both demo and video despite an older fallback checklist saying “or.”

The headline pool is **$100,000**. The source currently lists five $15,000 track allocations, a $20,000 additional grand prize, and $5,000 community bounties; a source comment calls that allocation a **placeholder split**. Treat the exact split as unresolved rather than guaranteed.

### Sponsor-track finding

**No named sponsor-specific tracks or bounty requirements were published in the materials inspected.** The source has six unnamed slots; its [screen renderer](https://thezecathon.com/darkpool/os.js) says sponsorship is not yet public and the slots fill on October 29, after submissions close. The organizer's [announcement](https://x.com/zksnarks_/status/2104606505177899314) corroborates the event and $100,000 headline, without establishing named sponsor challenges.

ZODL, NEAR Intents, Zebra and librustzcash are linked developer resources. That does **not** establish sponsorship. Section 4 maps meaningful integrations; any claim to a sponsor prize remains conditional on a separate official brief. This research does not invent sponsors, bounty amounts, API requirements, or prize stacking rules.

## 2. The technology model that should shape the ideas

### Zcash: private value transfer, with explicit application boundaries

A shielded payment uses private notes, commitments, nullifiers and proofs. Commitments and encrypted note data record value without exposing a conventional public account balance; nullifiers stop notes being spent twice. The wallet must discover its notes and maintain the state needed to spend them. A public block explorer cannot tell a merchant that a hidden output paid its invoice.

Orchard uses Halo 2 and Pasta curves. This does not provide a general EVM-like private contract runtime. An application cannot put “release funds when this patch passes tests” in a memo and expect consensus to enforce it. Auctions, conditional refunds and service delivery need an application coordinator or a separately implemented enforcement protocol. [Protocol design](https://zcash.github.io/orchard/), [ZIP 224](https://zips.z.cash/zip-0224).

**Current upgrade matters:** [ZIP 258](https://zips.z.cash/zip-0258) specifies NU6.3 at mainnet height 3,428,143 and introduces Ironwood. After activation the older Orchard pool cannot receive new value or perform cross-address payments. New payments using Orchard-protocol receivers need a compatible Ironwood path; Sapling and transparent transfers are distinct possibilities, so “all Zcash payments use Ironwood” would be incorrect. A read-only [Blockchair query](https://api.blockchair.com/zcash/stats) during research reported mainnet height 3,529,220, corroborating passage of the activation height; this is explorer evidence, not our own consensus validation.

Shielded transaction timing, structure, and net value crossing between pools can remain public. Moving an entire balance from one shielded pool to another can expose that movement amount even without a transparent address. “No transparent intermediate” and “no metadata leakage” are different claims.

### The useful primitives—and what they do not prove

| Primitive | Useful contribution | Limit that changes product design |
|---|---|---|
| [Unified Addresses / ZIP 316](https://zips.z.cash/zip-0316) | One address can contain supported receiver types. | A `u…` prefix alone does not prove a fully shielded route; inspect the selected receiver, inputs, change and pool. New revision-2 encodings are Draft. |
| Incoming/full viewing keys | A receive-only service can recognize payments without spending them. | Viewing keys reveal an account scope of activity. Diversified addresses do not make a viewing key a one-invoice proof. |
| [ZIP 321](https://zips.z.cash/zip-0321) | QR/deep-link payment requests with amounts and memos. | Not payment confirmation, recurring debit authority, or guaranteed cross-wallet interoperability. Memos in a URI are visible to its viewers before encryption on-chain. |
| [512-byte memos](https://zips.z.cash/zip-0302) | Opaque invoice IDs, commitments, small protocol payloads. | Not bulk storage, authenticated sender identity, or smart contracts. Larger memo bundles remain Draft. |
| [PCZT](https://github.com/zcash/librustzcash/tree/main/pczt) | Divide transaction construction, proving, signing and redaction between roles. | Intermediate data can expose recipients, values, witnesses or viewing keys. A remote prover is not automatically blind. |
| [FROST / ZIP 312](https://zips.z.cash/zip-0312) | Threshold control over appropriate signing authority. | Does not enforce auction correctness, hide all coordinator data, or create cross-chain atomicity. Current pool/wallet integration must be verified. |
| [Blind RSA / RFC 9474](https://www.rfc-editor.org/rfc/rfc9474.html) | Issue an entitlement without recognizing its final serial when redeemed. | Requires carefully scoped common keys, replay protection, recoverable issuance and metadata separation. Does not prove unique humanity. |
| [ZSA / ZIP 227](https://zips.z.cash/zip-0227) | Proposed native shielded assets. | Draft; do not base the MVP on live native shielded USDC, NFTs or custom asset issuance. |

Blind credentials are useful because Zcash and token blindness solve different parts of a problem. Zcash can hide the payment graph from public observers. A blind credential can prevent an issuer from linking a later access/reward token to its issuance transcript. IPs, timing, small cohorts, request contents and uniquely scoped keys can still reconnect the contexts. [Privacy Pass architecture](https://www.rfc-editor.org/rfc/rfc9576.html) is the appropriate reference for these deployment boundaries. RFC 9474 is an Informational CFRG RFC; [RFC 9578](https://www.rfc-editor.org/rfc/rfc9578.html) standardizes Privacy Pass issuance protocols. Reusing a blind-signature primitive does not alone make an application Privacy Pass compliant.

### Node, wallet, scanner and frontend must be separated

**Zebra** validates the chain; it is not a spending wallet. **Zaino/lightwalletd** serve wallet-friendly chain data; a server can still observe requests. **librustzcash** supplies key, transaction, scanning, database and PCZT components. **Zallet** is a beta full-node wallet replacement for zcashd's embedded wallet. **ZODL SDKs/Slipstream** are mobile-facing choices. A React/Expo frontend does not automatically gain private wallet scanning/proving just by displaying an address.

Current release research found Zebra 7.0.0 and an October Rust crate cohort including `zcash_client_backend` 0.25.0, `zcash_client_sqlite` 0.23.0, `zcash_protocol` 0.11.0, and PCZT 0.10.0. That is a starting point for version selection, **not** a tested combination in this workspace. New Zebra database formats and older Zallet dependencies make “install latest everything” unsafe as an implementation plan. Current Zallet development removed its alternative backend and does not provide a runnable native Windows/macOS backend; use a proven Linux/WSL stack or a coherent wallet-library/light-client path. See the [compatibility addendum](research/crosschain.md).

NU7 activation parameters differ by network: public testnet is configured at height 4,465,026 and mainnet at 3,543,405, above the mainnet height observed during this research. Public-testnet tooling already targets the newer branch. Check the actual tip, consensus branch and recent block hash reported by the chosen wallet, faucet and server; a successful mainnet setup is not evidence that an older build will work on the current testnet.

The original ECC mobile SDKs identify 4.0.0 as their final release with support ending at NU7. ZODL successors/Slipstream have AGPL/commercial licensing arrangements. The hackathon already requires open source, but the licensing and build implications should still inform stack choice. [Swift SDK](https://github.com/zcash/zcash-swift-wallet-sdk), [Android SDK](https://github.com/zcash/zcash-android-wallet-sdk), [Slipstream](https://github.com/zodl-inc/slipstream).

### NEAR Intents: useful routing, separate privacy and custody questions

NEAR Intents allows users to specify outcomes and solvers to quote fulfillment. Its verifier settles token differences within its own ledger; external bridge deposits and withdrawals have separate risks. The current [bridge table](https://docs.near-intents.org/integration/bridging/overview.md) places Zcash on the PoA bridge. It is not appropriate to assign another bridge's trust properties to that Zcash route.

There is an unresolved primary-source conflict: [general chain-support docs](https://docs.near-intents.org/resources/chain-support.md) say Zcash is **transparent-only**, while pinned current ZODL code requests shielded refund/recipient addresses. That may indicate newer or partner-specific support, unreleased code, or stale documentation. Neither source alone proves a fully shielded deployed bridge path. The live token endpoint returned HTTP 403 in this research environment; no swap was executed. The [cross-chain report](research/crosschain.md) preserves the exact code references and validation procedure.

Confidential Intents uses a permissioned private NEAR fork, FAR, with private RPC and bridge infrastructure. Its reduced public visibility is distinct from Zcash shielding and does not establish operator blindness. Partner/solver access may be required. The documented verifier has no testnet deployment, so a Base Sepolia/Zcash-testnet demo cannot truthfully claim it settled through NEAR testnet Intents. Use a separately labeled dry quote or reserve-routing demonstration if access is available. [Verifier](https://docs.near-intents.org/integration/verifier-contract/introduction.md), [Confidential Intents](https://docs.near-intents.org/integration/market-makers/confidential-intents.md).

**The viable short-build composition:** receive native shielded ZEC, fulfill the external-chain outcome from a separate prefunded reserve, and retain/recycle the ZEC shieldedly. NEAR can route **external reserves** between supported chains. This is a trusted inventory service, not a trustless bridge. It can satisfy the mechanical “no transparent ZEC intermediate” property while the operator still sees orders and destinations; organizer acceptance of that privacy model remains unconfirmed.

## 3. Fifteen concepts

The labels below describe plausible scope for a small experienced team with roughly four days. **Narrow MVP** means one coherent workflow with existing libraries; **stretch** means substantial integration or domain work remains. None has been built or validated in this research session.

### Shielded Payments

#### P1. QuietPass — buy usage without creating a billing identity

**Customer:** a paid API, research database, search service or community internet provider that wants revenue without persistent customer tracking.

**Idea:** one shielded payment buys a pack of blind-signed, single-use access tokens. The merchant reconciles the sale; the API validates a token later without its serial identifying the purchase. No permanent account or invoice-linked API key is required. This protects the relationship between payment and subsequent usage, which ordinary shielded checkout leaves exposed.

**Composition:** ZIP-321 wallet payment → locally verified confirmed receipt → exact-count blind issuance → local client storage → signature verification and atomic serial consumption at the service. Use common denomination/epoch keys, not customer-specific keys. Recoverable issuance must return the same authorized response after retries.

**Narrow MVP/demo:** three clients buy credits for one real document-conversion endpoint; redeem from a separate session; reject duplicate issuance and token replay. The known blind-signature protocol supports cryptographic unlinkability; the dashboard merely illustrates the separation. The provider still sees request content and network metadata. Confirmations are probabilistic: issued anonymous credits cannot be selectively revoked after a reorg without adding linkage, so the merchant bears that residual risk.

**Natural integrations:** ZODL handoff, librustzcash scanner, Zebra/lightwalletd-compatible data, an established blind-RSA library. Strong compositional privacy; moderate crypto-integration risk.

#### P2. Aftercare — private purchases with private returns and warranties

**Customer:** a merchant whose privacy-conscious customers need refunds, repairs or replacements without email registration or wallet-history disclosure.

**Idea:** a paid order issues a receipt bound to an ephemeral customer key. Later that key authorizes an aftercare action and a fresh shielded refund destination. The merchant needs to recognize one valid receipt, not establish a permanent customer identity or infer a nonexistent public shielded sender address.

**Composition:** local capability key → order commitment in a short payment memo → merchant recognizes the actual paid output → merchant signs an entitlement receipt → claimant signs a domain-separated refund request → merchant-controlled payout with durable reservations and identical-byte rebroadcast. The receipt export securely preserves the capability key, not merely a PDF-like receipt.

**Narrow MVP/demo:** buy one item, close checkout, recover the local receipt, get a shielded refund, then reject a replay and an altered destination. The merchant still knows its order and can refuse service under its policy; there is no autonomous on-chain refund contract. Delivery information may identify the customer independently.

**Natural integrations:** existing wallet, ZIP 321, a compatible scanner/signer, durable outbox patterns. **Best practical Payments entry** because the product remains useful after the sale and does not require custom cryptography.

#### P3. CoverDrop — separate aid eligibility from payout identity

**Customer:** a university hardship fund or community grant program that must check eligibility without maintaining one database joining sensitive circumstances to payment history.

**Idea:** the eligibility office blind-signs a standard entitlement; a separately operated payout service redeems it for shielded ZEC. The issuer knows who qualifies. The payer knows the token and destination. Neither protocol needs a shared person-to-payment identifier.

**Composition:** existing eligibility check → one blind issuance per program period → separate-session redemption → fixed-denomination shielded payout → spent-token registry and funded program cap. The authorized key policy determines value; the blind signer cannot inspect or trust hidden client-provided amounts.

**Narrow MVP/demo:** synthetic eligible roster, three issued credentials, a real shielded payout, duplicate issuance/redemption rejection, and a crash-safe pending payout. The payer still keeps token-to-address records. Timing, IPs, unusual grant sizes and small cohorts can reidentify recipients; blindness does not solve Sybil attacks or guarantee nontransferable benefits.

**Natural integrations:** wallet/scanner plus established blind credentials; later threshold fund governance if useful. Strong social utility; more deployment assumptions than Aftercare.

### Private Markets

#### M1. QuotaShade — auction capacity without linking later usage to bids

**Customer:** a small provider selling scarce API/compute bundles and companies that do not want competitors to learn their demand or bidding history.

**Idea:** a sealed, uniform-price auction allocates standardized service units. Winners pay shieldedly and receive blind quota credentials. A public clearing price supports price discovery; later usage does not carry the bidder's invoice identifier.

**Composition:** salted bid commitments/encrypted bids → explicitly trusted MVP auctioneer → private allocation/invoice → shielded payment → blind quota issuance → real API redemption. State the tie rule, capacity and what happens when a winner fails to pay. The operator can see bids; an unfinished MPC claim would weaken the entry.

**Narrow MVP/demo:** five bidders, three units, one visible rate, two paid winners and actual endpoint usage; an unpaid award expires. The service sees workload content. Common packs/epochs and separate sessions reduce correlation but do not guarantee network anonymity. Token delivery is not atomic with payment.

**Natural integrations:** wallet/scanning stack and blind RSA. More distinctive than another generic sealed-bid auction because it completes the allocation-to-consumption workflow. Overlaps QuietPass's primitive but solves scarce-resource pricing rather than fixed-price checkout.

#### M2. PatchPool — competitors privately buy a shared fix into public availability

**Customer:** companies that need the same dependency patch, dataset or license release but do not want to reveal their stack or strategic dependency.

**Idea:** a maintainer names a public buyout price and commits a testable deliverable. Organizations make private funded pledges. Once the threshold is met, a trusted escrow releases the seller's payment and publishes the artifact under explicit release terms. Failed pools refund contributors shieldedly.

**Composition:** encrypted artifact + content hash + acceptance tests → escrow checks deliverable → shielded conditional pledges with ephemeral claim keys → coarse threshold indicator → artifact/key/license release or private refunds. Coarse progress matters: continuously publishing exact totals can expose individual pledges through differencing.

**Narrow MVP/demo:** three organizations quietly fund a real patch; public release turns a failing reproduction green. A second failed pool refunds a contribution. The coordinator sees pledges and controls escrow; a hash establishes byte identity, not quality or copyright ownership. Public release retains a free-rider problem, so start with buyers who gain enough value from fixing the dependency to contribute anyway.

A signed pledge receipt acknowledges a contribution; it does not prove an honest hidden threshold calculation. Define a confirmed-funding cutoff, fees, excess/late-payment refunds and an operator loss policy for a reorg after irreversible artifact publication.

**Natural integrations:** wallet stack, encrypted storage, signed manifests; FROST only as a later independently operated custody layer. **Most distinctive market concept**, provided the demo includes an actual useful artifact.

#### M3. CohortBid — private procurement of research participation

**Customer:** a researcher who needs qualified participants and compensation without joining identity, sensitive answers and payment history in one public or centralized record.

**Idea:** eligible respondents submit private compensation bids; a reverse auction fills a cohort at one published rate. Accepted responses earn blind reward credentials, redeemed later for shielded payouts. Identity qualification, answers and payout addresses occupy separate roles.

**Composition:** eligibility issuer → anonymous participation credential → sealed bid and cohort selection → response quality/completion check → blind reward credential → shielded payout. Use distinct key domains for eligibility and reward. Publish only suitably aggregated cohort results and clear selection/compensation rules.

Use one predefined coarse result release, without overlapping subgroup queries that allow differencing; even then, collusion and identifying response content remain limitations.

**Stretch MVP/demo:** eight synthetic applicants, four places, one compensation rate, private responses and three real payouts. The recruiter sees answers and bids; answer content itself can identify a respondent. A minimum cohort size is not formal differential privacy. An auction for lower compensation is not inherently fairer; consent and compensation policy are part of the product hypothesis. A simpler demo can keep only the response-to-payout blind boundary and disclose the reduced privacy scope.

**Natural integrations:** wallet backend, blind credentials, encrypted response storage. Strong Monday customer; greatest mechanism complexity in this track.

### Cross-chain

All three proposals keep the ZEC leg shielded by using **separate operator inventory** for external-chain fulfillment. None claims atomic, operator-blind settlement. A public EVM transfer remains public. If reserves are insufficient, stop quoting rather than secretly unshielding ZEC to fund the next order.

#### X1. QuietLaunch — launch a public-chain project without exposing its old treasury

**Customer:** a pseudonymous team or researcher that needs gas and a contract deployment into a fresh account without a public funding transaction from its existing treasury.

**Idea:** pay shielded ZEC for a completed service: a bounded gas budget and deployment of an allowlisted contract template. The destination account key stays local. A relayer funds the service from its own external-chain reserve.

**Composition:** encrypted deployment manifest and signed quote → confirmed shielded invoice → idempotent relayer submission → public deployment receipt → refund of unused/cancelled service value under explicit rules. NEAR Intents can replenish the relayer's external gas reserve from stablecoins; Chain Signatures can optionally control that external signer but does not directly sign shielded Zcash spends.

**Narrow MVP/demo:** one template, Base Sepolia, one shielded test payment, fresh-account deployment, and duplicate-order refusal. The public sees the relayer and deployment. The operator knows the order/payment/destination linkage; the design does not conceal that linkage from the provider. It does not thereby expose the customer's arbitrary shielded wallet history or balance to the provider either.

**Natural integrations:** Zcash wallet/scanner, EVM SDK/relayer, separately demonstrated NEAR external reserve routing. Best developer-facing cross-chain experience.

#### X2. CommonCart — private group contributions, one stablecoin supplier invoice

**Customer:** a collective buying a shared cloud reservation, subscription or equipment order from a supplier accepting stablecoins.

**Idea:** members contribute shielded ZEC to a defined purchase. When the threshold is reached, the coordinator pays one supplier invoice from its stablecoin reserve. If the campaign expires, contributions return to authenticated fresh shielded destinations. The vendor sees one payment rather than a contributor roster.

**Composition:** signed procurement terms → private invitation/claim capabilities → confirmed shielded contributions → reserve reservation and exact invoice fulfillment → private completion receipts or refunds. Fix the ZEC quote window or define the exchange-rate buffer; a ZEC pledge is not a permanent dollar guarantee.

**Narrow MVP/demo:** three contributors, one test-USDC supplier payment, one failed campaign refund, and a participant view showing only that participant's records. The coordinator sees contributions and invoice linkage and temporarily controls funds. An operator-signed receipt is not a trustless proof of expenditure.

**Natural integrations:** native shielded wallet/scanner; external stablecoin wallet; NEAR exact-output routing for external reserve conversion where supported. **Best cross-chain product MVP** because it has clear success and failure outcomes. Direct NEAR testnet settlement is not assumed.

#### X3. Counterflow — reciprocal trade that keeps ZEC circulating shieldedly

**Customer:** a cooperative with exporters receiving stablecoins who want ZEC, and importers holding ZEC who owe stablecoin invoices.

**Idea:** match those opposing needs in bounded settlement windows. Stablecoin inflows replenish stablecoin obligations; shielded ZEC inflows replenish shielded payouts. This addresses the sustainability flaw in a demo that receives unlimited ZEC while exhausting a finite external wallet.

**Composition:** encrypted invoices and reserved quotes → two-asset inventory ledger → bounded batch matching → external supplier payments and shielded payouts → unmatched-order refund. NEAR rebalances external stablecoins between chains; insufficient native ZEC liquidity caps new orders.

**Stretch MVP/demo:** four orders, two currencies, one operator, explicit before/after reserve accounting and an unmatched cancellation. The operator sees the order graph and bears inventory/settlement risk. Public stablecoin amounts and timing remain correlatable. Netting reduces unnecessary conversions in a demonstrated batch; it does not itself prove anonymity or universal fee savings.

**Natural integrations:** supported Zcash wallet, durable accounting, external-chain SDK, NEAR reserve routing. Best long-term market mechanism, highest operational complexity of the cross-chain options.

### Core & Tooling

#### C1. Sieve — a confidentiality firewall for transaction handoffs

**Customer:** wallet, hardware-signing and payment teams passing PCZTs between app, coordinator, prover and signer.

**Idea:** the final transaction is shielded, but the intermediate file may expose an entire viewing key or private recipient details. Sieve makes each role's permitted disclosures explicit, audits the handoff and applies validated redaction while preserving required signing checks.

**Composition:** pinned PCZT library → per-role field policy → local exposure report → valid redacted handoff → signer verifies approved intent → final transaction. Do not remove fields the signer needs to verify what it signs or promise that a prover can work without its required witnesses.

**Narrow MVP/demo:** one coordinator/signer flow; reject accidental viewing-key export, an unexpected transparent output and a changed recipient; complete the allowed flow. Prior art already verifies transaction intent, so the distinction is **role-specific confidentiality and safe redaction**, not another generic verifier.

**Natural integrations:** librustzcash/PCZT, current wallet fixtures, optionally hardware/wallet adapters with verified support. Strong infrastructure concept; real PCZT generation and completion are the first-hour gate.

#### C2. Darkroom — show wallet developers exactly how their app leaks

**Customer:** wallet and payment developers whose transaction tests pass while requests, logs or transparent fallbacks expose users.

**Idea:** a deterministic test lab runs a wallet/application scenario, then gives an adversary only the metadata a real server or backend would see. It explains the concrete inference and re-runs after a mitigation.

**Composition:** existing darksidewalletd fixtures → instrumented RPC proxy → known synthetic ground truth → observer rules → CI failure and trace report. This extends existing test infrastructure with privacy assertions rather than claiming a new node simulator.

**Narrow MVP/demo:** expose a transaction-specific fetch, a memo leaked through checkout logging, and a silent transparent fallback. Fix one and show which observable disappears. Do not convert a toy candidate set into a universal anonymity percentage. At least one trace must come from a real instrumented client, not a scripted cartoon.

**Natural integrations:** lightwalletd/Zaino, Zebra-compatible fixtures, actual wallet requests, partner route adapters if documented. **Best direct response to “leaks are disqualifying”** for a team comfortable with backend/Rust tooling.

#### C3. Epoch Rehearsal — test wallet recovery across upgrades before users suffer

**Customer:** wallet maintainers and payment services navigating Ironwood, key-recovery changes and upcoming branch/version transitions.

**Idea:** rehearse seed restore, early wallet birthdays, delayed/offline transaction preparation and shielded-pool migration against versioned fixtures. Show recovered notes and invariants, not merely a green sync indicator.

**Composition:** current wallet APIs → deterministic recovery/activation fixtures → release/version manifest → semantic comparison of expected and restored notes → compatibility report. Existing migration logic should be tested and integrated, not rewritten for novelty.

**Stretch MVP/demo:** restore one real library-generated Ironwood receipt; reject a transaction prepared for an invalid branch; illustrate the value leakage of an entire-balance pool migration versus the documented migration policy. Clearly label synthetic activation maps and Draft wallet guidance. A passing fixture does not certify all future recovery or quantum security.

**Natural integrations:** librustzcash, wallet SDKs, Zebra/lightwallet fixtures and existing migration engine. Timely and potentially valuable; choose it only if the team can exercise genuine wallet state immediately.

### Wildcard

#### W1. Fogbound — a strategy game whose financial alliances remain secret

**Customer:** a small multiplayer group or event that wants hidden-coalition strategy without spectators reading alliances from a public payment graph.

**Idea:** players privately fund different factions during simultaneous turns. Public state reveals only the turn's aggregate effects. Give factions separate receiving accounts so a transparent version would actually expose payer-to-faction edges; otherwise the claimed advantage would be overstated.

**Composition:** signed game rules → separate faction shielded receivers → authenticated off-chain plans tied to short memo commitments → confirmed payments → deterministic turn resolution → private inclusion receipts. The host controls and can inspect the relevant faction accounts/actions. No prize is necessary; a host-funded fixed skill prize is optional.

**Narrow MVP/demo:** six clients, three turns, two secret rival supporters and a deterministic replay. Compare the observer-visible graph in transparent and shielded variants. The host is trusted, small groups can infer actions from outcomes, and network timing can expose players. Use turn-based timing that accommodates confirmation; do not promise instant on-chain gameplay.

Public replay verifies disclosed aggregate-to-board computation, not completeness or fairness of the host's hidden-input tally. A private inclusion receipt proves membership in the committed action set, not that every action was adjudicated correctly.

**Natural integrations:** wallet/memo scanner and a browser game engine. Strongest Wildcard explanation of why payment privacy changes the rules of the experience.

#### W2. Unowned Encore — privately commissioned collective art

**Customer:** a live performer, museum or streamer who wants paid audience authorship without a public patron/preference leaderboard.

**Idea:** viewers privately commission branches of a soundscape or visual story. The installation reveals only a coarse result for a batch. A payment changes the work; it is more than a tip beside an animation.

**Composition:** signed scene choices → shielded payment with short branch/round reference → local viewing-wallet scanner → bounded aggregation → WebAudio/canvas/projector or LED output. Never project raw prompts, per-payment timestamps or individual amounts. Small batches are merged or refunded under an explicit policy.

**Narrow MVP/demo:** collect and confirm payments before a short stage reveal, then render a live transformation and show pending payments entering the next round. A 90-second presentation should not depend on multiple new payments achieving enough confirmations instantly. The artist sees choices; payment counts are not counts of unique people, and an adversary contributing all but one choice can infer the remainder. Coarse output reduces disclosure but does not prove formal anonymity.

**Natural integrations:** existing wallet/scanner, local browser audiovisual engine, optional Raspberry Pi/ESP32 output. **Best creative stage demo**, with commercial demand still to validate.

#### W3. StreetProof — useful sensor rewards without a public earnings-and-movement graph

**Customer:** a local newsroom, university or civic buyer needing heat, noise or accessibility measurements.

**Idea:** contributors submit useful measurements to a validator, receive blind fixed-value reward credentials, and redeem them separately for shielded payouts. Publish coarse area statistics without publishing a contributor's wallet trail.

**Composition:** actual sensor/phone gateway → metadata minimization and bounded location/time → calibration/quality checks → blind reward → separate shielded payout → aggregated publication. Keep raw traces off-chain. The validator sees submitted location/time; hardware attestation can introduce persistent identity.

**Stretch MVP/demo:** use one real calibrated reference sensor, show a changed measurement, reject an invalid sample, accept and pay a contribution, then reject a duplicate reward. A signature does not prove where a device was; a shielded payout does not prove data truth. GPS spoofing, copied sensors, unique routes and Sybil abuse remain outside the demonstrated claim. Fixed funded budgets limit exposure.

**Natural integrations:** ESP32/phone gateway, existing wallet/scanner, blind credentials, a coarse map. Strong civic usefulness; only a top sprint choice if hardware and measurement experience are already available.

## 4. Meaningful ecosystem integration—and conditional sponsor fit

No cell below establishes an official sponsor or bounty. It maps a technology to work that would demonstrate it substantively if a matching sponsor brief is supplied.

| Technology / possible partner category | Best-fitting concepts | What a real integration demonstrates | What would be superficial or misleading |
|---|---|---|---|
| ZODL / existing wallets | Aftercare, QuietPass, PatchPool, Encore | Tested payment URI/memo handoff and real recipient recognition. | Displaying a wallet logo or assuming PCZT/deep-link capabilities without checking. |
| librustzcash / PCZT | Sieve, Epoch Rehearsal, payment scanners | Actual proposals, scanning, note recovery or redaction with successful downstream completion. | A mocked balance or static “verified” badge. |
| Zebra / Zaino / lightwalletd | Darkroom, Epoch, all receipt-dependent products | Real chain/RPC traces, confirmation state, reorg handling and explicit metadata boundaries. | Calling Zebra a spending wallet or calling an indexer a private balance oracle. |
| NEAR Intents / 1Click | CommonCart, QuietLaunch, Counterflow | Supported external-reserve quote/routing, exact-output behavior, refunds and custody disclosure. | Claiming a shielded ZEC bridge from a final shielded address, or a testnet swap through a nonexistent verifier deployment. |
| NEAR Chain Signatures | QuietLaunch, later treasury automation | User/contract-governed signing of a supported external-chain transaction. | Assuming secp256k1/Ed25519 MPC directly authorizes an Ironwood shielded spend. |
| FROST / threshold custody | Later PatchPool or CoverDrop governance | Real independent signer cooperation with a current compatible transaction. | “2-of-3” processes on one machine claimed as independent security or auction enforcement. |
| Blind-signature libraries | QuietPass, CoverDrop, QuotaShade, CohortBid, StreetProof | Correct issuance/unblinding/redemption, shared key policy, durable retries and replay rejection. | Signing hidden client-selected token values and calling every resulting entitlement valid. |
| Hardware / sensor partners | StreetProof, Encore | Actual measurements or physical output changing from verified payment events. | A sensor picture, random synthetic measurements or a hardware device unnecessarily storing wallet seeds. |

The strongest stack composition is usually **one Zcash primitive + one complementary capability + one concrete customer problem**. Adding every partner increases failure and disclosure surfaces. The first submission should make its critical integration visible in the two-minute demo; optional extensions belong in the roadmap.

## 5. How I would choose

These are qualitative research judgments, not official scores or probabilities of winning.

| Concept | Differentiating strength | Four-day implementation outlook | Main reason to hesitate |
|---|---|---|---|
| Aftercare | Private post-purchase rights | Strong after wallet proof | Refund correctness and capability recovery |
| QuietPass | Payment-to-usage unlinkability | Good with crypto experience | Key scoping, issuance/reorg and metadata handling |
| CoverDrop | No unified identity-to-payment registry | Moderate | Eligibility trust and network separation |
| QuotaShade | Private price discovery through consumption | Moderate | Auction plus token service is two workflows |
| PatchPool | Hidden competitive demand buys a public deliverable | Good with a real artifact | Custody, release rights and free-riding |
| CohortBid | Separate identity, bids, answers and compensation | Stretch | Two credential flows and study design |
| QuietLaunch | Deliver useful public-chain service privately funded | Good with EVM experience | Operator linkage and track interpretation |
| CommonCart | Confidential group purchase with refunds | Good with wallet/EVM experience | Reserve management and price exposure |
| Counterflow | Sustainable shielded inventory circulation | Stretch | Accounting, liquidity and matching |
| Sieve | Audit data disclosed before chain submission | Good with PCZT experience | Redaction must preserve downstream verification |
| Darkroom | Demonstrate and prevent concrete privacy leaks | Strong with backend experience | Must instrument real behavior |
| Epoch Rehearsal | Timely recovery and upgrade conformance | Stretch without Rust familiarity | Version-sensitive state fixtures |
| Fogbound | Hidden financing changes strategy | Good once payment path works | Host trust and small-group inference |
| Unowned Encore | Memorable paid collective art | Strong with scanner + audiovisual skills | Meaningful aggregation and confirmation timing |
| StreetProof | Paid civic data without public contributor trail | Stretch unless hardware ready | Measurement truth and location privacy |

**My practical recommendation:** shortlist **Aftercare, PatchPool and Darkroom**, then spend the first hour on each candidate's hardest dependency before selecting one. Aftercare gives the clearest Monday-useful customer workflow. PatchPool gives the freshest product story. Darkroom turns the event's strictest judging requirement into the product itself.

Choose **QuietPass** if the team is comfortable with blind credentials and wants the clearest cryptographic composition. Choose **CommonCart** if cross-chain is a priority and a disclosed inventory coordinator is acceptable to the organizers. Choose **Unowned Encore** if the team has strong experience design and can produce a real performance tool.

### The first-hour decision gates

- **Aftercare / PatchPool / CommonCart:** receive and decrypt one compatible shielded payment with the intended memo; construct a refund; restart the worker without duplicate payment. For CommonCart, also reserve and send one external test-chain invoice payment.
- **QuietPass / QuotaShade:** run exact-scope blind issuance and single-use redemption, then attach real confirmed payment recognition. Test that a changed batch cannot be issued against the same paid invoice.
- **Darkroom:** capture one real wallet/application request through the observer proxy and reproduce the claimed leak. If it only works with a fabricated trace, narrow the claim or change the idea.
- **Sieve:** obtain a real compatible PCZT and prove the next role still works after the intended redaction. If not, deliver a precise inspection tool rather than promise a private remote prover.
- **Epoch Rehearsal:** create and restore a real library-generated note and reproduce one boundary-condition fixture. Do not spend the whole window constructing a new wallet.
- **Wildcard:** confirm the payment/sensor/controller path before designing the whole experience; pre-confirm a stage demonstration where chain timing would otherwise dominate.

### A realistic remaining sprint

**Day 1:** lock one track, adversary model and compatible stack; complete the first real payment/trace; write the shortest useful workflow. Acquire test funds and sync the wallet early. Avoid full historical node setup unless already available.

**Day 2:** implement only the differentiating mechanism and its failure states: receipt capability, blind issuance, collective release, privacy assertion or cross-chain inventory fulfillment. Keep unrelated identity and payment data out of shared logs.

**Day 3:** exercise replay, underpayment, expiry, denied permission, worker crash and any relevant reorg fixtures. Review exactly which actor can infer each relationship. Replace vague privacy claims with the actual evidence.

**Day 4:** make the repository public, prepare a usable demo, record the two-minute narrative, and submit early enough to verify the submitted state. The description should explain the problem, the working mechanism, who can still see what, and what is mocked. Choose one main track even if the architecture spans several.

## 6. Evidence, limitations and ideas deliberately excluded

Research used the official event page/public assets, current ZIPs, released library records, official NEAR documentation, wallet implementation source, RFCs, and existing project READMEs. Three parallel research agents examined protocol/tooling, cross-chain integrations, and markets/wildcard; their findings were reconciled and subjected to additional technical review. Competing repositories were read for differentiation, not assumed audited or confirmed hackathon submissions.

**No wallets were installed or synchronized, no funds were moved, no sample project was built, and no organizer was contacted.** Feasibility here is grounded in documentation and code inspection, with explicit integration gates; it is not the result of end-to-end execution. Mutable sources may change after the research date. Several links use pinned releases or commits where the distinction matters.

Generic checkout, payroll CSVs, tipping, NFT-on-Zcash claims, another sealed-auction interface, and generic FROST treasuries were not selected as primary ideas because existing work already occupies that territory or the required native feature is not established. Public comparisons include [CipherPay](https://github.com/atmospherelabs-dev/cipherpay-web), [Grove](https://github.com/davisonio/grove), [Blindfold](https://github.com/onemanbillion/blindfold), [Zeek](https://github.com/shrooms08/zeek), [ZECpages](https://github.com/michaelharms6010/zecpages), [payment sessions](https://github.com/Iniwura/zcash-payment-sessions), and [durable outbox](https://github.com/devfola/zcash-durable-outbox). The source reports explain how each selected concept differs.

Likewise, native shielded stablecoin lending, arbitrary Zcash smart-contract escrow, a new trustless shielded bridge, and a private remote prover requiring no witness disclosure were excluded from the four-day recommendations. Each would need substantially more protocol evidence and implementation work than its appealing headline suggests.

The sponsor matrix can be made eligibility-specific only when actual sponsor challenges are available. The main portfolio remains usable now: every concept has a named customer, a specific privacy mechanism, a plausible stack, an honest trust boundary, and a demo that can be tested before the deadline.
