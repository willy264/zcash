# Zecathon cross-chain research and product concepts

Research date: **24 October 2026**. Primary documentation and source code were read directly. Source URLs are collected below. No funds were moved; the live 1Click token endpoint returned HTTP 403 from this research environment, so production quote/execution support was not independently exercised.

## Decision in one paragraph

The interesting opportunity is **to deliver a cross-chain outcome while circulating ZEC inside its native shielded pool**, rather than immediately swapping each customer's ZEC through a transparent bridge. A small, explicitly trusted operator can receive a shielded payment, fulfill an external-chain service from prefunded inventory, and recycle its shielded ZEC to customers moving the other way. Three useful products are proposed below: source-protected deployment and service provisioning; private collective procurement; and reciprocal invoice settlement for trading cooperatives. These can demonstrate real transactions without inventing cryptography. They do **not** deliver trustless or operator-blind cross-chain settlement. A generic 1Click swap interface, transparent HTLC, or a private off-chain ledger should not be described as satisfying the brief's literal “without unshielding on the way through.”

## Brief and sponsor status

The official site's cross-chain wording, established in the parent research, is: **“Swaps, bridges and intents that reach other chains without unshielding on the way through.”** NEAR Intents, Zebra, librustzcash and ZODL are linked resources. This is not evidence that any is a confirmed prize sponsor. The parent investigation found six unnamed sponsor slots and official copy saying sponsors/judges would be announced October 29, after the October 28 submission deadline. Do not promise a NEAR bounty, fabricate a sponsor integration requirement, or treat a resource logo as a prize allocation.

Current Zcash compatibility matters: the parent investigation verified ZIP 258's NU6.3/Ironwood activation details and restrictions on subsequent Orchard use. Build against a currently compatible wallet SDK and active shielded pool. Do not hardcode a 2025 Orchard-only architecture. Pool migration can disclose net pool movements even when there is no transparent address. Native ZEC remains the asset; these ideas do not require Zcash Shielded Assets to be deployed.

## What NEAR Intents actually provides

### Execution and trust boundaries

1. A user specifies an outcome such as “spend X of asset A and receive at least Y of asset B.” Market makers compete by returning signed quotes.
2. The public `intents.near` verifier holds a token-accounting ledger. Matching `token_diff` intents can settle atomically **inside that ledger**. This does not make all deposits and withdrawals across external chains atomic.
3. External tokens enter and leave through bridges, each with a separate trust model. The current bridge table assigns Zcash to the **POA bridge**, not to Omni Bridge. Omni supports a different chain set. Do not attribute Omni's Bitcoin security model or NEAR Chain Signatures to Zcash simply because all appear in NEAR's ecosystem.
4. The standard Message Bus is optional infrastructure. Applications can use other quote channels, and the verifier remains separately accessible. A local shielded-ZEC RFQ endpoint is a custom application service; it is not automatically a supported NEAR solver settlement asset.
5. The official verifier documentation says **there is no testnet deployment**. Local or testnet application demos must label simulated NEAR routing clearly; a genuine 1Click integration can use read-only/dry quotes, with optional separately funded mainnet execution later. Do not claim a testnet 1Click settlement that never occurred.

Sources: [NEAR verifier](https://docs.near-intents.org/integration/verifier-contract/introduction.md), [bridges](https://docs.near-intents.org/integration/bridging/overview.md), [market makers](https://docs.near-intents.org/integration/market-makers/introduction.md).

### The Zcash-address evidence is conflicting

The current [NEAR chain-support page](https://docs.near-intents.org/resources/chain-support.md) says Zcash supports `t1`/`t3` addresses and explicitly warns **“Transparent addresses only.”** It repeats “ZEC (Zcash, transparent only)” in the asset table. This is the published general integration guarantee.

However, current ZODL Android source provides stronger, conflicting evidence than older explanatory articles:

- At commit `229dd3bff7490ea90ca4d7d79f3c539d71976f66` dated October 23, 2026, `RequestSwapQuoteUseCase` calls `requestNextShieldedAddress()` for ZEC refunds and swap-into-ZEC destinations.
- `AccountDataSource` derives these addresses using shielded/Orchard address requests; it is not merely renaming an ephemeral transparent address.
- `NearSwapDataSource` passes the shielded address through as `refundTo`/`recipient`, and its address parser accepts Unified/Shielded alongside Transparent/TEX.
- The iOS `Near1Click` adapter uses the real 1Click endpoints and identifies native ZEC as `nep141:zec.omft.near`; it also exposes Tor transport as a network privacy option.

This could reflect a newer or partner-specific capability, unreleased wallet code, or stale general documentation. **Source code is not proof of a successful deployed route.** It also does not establish that bridge custody remains shielded between the entry and exit. The honest conclusion is: *general documentation guarantees transparent support; current wallet source has shielded-facing flows; validate the exact route and transaction before claiming more.*

Before committing to direct 1Click shielded integration: obtain a real quote, decode the returned Zcash deposit receiver, validate the destination/refund address acceptance, inspect a completed transaction's actual pool/transparent components, and verify custody/rebalancing assumptions with the provider. A Unified Address string alone is not enough: inspect its receiver types and the receiver actually used. See [ZIP 316](https://zips.z.cash/zip-0316).

Pinned code: [quote use case](https://github.com/zodl-inc/zodl-android/blob/229dd3bff7490ea90ca4d7d79f3c539d71976f66/ui-lib/src/main/java/co/electriccoin/zcash/ui/common/usecase/RequestSwapQuoteUseCase.kt), [address derivation](https://github.com/zodl-inc/zodl-android/blob/229dd3bff7490ea90ca4d7d79f3c539d71976f66/ui-lib/src/main/java/co/electriccoin/zcash/ui/common/datasource/AccountDataSource.kt), [NEAR adapter](https://github.com/zodl-inc/zodl-android/blob/229dd3bff7490ea90ca4d7d79f3c539d71976f66/ui-lib/src/main/java/co/electriccoin/zcash/ui/common/datasource/NearSwapDataSource.kt), [iOS adapter](https://github.com/zodl-inc/zodl-ios/blob/bfce63e808f595f716db56d3e2c87cb4f002c259/secant/Sources/Dependencies/SwapAndPay/sources/Near1Click.swift).

### Confidential Intents is a distinct privacy system

As of the July 2026 B2B launch, 1Click accepts `confidentiality: "public" | "basic" | "advanced"`. Foreign-to-foreign swaps can set this parameter; embedded private balances use `CONFIDENTIAL_INTENTS` deposit/recipient/refund types and signed intent execution.

The underlying system is **FAR, a permissioned private NEAR fork**, with a small validator set, no public networking, private authenticated RPC, a treasury on public NEAR, and a Private PoA Bridge that mints/burns backed IMT balances. It conceals internal balances and settlement details from **public indexers**. It is not Zcash's zero-knowledge shielded pool. The docs' statement that deposits/withdrawals cannot be tracked should be understood within that architecture, not as a proof of resistance to operators, end-to-end timing correlation, or every observer.

The private relay requires partner credentials and, according to the changelog, **whitelisted solver access**. Do not make a weekend demo depend on unconfirmed access. The reviewed public documents expose `basic` and `advanced` but do not specify enough to independently compare their complete privacy guarantees; do not invent differences.

Sources: [Confidential Swaps](https://docs.near-intents.org/integration/distribution-channels/1click-api/quickstart/confidential-swaps.md), [private architecture](https://docs.near-intents.org/integration/market-makers/confidential-intents.md), [dated changelog](https://docs.near-intents.org/changelog/overview.md).

### Useful API details and pitfalls

| Capability | What it enables | Product consequence |
|---|---|---|
| `/v0/tokens` | Current token IDs/decimals | Do not hardcode an entire asset catalog or confuse native ZEC with ZEC representations on Solana/NEAR. |
| `EXACT_INPUT` | Fixed spend with minimum received | Suitable for selling balances. Below-required deposits are refunded by deadline; excess is refunded. |
| `EXACT_OUTPUT` | Fixed invoice amount with input slippage buffer | Useful for supplier payments. Excess input may be refunded; show the maximum debit separately from expected debit. |
| `FLEX_INPUT` | Variable amount within quote rules | Useful for sweeping a settling balance; confidential-balance support is not equivalent to exact-input support. |
| `ANY_INPUT` | Standing multi-token collector | Partner-only, deposits into Intents/private Intents, sweeps after a $1,000 threshold; not a universal tiny-donation checkout. |
| `dry: true` | Preview/validate without initiating execution | Does **not** return a deposit address. A successful dry quote is not a successful swap. |
| `refundTo`, `recipient`, `amount`, deadlines | Explicit execution instructions | Provider sees these data. Avoid account-wide wallet addresses or personal memo content in public logs. |
| Signed quote response | Provider authenticity and dispute evidence | Retain the full signed quote locally and verify it before using payment fields. It does not prove settlement or enforce a refund. |
| `/v0/status` | Progress keyed by deposit address (+ memo when required) | Protect identifiers; the normal explorer/history model should not be treated as anonymous. |

The official docs currently describe unauthenticated quote fees of an additional 25 bps; authenticated normal routes typically 20 bps, stablecoin/same-asset routes typically 1 bp, plus protocol and relevant network/bridge fees. Fees and partner agreements vary; take the executable quote as authoritative. September 2026 changelog entries report temporary minimums: $1,000 for foreign-to-foreign routes on nine named chains and $100 for origin deposits on Tron. **Base is not in that nine-chain list**, making Base a simpler narrow demo target, subject to actual quote availability. Never advertise guaranteed micropayment economics from token support alone.

Sources: [request lifecycle](https://docs.near-intents.org/integration/distribution-channels/1click-api/quickstart/making-a-request.md), [quote API](https://docs.near-intents.org/api-reference/oneclick/request-a-swap-quote.md), [swap types](https://docs.near-intents.org/integration/distribution-channels/1click-api/swap-types.md), [fees](https://docs.near-intents.org/resources/fees.md), [nine-chain restriction](https://docs.near-intents.org/changelog/entries/2026-09-09-temporary-1000-minimum.md), [authentication](https://docs.near-intents.org/integration/distribution-channels/1click-api/authentication.md).

### Chain Signatures does not turn Zcash into a smart-contract chain

NEAR Chain Signatures allows a NEAR account/contract to request MPC signatures for external-chain accounts. Current domains are secp256k1 and Ed25519. The derivation path, NEAR account and MPC key determine the external address. The service supplies signatures, while the application still constructs and submits transactions. It is an outbound signing service, not automatically an oracle proving incoming external state.

This is useful for an EVM service relayer or inventory wallet. It does **not** natively authorize Zcash shielded spends: those use distinct spend-authorization schemes, transaction witnesses and proofs. ZIP 312 describes rerandomized FROST compatible with Sapling/Orchard signatures and explicitly trusts coordinators/share holders for privacy. Threshold custody is not cross-chain atomicity. In the current post-Ironwood setting, verify the exact active SDK/signing compatibility as well.

For a hackathon MVP, keep shielded keys in a local wallet service using existing libraries, use a separate EVM signer, and add MPC only when it solves a demonstrated custody requirement. A NEAR contract receiving a hash of a Zcash transaction cannot infer that a hidden invoice was paid unless someone supplies a separately sound proof or a trusted attestation.

Sources: [NEAR Chain Signatures](https://docs.near.org/chain-abstraction/chain-signatures.md), [ZIP 312](https://zips.z.cash/zip-0312).

## Existing projects and why the obvious idea is crowded

These are self-published repositories, not audited endorsements. Their specific limitations are more informative than their headlines.

- **ZODL/CrossPay:** real wallet source already integrates 1Click, exact-input/output swaps, recipient/refund handling, Tor transport and multiple chains. “ZEC wallet with swaps” is not a differentiated idea.
- **Paralock:** advertises shielded-ZEC/stablecoin payments, payroll and seller-secret exchange, but the README explicitly says both available modes use a **transparent Zcash output**. HTLC mode shares a hash across chains; its experimental adaptor mode removes that particular cryptographic linkage but still uses transparent multisig, with timing/amount correlation remaining. Fully shielded FROST is research; public deployments are testnet, unaudited, and no public maker is available. [Source](https://github.com/bitfalt/paralock).
- **Hollow:** combines 1inch Fusion+ and NEAR Chain Signatures, but explicitly describes **transparent Zcash HTLCs**. Not a ready-made shielded settlement component. [Source](https://github.com/mangekyou-labs/hollow-zec-evm).
- **ZeeSwap:** documents shielded Zcash/Base testnet experiments and a reverse flow, while honestly stating that the operator controls inventory and can associate both sides. It provides a feasibility precedent for shielded inventory fulfillment, not a trustless production protocol. [Source](https://github.com/EndPx/ZeeSwap).
- **ZEC SSL:** current repository is a dashboard/API expecting a separately supplied coordinator, operator endpoints and contracts; a dashboard labeled threshold settlement is not evidence that that settlement network exists. [Source](https://github.com/SKAHLO1/zecssl).

Do not make claims about THORChain/MAYA shielded support without independently examining their current chain support and implementation. They are not needed for the recommended MVPs.

## Three distinct product concepts

All three use the same honest principle: the customer pays a native shielded ZEC receiver; the provider fulfills from **separate, prefunded external-chain inventory**. The provider may retain ZEC as earnings or pay it out shielded to opposite-direction customers. Inventory shortages stop quotes. The MVP never silently converts received ZEC through a transparent bridge. A generic “bridge” label would misrepresent this design.

### 1. QuietLaunch — source-protected on-chain service provisioning

**User/problem.** An independent researcher, pseudonymous software team or creator wants a new on-chain project without its gas funding transaction revealing the existing treasury. Even a fresh EVM address becomes associated with the old wallet when that wallet sends it gas. Ordinary swap UIs make the user assemble gas purchase, wallet setup and deployment manually.

**Product.** Pay one shielded invoice to receive a concrete service: a gas budget and deployment of a selected contract template into a newly generated EVM account. The user keeps the destination account key locally. An encrypted order specifies template, constructor arguments, destination chain, spending ceiling and expiry. The service replies with a signed quote, receives shielded ZEC, then submits the allowlisted deployment from its inventory or sponsors the new account's operation. Return a deployment receipt, remaining budget and explicit refund handling.

**Why the stack combination matters.** Zcash protects the funding history. The external chain provides programmable deployment and composable accounts. A service relayer separates the customer's fee funding from its existing public wallet. NEAR Intents can replenish the relayer's **external-chain** gas inventory from its stablecoin reserve; its route need not touch the customer's ZEC. Chain Signatures is an optional alternative external signer, not a necessary shielded-spend component.

**MVP.** One wallet integration, one current shielded test network, Base Sepolia, one contract template such as an encrypted-document hash registry, and a prefunded relayer. Prove shielded invoice receipt with the recipient wallet's scanner; lock order state before broadcasting; persist raw signed external transactions for crash-safe rebroadcast. Show failed deployment and shielded refund. Keep generic arbitrary-contract execution out of scope.

**Two-minute demo.** Create a fresh account, pay a shielded invoice, deploy a useful registry, and compare the public chain view: the service operator is visible but the payer's prior treasury is not attached by a gas top-up. Then replay the same order and show no duplicate deployment.

**Privacy/trust.** The operator learns the quote, payment and destination linkage. Public viewers see the deployment, amount of gas and relayer. Network and timing correlation remain. This protects funding history from public observers; it is not anonymous deployment against the provider. Provider holds prepaid value until fulfillment/refund.

**Novelty and commercial shape.** Sell completed on-chain services under a private funding budget, not a token swap. Start with deployment agencies or pseudonymous project launch tooling. Charge a fixed provisioning fee plus gas. Best technical stretch: user-approved transaction manifest and destination-signer checks, rather than novel ZK.

**Track fit.** Cross-chain outcome with no transparent ZEC intermediate; also plausible Wallets/UX overlap. Confirm organizer interpretation because service fulfillment is not atomic coin bridging. A linked resource integration is not confirmed sponsor eligibility.

### 2. CommonCart — private collective procurement with one external invoice

**User/problem.** A research collective, open-source team or community wants to buy a shared cloud reservation, data subscription or equipment order. The vendor accepts stablecoins; publicly pooling from individual wallets exposes who belongs to the group, each contribution and potentially negotiating power.

**Product.** The organizer creates a procurement commitment: supplier invoice, minimum target, cap, deadline and allocation/refund policy. Members receive private invitation capabilities and independently pay shielded ZEC to the coordinator. Once fully funded, the coordinator settles **one exact-output stablecoin invoice** from its reserve. If the target fails, members receive shielded refunds. A signed completion receipt can prove to each contributor that the supplier invoice was fulfilled without publishing a contributor roster.

**Why the stack combination matters.** Shielded memos or per-order receiver mappings carry opaque contribution IDs. An encrypted off-chain manifest contains procurement details, rather than publishing member identities in EVM calldata. External-chain stablecoins make the supplier settlement interoperable. NEAR Intents helps the coordinator choose/replenish the stablecoin chain preferred by the supplier, with exact-output semantics and refund buffers. The operator need not route contributors' ZEC through that bridge.

**MVP.** Three contributors, a fixed ZEC contribution schedule per campaign, one supplier invoice in test USDC on Base Sepolia, two campaign outcomes (filled and expired). Use a short fixed quote window or an explicit price buffer: a ZEC-denominated contribution does not guarantee a future dollar amount. Store membership data encrypted; share campaign total only with authorized members. Implement idempotent payment recognition and refunds before adding multiple chains.

**Two-minute demo.** Three unrelated wallets fund the purchase. The vendor sees one stablecoin payment. A participant sees its own contribution/refund record but cannot browse other members' amounts or addresses. Show the expired-campaign refund path alongside the successful campaign.

**Privacy/trust.** The coordinator can identify contributions it receives and links them to the invoice. The supplier sees its invoice and coordinator payment, while public observers see the supplier transfer. It is a custodial conditional purchase; no EVM contract independently verifies hidden Zcash funding. A signed receipt is operator evidence, not a zero-knowledge proof of expenditure.

**Novelty and commercial shape.** The innovation is confidential purchasing power with collective failure semantics, not a donation page. Fixed invoice procurement avoids the additional problems of open-ended pooled investment. Revenue is a campaign coordination fee or supplier commission.

**Track fit.** Cross-chain and Private Markets overlap, with an unusually clear successful demo. No new token, shielded asset issuance, FROST network or generalized escrow is required. Strongest recommendation for a small team.

### 3. Counterflow — reciprocal invoice settlement for trading cooperatives

**User/problem.** A cooperative contains two populations: exporters/freelancers receiving stablecoins who want private ZEC savings; and importers/businesses holding shielded ZEC who owe stablecoin supplier invoices. Individually swapping every invoice leaks patterns, pays repeated bridge costs and creates avoidable rebalancing.

**Product.** A bounded settlement window matches the opposing demands. Participants upload encrypted invoices and reserve quotes. Stablecoin inflows replenish the operator's stablecoin inventory; shielded ZEC inflows replenish its shielded inventory. The operator pays the importers' invoices on the selected external chain and pays exporters' shielded destinations from the ZEC pool. Net the batch, with strict per-order caps and refund deadlines. The market maker earns a spread for advancing inventory and bearing settlement risk.

**Why the stack combination matters.** Native shielded transfers hide the Zcash participant graph. Stablecoin contracts provide exact public invoice receipts on the chain vendors already use. NEAR Intents rebalances **external stablecoin inventories between chains**, or converts permitted non-ZEC reserves; it does not need a transparent ZEC bridge for each customer. If there is insufficient opposite-direction ZEC flow, the service caps new quotes instead of quietly unshielding user value.

**MVP.** One cooperative, one operator, two input currencies, a four-order clearing window, Base Sepolia and a current shielded Zcash test environment. Match exact pre-agreed quotes; show reserve accounting before and after, inventory reservations, partial unmatched batch cancellation, and a refund. Keep the ledger local, encrypted and auditable by the operator. Multi-operator bonds and proof-of-reserves are later work.

**Two-minute demo.** Two exporters need shielded ZEC, while two importers need a stablecoin invoice paid. Complete four real test transactions, display balanced obligations, and show that the ZEC never enters a transparent receiver. Compare the number of required external reserve-conversion operations with four isolated swaps. Savings are demonstrated for that batch, not promised universally.

**Privacy/trust.** The operator knows the bilateral order graph and has temporary custody. Public chain observers see stablecoin inputs/outputs and may correlate timing/amounts. Zcash on-chain observers do not see the shielded internal graph, subject to active protocol and network privacy limitations. Netting is an inventory optimization, not a mathematical anonymity guarantee.

**Novelty and commercial shape.** A purpose-built reciprocal trade desk makes privacy-preserving inventory sustainable, addressing the flaw in demos that receive unlimited ZEC while spending a finite stablecoin wallet. This is more differentiated than “cross-chain payroll.” Start with one cooperative and one trade corridor, not a global exchange.

**Track fit.** Strong cross-chain relevance with a technically meaningful NEAR Intents role. More operationally complex than CommonCart, but the clearest explanation of how a service can preserve a shielded ZEC circuit beyond one demo payment.

## Shared technical scope and acceptance criteria

The minimum architecture is: local/native Zcash wallet and scanner → durable order ledger → bounded operator inventory → external-chain transaction sender → local receipts and refund state machine. Zebra is the consensus/full-node component, not a drop-in shielded wallet RPC; use a compatible wallet library/service for scanning and spending. Expose only an incoming viewing capability to a read-only scanner where possible, and keep spending authority isolated.

Before accepting a demo as real:

1. Decode the actual Zcash transaction and verify there are no transparent customer inputs/outputs for the claimed shielded leg; check active-pool behavior and disclose pool migration if present.
2. Independently confirm recipient wallet receipt and external-chain receipt. A backend “SUCCESS” string or dashboard balance is insufficient.
3. Kill and restart the worker after payment broadcast and show no duplicate transfer. Keep idempotency and durable inventory reservations.
4. Demonstrate expiration/refund and insufficient-reserve refusal. Do not accept orders backed by an imaginary future opposite trade.
5. Document exactly which actor sees which field: public observer, API/router, operator, supplier and recipient. Avoid shared analytics IDs between private and external-chain flows.
6. Show the actual NEAR integration separately: live dry quote or genuine reserve rebalance, with simulation clearly marked where needed. Do not claim NEAR settled the shielded ZEC leg.

### Ranking

| Concept | Differentiation | Hackathon feasibility | Main risk | Recommendation |
|---|---|---|---|---|
| CommonCart | High: private membership + collective success/refund + supplier chain choice | Highest of these three | Custodial coordination and exchange-rate window | Best product MVP |
| QuietLaunch | High: deliver a concrete public-chain service without public treasury top-up | High with one contract template | Public deployment identity and operator linkage | Best developer-facing demo |
| Counterflow | High: operationally sustainable shielded inventory circuit | Medium | Matching liquidity and accounting complexity | Best technical/market mechanism |

An optional supporting tool, **RouteGlass**, could display the actual route's address types, transparent components, provider-visible fields, custody model and refund guarantees before signature. It is valuable differentiation inside any product, but should not replace all three product ideas with infrastructure dashboards.

## Claims to avoid in a pitch

- “NEAR Intents is a confirmed Zecathon sponsor” without an official announcement.
- “Confidential Intents uses Zcash zero-knowledge proofs” or “basic/advanced guarantees operator blindness.”
- “NEAR MPC directly signs shielded Zcash transactions” based only on its secp256k1/Ed25519 support.
- “The swap is atomic across chains” because `token_diff` execution is atomic within NEAR's ledger.
- “All current ZODL swaps are transparent,” given conflicting current wallet source; equally, “1Click is fully shielded” without execution/custody evidence.
- “FROST makes the bridge trustless,” “the EVM contract can read shielded memos,” or “a signed receipt proves a private payment to anyone.”
- “No transparent Zcash intermediate means no metadata leakage anywhere.”

The defensible promise is narrower and still useful: **native ZEC can stay in shielded circulation while a service provides useful outcomes on external chains, with the provider trust and public-chain visibility made explicit.**

## Independent follow-up: deployed protocol, backend compatibility, sponsors

Verified again on October 24, 2026 against release tags, package releases, protocol source and the organizer's public announcement. These findings materially change an implementation plan with four days remaining.

### Protocol and release state

- [ZIP 258 source](https://github.com/zcash/zips/blob/main/zips/zip-0258.md) is **Final** and sets NU6.3/Ironwood activation at mainnet **3,428,143**, testnet **4,134,000**, branch `0x37A5165B`. It prohibits new value entering the Orchard pool and cross-address Orchard transfers after activation. It explicitly introduces Ironwood and version-6-related consensus rules. This is deployed protocol work, not merely an aspirational ZSA proposal.
- An independent read-only [Blockchair stats](https://api.blockchair.com/zcash/stats) response reported mainnet height **3,529,220**, block timestamp **2026-10-24 17:37:56**. That is beyond NU6.3 and below the upcoming NU7 height. Use this third-party height only as corroboration; its other accounting fields are not relied upon here.
- [Zebra 7.0.0](https://github.com/ZcashFoundation/zebra/releases/tag/v7.0.0), released October 22, supports mainnet NU7 at **3,543,405**. [Zebra 7.0.0-rc.0](https://github.com/ZcashFoundation/zebra/releases/tag/v7.0.0-rc.0) set public testnet NU7 at **4,465,026**, expected around October 6. Thus mainnet and current public testnet are at different upgrade stages. An old testnet faucet/wallet/node combination can appear healthy while following a different chain; compare block hashes and consensus branch, not merely the word “testnet.”
- [zcash_protocol 0.11.0](https://crates.io/api/v1/crates/zcash_protocol/0.11.0), published October 21, sets those NU7 activation parameters and makes NU7 support available **without** the old unstable configuration flag. The current [protocol changelog](https://github.com/zcash/librustzcash/blob/main/components/zcash_protocol/CHANGELOG.md) identifies branch `0x77190AD9` and changing block spacing. Do not hardcode 75 seconds per block for public-testnet confirmation estimates.
- The same October 21 release cohort contains [zcash_client_backend 0.25.0](https://crates.io/api/v1/crates/zcash_client_backend/0.25.0), [zcash_client_sqlite 0.23.0](https://crates.io/api/v1/crates/zcash_client_sqlite/0.23.0), and [zcash_keys 0.17.0](https://crates.io/api/v1/crates/zcash_keys/0.17.0); minimum Rust version is 1.89. They are independently released crates, not one version of a `librustzcash` binary. Pin a mutually compatible cohort. Recent APIs changed clock/RNG arguments, address expiry and wallet database interfaces; a 2025 code snippet is not drop-in code.
- [lightwalletd 0.6.0](https://github.com/zcash/lightwalletd/releases/tag/v0.6.0), released October 20, is the current release observed. [0.5.4 release notes](https://github.com/zcash/lightwalletd/releases/tag/v0.5.4) explain a concrete Ironwood discovery bug: `GetLightdInfo.lightwalletProtocolVersion` had been empty even when newer compact block data was supported. Check protocol capability and actual Ironwood scanning; an open gRPC port is insufficient.

### Zallet is useful, but “install latest Zallet + latest Zebra” is unsafe planning

[Zallet v0.1.0-beta.3](https://github.com/zcash/zallet/releases/tag/v0.1.0-beta.3), released August 26, is the newest release observed. Earlier [beta.1 release notes](https://github.com/zcash/zallet/releases/tag/v0.1.0-beta.1) explicitly added Ironwood treestate, scanning, RPC inspection and NU6.3 mainnet support. Therefore it is wrong to dismiss all Zallet releases as lacking Ironwood.

Its convenient JSON-RPC surface includes account creation/address derivation, receipt queries, `z_viewtransaction`, `z_sendmany` and asynchronous operation tracking. The [release-tag first-transaction tutorial](https://github.com/zcash/zallet/blob/v0.1.0-beta.3/book/src/guide/first-wallet.md) supplies the workflow. Default shielded sending refuses privacy-downgrading transparent destinations. Received funds are spendable after **10 confirmations by default**; wallet change after three. A two-minute video must use prefunded wallets and distinguish detection from spendability; show pre-recorded real receipts or explicitly accelerated local consensus where appropriate.

However, there are three decisive compatibility traps:

1. **Database ABI:** Zebra 7 moves the database from **v28 to v29**, and its release notes explicitly require matching Zallet/Zaino direct-state readers. Zallet beta.3's [backend manifest](https://github.com/zcash/zallet/blob/v0.1.0-beta.3/backends/zebra/Cargo.toml) still pins `zebra-state 12`, `zcash_client_backend 0.24.0`, and `zcash_protocol 0.10`, with a fixed older librustzcash Git revision. Do not assume that binary can read Zebra 7 state or follow current NU7 testnet. This is a source-verified version mismatch risk; this research did not run the pair.
2. **Release versus main:** beta.3 ships a `zallet-zaino` JSON-RPC backend, but [commit 9e71ec8, October 7](https://github.com/zcash/zallet/commit/9e71ec8c05afa422966c8146941364c60c3e7d73), removed it from main. The current development guide and the released binary consequently describe different products. Pin the documentation to the exact tag/commit used.
3. **Windows:** current development [platform docs](https://github.com/zcash/zallet/blob/main/book/src/guide/platform-support.md) state that **no chain backend currently runs on native Windows or macOS**. Their core/launcher tests passing does not mean the wallet can sync there. The remaining Zebra backend is Linux-only, requires a co-located state directory, and needs Zebra built with its non-default `indexer` feature. On this Windows workspace, use a Linux host or WSL2/Linux container if choosing Zallet. Its Rust source build requires 1.95; building from crates.io is not the documented working path at present.

For a four-day product, **prefer a small local receiver/sender service around the current, coherent librustzcash wallet crates or an already functioning current wallet SDK sample, consuming a verified lightwalletd-compatible endpoint**. This avoids full-node initial sync and direct-state format coupling. It still requires actual wallet persistence, note scanning, proof generation and key handling; it is not a browser `fetch` integration. If the team already has a working matched Linux Zallet stack, JSON-RPC can be faster. Choose Zallet only after the roundtrip probe passes; do not spend the event debugging a new node/indexer/wallet release matrix. A local regtest fallback is acceptable if labeled accurately, and it must activate the modern pool rules being claimed.

### The decisive first-hour probe

Do this before product UI or cross-chain contracts. It is a go/no-go experiment, not a promise that compilation or confirmations finish in exactly one hour.

1. **Freeze the stack.** Record wallet/SDK versions, Rust/toolchain if needed, server version/protocol, chosen network, current height, active consensus branch and a recent block hash. Cross-check the hash with a second endpoint. Public testnet requires the current NU7-aware cohort; released Zallet beta.3 should not be presumed suitable.
2. **Create two disposable wallets/accounts and a fresh shielded receiver.** Decode the address; confirm the sending wallet chooses the active shielded transfer protocol. Set wallet birthday close to the probe block so a fresh wallet need not scan years of history. Keep the account keys out of product API logs.
3. **Receive one real test payment with an opaque order ID memo.** The receiver must independently decrypt the correct amount and memo and report mined height/confirmations. Compare wallet scan height with network height. A sender screenshot or explorer transaction existence is not a receiver verification.
4. **Refund part of it shielded and restart the service.** After the configured spendability threshold, build/prove/broadcast a return payment, verify its receipt in the other wallet, restart the receiver and show persisted recognition without a second payout. Inspect the actual transaction for transparent components and pool migration.
5. **Set the project scope from the result.** If this fails because of a protocol/backend mismatch, switch immediately to a known working compatible sample or a pinned modern regtest stack. Do not build custody, conditional procurement or invoice settlement on an unverified “paid” webhook. A receive-only prototype can still be honest, but cannot demonstrate automatic refunds it cannot execute.

The critical pass artifact is a small reproducible log/report containing versions, network/branch, receiver-derived transaction data, successful refund receipt and restart behavior, with secrets redacted. It resolves the largest technical uncertainty in all three product proposals.

### Independent organizer/sponsor verification

The organizer's pinned public [September 28 announcement](https://x.com/zksnarks_/status/2104606505177899314) was retrievable through a public text rendering of the original X page. It says “zkSNARKS presents ZECATHON,” calls for useful Zcash technology, specifies a **$100K prize pool**, and says submissions are open for the next 30 days. **That announcement names no sponsor or sponsor bounty.** Only a limited public timeline was accessible, so this is not an exhaustive search of all posts/replies.

Independently fetched official [os.js](https://thezecathon.com/darkpool/os.js) literally renders: **“Announced with the judges”** and **“Backing this pool is not yet public. These slots fill on Oct 29.”** Official [data.js](https://thezecathon.com/darkpool/data.js) has `sponsorSlots: 6`, submission cutoff `2026-10-28T23:59:00Z`, and judge reveal `2026-10-29T16:00:00Z`. Therefore no named sponsor track can responsibly be inferred today from the official material checked. Architect integrations as useful capabilities and keep prize eligibility conditional on later rules.
