# ZECATHON: verified brief and unresolved details

Research checked on **24 October 2026**. This concerns the Zcash event at [thezecathon.com](https://thezecathon.com/), presented by zkSNARKs. It is distinct from Zenith Bank's similarly named Zecathon.

## Source method

The landing page was read in the browser. Its public JavaScript content and rendering/form files were read directly to recover the track wording and requirements without creating an account. No registration or submission was made. Public source is useful evidence of what the site currently publishes, but comments, unused screens, and old form branches are not all equally authoritative.

- [Landing page](https://thezecathon.com/): event identity, $100,000 headline, five tracks, countdown, guide.
- [Public content configuration](https://thezecathon.com/darkpool/data.js): dates, exact track descriptions, judging criteria, rules, resource links, prize amounts, unnamed sponsor slots.
- [Screen renderer](https://thezecathon.com/darkpool/os.js): reads the configuration to display the tracks; sponsor screen wording.
- [Submission forms](https://thezecathon.com/darkpool/forms.js): current account-backed submission requirements; also contains older fallback/team UI code.

## Five tracks

| Track | Exact published description | Product implication |
|---|---|---|
| Shielded Payments | “Point of sale, payroll, remittance, subscriptions. Money that moves without publishing who paid whom.” | Deliver a real payment workflow while preserving the relationship between payers and recipients from public observers. |
| Private Markets | “The dark-pool thesis. Sealed-bid auctions, order flow, settlement, anything where the price should be public and the participants should not.” | Explain exactly which market information becomes public and which actors can still see bids, identity, or settlement. |
| Cross-chain | “Swaps, bridges and intents that reach other chains without unshielding on the way through.” | A route that unshields ZEC and then reshields later is a poor fit. An inventory-funded service can keep the native ZEC leg shielded, but its trusted operator must be disclosed. |
| Core & Tooling | “Zebra, librustzcash, indexers, wallet SDKs, provers. The unglamorous work everything else is built on.” | A reusable tool with measurable improvement can compete strongly without an elaborate consumer application. |
| Wildcard | “Art, games, social, hardware. Anything that makes a shielded transaction worth making.” | The shielded transaction should change what the experience can do, rather than merely serve as a checkout button. |

## Judging criteria

The configuration publishes these four criteria without numerical weights:

1. **Privacy:** “Does it actually preserve privacy, or does it only look like it does? Leaks are disqualifying, not deductions.”
2. **Usefulness:** “Would someone use this on Monday.”
3. **Execution:** “Does it run. Working beats ambitious.”
4. **Originality:** “Built for this hackathon, not repackaged.”

Interpretation for project selection: make one narrow, real workflow work; demonstrate both its normal path and an adversarial/failure path; describe privacy by observer, including the service operator. Numerical scores elsewhere in this research are our own selection rubric, not official judging weights.

## Schedule

| Milestone | Published time |
|---|---|
| Registration and submissions open | 28 September 2026, 00:00 UTC |
| Submission deadline | **28 October 2026, 23:59 UTC** |
| Deadline in Lagos | **29 October 2026, 00:59 WAT** |
| Judges revealed / judging begins | 29 October 2026, 16:00 UTC |
| Judging period | 29 October–22 November 2026 |
| Winners announced | 23 November 2026, 16:00 UTC |

The site describes a thirty-day build window. On the research date only approximately four days remain. The idea portfolio distinguishes a narrow sprint MVP from a broader product; the research does not assume there are still thirty days to implement.

## Eligibility and submission

Published rules say:

- One person registers and submits; the prize goes to that account; build with anyone.
- All code must be written during the build window. Libraries are allowed; existing products are not.
- Open source at submission; a private repository made public by the deadline counts.
- One submission per account, entered in **exactly one track**.
- Submissions remain sealed until judging; editing is permitted before the deadline.
- Organizers and judges cannot enter.

The current account-backed form requires a project name (maximum 64 characters), one track, a description (maximum 2,000 characters), repository URL, **working demo URL and two-minute video URL**, and confirmation that the repository will be public. Its description prompt is unusually useful: “What it does, why it does not leak, what is real and what is a mock. Judges read this first.”

The guide also names both demo and video. An older fallback checklist says demo **or** video, so prepare **both**. Source also contains a team workflow with up to five members, while the current guide/rules describe one submitting account. Do not treat a dormant team branch as a definitive team-size rule; follow the live account form and confirm team administration if necessary. Saving a draft is distinct from pressing SUBMIT; the current form says changes after submission need to be submitted again.

Prizes are described as paid in ZEC to a shielded address, with winners contacted on Telegram. This is published information, not independent confirmation of escrow or funding.

## Prize and sponsor evidence

The site advertises **$100,000 total**. Its content lists $15,000 for each of five tracks, a $20,000 grand prize awarded on top of a track prize, and $5,000 in community bounties. Those numbers add to $100,000. However, the source comment above the split explicitly says **“PLACEHOLDER SPLIT. The total is the number you gave me; the division is my proposal.”** Therefore report the split as currently listed, with provisional status unresolved. Do not base a project's expected value on the exact allocation.

There are **no named sponsor challenges in the public content read**. The configuration has six unnamed sponsor slots. The screen renderer says “Announced with the judges” and “Backing this pool is not yet public. These slots fill on Oct 29.” That date is after the submission deadline. This may be an unpublished/unused screen, but it supplies no evidence for named sponsor tracks either way.

The site's resource list includes Zcash developer docs, ZIPs, librustzcash, Zebra, the ZODL GitHub organization, and NEAR Intents. **Being listed as a resource does not establish sponsorship.** The main report therefore supplies an ecosystem integration matrix and conditional sponsor fits, with no claimed sponsor eligibility or guaranteed prize stacking.

## Details to resolve before submitting

These are clarification items, not reasons to stop building or researching:

1. Whether a separate sponsor/bounty brief exists, with explicit eligibility, evidence requirements, and stacking rules.
2. Whether the listed prize split is final.
3. Whether the Cross-chain track accepts shielded inventory settlement with a disclosed trusted operator, and what privacy observers the organizers intend to exclude.
4. Whether a testnet or reproducible local demo is acceptable; the public form requires a working demo but does not specify mainnet.
5. If a team is involved, which current account/team mechanism should control the final submission.

No organizer was contacted as part of this research.
