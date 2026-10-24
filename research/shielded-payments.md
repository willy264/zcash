# Shielded Payments: mechanisms, existing work, and three product concepts

Checked **24 October 2026**. These are proposed projects, not implemented or audited systems. Sponsor relationships are unconfirmed; integrations below are technical fits.

## What the available technology actually contributes

Zcash hides payment details from public chain observers when a compatible wallet constructs a fully shielded transfer. The recipient still learns the received amount and memo. A payment backend holding an incoming viewing key can recognize payments without having spending authority, but it gains visibility into the corresponding account: a new diversified address does not turn an account viewing key into a narrowly scoped receipt key.

[ZIP 321](https://zips.z.cash/zip-0321) provides payment requests with amounts, addresses and optional memos. It is a request format, not a wallet connection/session protocol, payment confirmation, or recurring debit mandate. Wallet implementations vary; multi-recipient and memo handling must be tested. Its memo parameter uses unpadded base64url, with a maximum decoded memo size of 512 bytes. [ZIP 302](https://zips.z.cash/zip-0302) describes memo interpretation; arbitrary application data is not a smart contract.

The [NU6.3 deployment specification](https://zips.z.cash/zip-0258) introduces Ironwood, prohibits new value entering the Orchard pool after activation, and restricts Orchard cross-address transfers. Use an upgrade-compatible wallet and inspect the actual transaction pool. [ZIP 326](https://zips.z.cash/zip-0326), currently Draft, explains the distinction between an Orchard-protocol receiver and the pools in which it is used. An old tutorial or an address prefix is insufficient evidence of a current compatible transfer.

The [Zallet Book](https://zcash.github.io/zallet/) describes a beta wallet replacing the deprecated embedded zcashd wallet; it explicitly warns of breaking changes and incomplete RPC coverage. A Rust backend can instead use [librustzcash](https://github.com/zcash/librustzcash), with the engineering cost of a real scanner and wallet state. For a short hackathon, reuse an established wallet and keep private spending keys out of the web application. Pin compatible releases and prove the receiving/scanning path before building the UI.

Two additional primitives unlock products beyond ordinary checkout:

- **Blind signatures:** the issuer authorizes a hidden random token without learning the token later presented for redemption. [RFC 9474](https://www.rfc-editor.org/rfc/rfc9474.html) specifies an RSA blind signature protocol. [Cloudflare's TypeScript implementation](https://github.com/cloudflare/blindrsa-ts) implements its variants. This is a known primitive; the innovation must be the complete payment workflow and privacy-preserving deployment.
- **Ephemeral capability keys:** a locally generated signing key can authorize a narrowly defined action—such as a refund of one receipt—without a permanent account or disclosure of the original payer's financial history. Such a key proves possession of that capability, not a person's legal identity or the identity of a shielded sender.

[Privacy Pass architecture, RFC 9576](https://www.rfc-editor.org/rfc/rfc9576.html), is particularly relevant: cryptographic token blindness does not erase IP addresses, timing, cookies, unusual denominations, or issuer key tagging. [RFC 9578](https://www.rfc-editor.org/rfc/rfc9578.html) standardizes token issuance variants. Calling a custom payment-token protocol “Privacy Pass compliant” would require implementing its full relevant formats and behavior, not merely using blind RSA.

## Existing work: where a generic entry would blend in

Repository descriptions establish public prior-art claims, not working implementation, security, or official entry status. Examples inspected:

| Existing project | Publicly described scope | Consequence for our ideas |
|---|---|---|
| [CipherPay Web](https://github.com/atmospherelabs-dev/cipherpay-web) | Hosted checkout, invoices, POS, billing, refund address, subscriptions/x402 | “Stripe for shielded payments” is already crowded. |
| [Grove](https://github.com/davisonio/grove) | Shielded payroll batches prepared for ZODL signing | CSV-to-shielded-payroll is an insufficient differentiator. |
| [ZecPay](https://github.com/Spider333/zecpay) | CSV to ZIP-321 batch requests | Multi-recipient QR generation alone is also crowded. |
| [Velipay](https://github.com/sopskistar/velipay) | Private invoices, payment links and QR codes | A freelancer dashboard needs a new mechanism, not new branding. |
| [Zcash payment service](https://github.com/nerdcash/zcash-payment-service) | UIVK address allocation, scanning, reorg handling, HMAC webhooks | Reuse ideas carefully; a new payment receiver must do more than emit a webhook. |
| [Payment sessions](https://github.com/Iniwura/zcash-payment-sessions) | Persistent, replay-safe ZIP-321 sessions | Session persistence is useful infrastructure, but not by itself an original product. |
| [Durable outbox](https://github.com/devfola/zcash-durable-outbox) | Durable payment IDs and identical-byte retries for Zallet | Reliable refunds should acknowledge existing outbox work. |

The payment-service README contains a QR example whose memo encoding does not match ZIP 321's base64url requirement. This illustrates why examples must be checked against specifications; it is not a claim that the actual implementation necessarily has the same defect.

## P1 — QuietPass: pay once, use a service without a billing identity

**Problem and first customer.** A privacy-focused API, research database, community Wi-Fi provider, or paid search service wants to charge for usage without turning every query into a persistent customer profile. A shielded checkout still fails that goal if it returns a permanent API key tied to the invoice.

**Distinctive mechanism.** Convert one shielded payment into a pack of unlinkable, single-use service credentials. The payment proves value was received; blind issuance prevents the issuer from recognizing which token is later redeemed. Make the privacy outcome visible: the merchant can reconcile money and total token liability, while the usage service cannot join a request to its purchase record through a token identifier.

**Flow.** The browser generates random token secrets and blinds them locally. A payment session commits to the batch of blinded requests with a salted digest. It displays a ZIP-321 request to the merchant's supported shielded receiver, carrying only a random session reference. After the scanner observes sufficient confirmations, the merchant signs exactly the purchased number of blinded requests. The client unblinds and stores the resulting tokens locally. Later it redeems one token per service action; the service verifies the signature and atomically marks its serial as consumed. A duplicate redemption is rejected.

**Architecture.** TypeScript client and token verifier; RFC-9474 library; Rust receiving adapter using a supported wallet/scanner; isolated issuer signing key; SQLite for invoice states, issuance idempotency and spent-token serials. Use common public keys per service class and broad epoch, common pack sizes, and fixed service denomination. Do not let each customer receive a unique signing key. Do not put the purchase ID, Zcash transaction ID, or payer address in the redeemable token.

The blind signer cannot inspect the hidden message. Service class, value and expiry therefore come from an authenticated, allowlisted issuer-key policy and fixed verification rules; a customer cannot request a cheap token that asserts an expensive denomination inside its hidden payload. Persist the exact authorized batch and response before acknowledging issuance, and return that same response on retry. Token issuance cannot be selectively reversed after a payment reorg without creating a linkage mechanism; the merchant must choose a confirmation threshold and explicitly bear residual credit risk.

**Privacy boundary.** The chain observer should not learn the merchant/customer transfer details from a fully shielded payment. The merchant sees its incoming payment and issuance session, but the blind signature protocol does not reveal the final token serial. The service sees redeemed token serials and the service request, which may itself contain identifying information. Shared IPs, cookies, timing, or tiny anonymity sets can reconnect the contexts; isolate issuance and redemption sessions, avoid third-party analytics, and demonstrate an explicitly configured anonymizing transport if claiming network unlinkability. Blindness alone does not establish full user anonymity.

**Four-day MVP.** One local paid document-conversion API, one pack size, three independent clients, an existing wallet for payment, a real confirmed test transaction, tokens issued once, replay protection, and an operator dashboard that shows only the fields each role legitimately needs. Do not build a general wallet, subscription engine, token exchange, or reusable bearer cash system.

**Two-minute demo.** Three clients each buy five credits; one later redeems from a clean session; show issuer records cannot join the redeemed serial to a particular blinded request. Attempt a second redemption and a second issuance for the same invoice; both fail. The display should say “cryptographic issuance/redemption unlinkability; network assumptions shown,” not “untraceable.”

**Integration fit.** ZODL for the payment handoff, librustzcash for recognition, Zebra/Zaino or lightwalletd-compatible infrastructure for chain data. Cloudflare's open-source crypto library is a library choice, not evidence that Cloudflare sponsors the event. A NEAR on-ramp is optional and should not enter the minimum privacy claim.

**Hardest dependency / first-hour test.** Confirm one compatible shielded payment with memo survives wallet handoff and is detected by the chosen backend. Separately run the blind issuance/redeem example. If either is unreliable, shrink to a local/testnet receiver and state that explicitly; do not simulate a real settlement and call it confirmed.

**Why this is more than checkout.** The product solves the payment-to-usage correlation problem after the blockchain transaction has finished. The ongoing service experience gains privacy from the composition of shielding and blind credentials.

## P2 — Aftercare: returns and warranties without creating a customer account

**Problem and first customer.** A customer buys a privacy-sensitive product or service with ZEC but later needs a refund, replacement, repair, or warranty claim. Conventional aftercare asks for an email account, original card, public wallet history, or other permanent identity. With shielded payments, there is no reliable public “sender address” to refund automatically.

**Distinctive mechanism.** A payment-backed receipt grants a narrowly scoped aftercare capability. Prove that the claimant controls the key bound to a paid order; do not pretend to identify the shielded sender. The product preserves a private purchase through the whole returns lifecycle.

**Flow.** The buyer generates an ephemeral receipt key locally. The checkout commits its public key and random order ID in the payment request/memo or a signed order commitment. The merchant's scanner verifies a matching confirmed payment and issues a signed receipt containing the order commitment, refund ceiling, policy version and capability public key. A later claim signs a fresh server nonce, receipt ID, requested action, exact refund amount, and a new supported shielded destination. The merchant validates the receipt and signature, atomically reserves the claim, approves the refund under its policy, and tracks the outgoing transaction until confirmed. Warranty use and monetary refund have explicit state transitions.

**Architecture.** A receipt PWA or local web app, standard signature library/WebCrypto where supported, merchant-owned incoming scanner, merchant signature key, and an isolated refund signer. The online service never needs the customer's seed or viewing key. An idempotent dispatch outbox records the transaction before broadcasting so a timeout cannot trigger a second payment.

Once signed bytes exist, retries rebroadcast those exact bytes. If a wallet construction request has an ambiguous outcome before bytes are obtained, retain the claim reservation and reconcile the wallet operation before attempting a replacement. A database flag alone cannot make an external wallet operation exactly once. Bind the receipt's network, merchant, capability key and policy to the actual detected payment, and apply cumulative limits if partial refunds are added later.

**Privacy boundary.** The merchant knows the item, price, and that the same receipt was presented for aftercare. A delivery address or repair interaction may identify the buyer; this product cannot erase that. Public observers should not learn the payer/merchant/refund relationships through chain details. The receipt intentionally links a refund to a purchase for the merchant and claimant, while avoiding a universal customer profile. The ephemeral key is proof of capability, not cryptographic evidence of the original sender's identity. Someone who steals the private receipt key can claim its rights, subject to policy; recovery and backups must be explained.

**Four-day MVP.** One merchant, one item, full refunds only, a real shielded purchase and refund, locally downloadable receipt, no email signup, replay resistance, merchant approval, and a clear pending/refunding/refunded state machine. No automatic arbitrary dispute resolution or transferable consumer credit market.

The local export must securely preserve the private capability key as well as the signed receipt; a receipt file without that key cannot authorize a later claim. Domain-separate claim signatures and bind merchant, network, receipt, action, integer amount, destination and expiring nonce. Limit authority to the actual paid amount minus completed/reserved refunds.

**Two-minute demo.** Buy an item, close checkout, reopen only the local receipt, submit a signed claim to a fresh shielded destination, obtain the refund, then attempt a duplicate and a modified-destination claim. Show that the merchant sees one order and claim, while no permanent customer account or wallet history was required.

**Integration fit.** ZIP 321 and supported ZODL handoff; librustzcash or a pinned Zallet beta adapter for receipt detection and payout; Zebra-backed confirmation checking; existing durable-outbox patterns. FROST merchant approval could be a later extension, but it is unnecessary for the MVP and would not make the refund policy an autonomous Zcash contract.

**Hardest dependency / first-hour test.** Confirm receipt memo handling and one outgoing shielded refund, including recording an outgoing transaction before retry. A memo containing an arbitrary refund address alone is not adequate authorization.

**Why this is more than private invoicing.** The differentiation is post-purchase rights, accountless proof of entitlement, and safe private refund handling. It extends usefulness beyond the initial sale.

## P3 — CoverDrop: eligibility-checked aid without a unified identity-to-payment registry

**Problem and first customer.** A university hardship fund, community relief program, or contributor-support fund must verify eligibility while avoiding a durable database that combines a person's sensitive circumstances, withdrawal address, and payment history. Ordinary shielded payroll still leaves this combined registry at the service operator.

**Distinctive mechanism.** Separate the organization that knows eligibility from the service that pays. A blind credential proves entitlement to a standard grant without revealing the identity used to obtain it. Zcash then keeps the payout from publishing the recipient graph on-chain.

**Flow.** The eligibility issuer verifies a person under its existing process and permits one blind issuance for a program period. The client creates and blinds a random entitlement token. The issuer signs it with a program-wide, fixed-denomination key. Later, from a separate session, the recipient presents the unblinded token and a fresh supported shielded address to a funded payout service. The service verifies the credential, reserves its serial atomically, and pays the standard grant. The issuer records that someone received an entitlement; the payout service records a token and payout but has no identity lookup table. Neither service should export combined records by design.

**Architecture.** Separate issuer and payout services, common credential keys/epochs, local client secret storage, standard blind-signature library, duplicate-issuance control at the issuer, single-use serial database at payout, and a supported shielded wallet. Keep grant denominations and release windows broad enough to avoid making each recipient unique. A published issuer key directory helps clients reject per-person keys used as tags.

The payout amount and program period are determined by the approved issuer key and verifier policy, not by uninspected client-controlled fields in the blind-signed message. Authenticate and pin common issuer keys; a directory that gives each person a different key can still tag users. Enforce a funded program cap because the issuer is trusted to authorize grants. Credential issuance retries return the same persisted response. Token redemption uses a reserved/pending/paid state machine bound to the original destination and amount: an uncertain wallet operation retains its reservation until reconciled, and an existing signed transaction is rebroadcast unchanged. Otherwise a crash can either double-pay or permanently consume an unpaid entitlement.

**Privacy boundary.** The issuer knows identities and eligibility facts; the payout service knows redeemed serials and receiving addresses/amounts. Token blindness prevents a direct cryptographic join even if records are later compared, but synchronized timing, IP addresses, unique grant amounts and tiny cohorts can still reveal the mapping. Transport separation, delayed batch redemption and minimum cohorts are deployment requirements for a stronger claim. The payout service is a trusted funded custodian until payment completes. Credentials are bearer rights and may be transferred; this is not a trustless proof of unique humanity or a guarantee against colluding applicants.

**Four-day MVP.** A synthetic roster of ten eligible people, one standard grant, separate issuer/payout interfaces, at least three blind issuances, one real shielded payout, duplicate claim rejection, and retry-safe dispatch. Use synthetic identities in the demo. Keep real-world eligibility verification external; the demo does not need an identity-document upload service.

**Two-minute demo.** Issue entitlements to three eligible participants, redeem one in a separate session, show that issuer and payer tables have no shared token identifier, then reject a repeat issuance and repeat payout. Explain what a network observer could still correlate.

This table comparison illustrates data separation, not a proof of unlinkability. The cryptographic claim comes from the selected standard protocol under its assumptions. The payout service still maintains token-to-address/payment records; the avoided record is a unified identity-to-payment registry.

**Integration fit.** Zcash wallet libraries plus a compatible existing signer; optional FROST for fund governance later; optional NEAR conversion of treasury reserves separately, without attaching a beneficiary's identity to an on-ramp or claiming the conversion itself is fully shielded. No native Zcash token issuance or custom consensus circuit is needed.

**Hardest dependency / first-hour test.** Exercise issuance, blinding, fixed key validation and single-use redemption between genuinely separate processes. Then prove one shielded payout with a durable transaction record. If metadata cannot be separated in the demo deployment, scope the claim to protocol unlinkability rather than full identity protection.

**Why this differs from QuietPass.** QuietPass decouples a payment from future usage; CoverDrop decouples an eligibility decision from money received. The common cryptographic primitive serves a different trust boundary, customer, abuse model, and cash flow.

## Selection within this track

For the remaining sprint, **Aftercare** offers the clearest everyday product and a compact end-to-end demo. **QuietPass** has the strongest compositional privacy story if the team can implement the token flow correctly. **CoverDrop** has substantial social usefulness but requires the most care around eligibility, metadata, and custodian assumptions. None requires inventing a new cryptographic proof system or relying on unreleased native stablecoins.
