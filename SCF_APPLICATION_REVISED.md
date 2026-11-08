# Revised SCF application answers

AI-assisted draft dated 8 November 2026. Review before submission. These answers cover the fields visible in the supplied old application; no additional form fields or current program rules were supplied. Budget and relative timeline are proposed, not an approved award schedule.

## Project title

CoverFi: one-market stablecoin protection and reserve pilot

## Products & services

CoverFi is narrowing its grant request to one USDC testnet protection market on Stellar/Soroban. The core deliverable is a protection engine and reserve vault: users sign a capped, short-duration protection position, maximum claim capacity is locked before acceptance, and an eligible expiry observation determines settlement. A minimal custody vault and price-feed support are included only where required for this lifecycle.

This grant does not fund username registration, receipts, payment locks, floor shields, ZK verification, protected invoice APIs or an AI assistant. These are legacy or separate experiments. One partner will integrate the core quote/open/settle experience rather than a new invoice platform.

The source now defines prices and claims in a distinct seven-decimal test payout token, binds the oracle pair and removes the proof dependency. The pilot will validate that specification on-chain, reserve reconciliation, stale-oracle behavior and the complete purchase-to-claim lifecycle. It is testnet-only. Mainnet launch is outside this request.

## Requested budget

USD 40,000, reduced from USD 94,000. Proposed allocation:

| Work | USD |
| --- | ---: |
| Core engine/reserve scope, denomination specification and invariant work | 14,000 |
| Testnet release, verification and minimal user flow | 7,000 |
| Third-party pilot and one partner integration | 6,000 |
| Independent scoped technical review and remediation | 6,000 |
| Monitoring, evidence reporting and documentation | 4,000 |
| Pilot operations and infrastructure | 3,000 |
| Total | 40,000 |

These are planning allocations, not supplier quotes. Independent review is scoped technical review, not a promised comprehensive audit. No reserve seeding is included in the grant budget. Confirm current SCF eligibility, tranche structure and dates before submission.

## Traction evidence

The organization publishes seven repositories at https://github.com/CoverFI-space. The public contracts workspace currently contains seven crates; it is not the ten-contract local workspace described by older architecture materials.

The team reports receiving more than USD 1,000 from Stellar Build Delhi, formal founder communications with Tael and Fianza/Finza, and product trials by those contacts. Award/payment evidence, exact amount, partner name confirmation and public testing evidence remain to be attached. We do not claim signed LP commitments, a live partner integration, verified third-party protection activity, paid volume or mainnet deployment.

Fresh source-matched testnet IDs, mock-asset disclosures, smoke evidence and a separate historical archive are published at https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json. Historical smoke tests and old test counts do not establish current deployment health or correspondence to current source. The source now passes 52 workspace unit tests and 8 deployment-boundary tests locally; CI records the four-contract build. The fresh deployment now publishes source commit, WASM hashes, initialized settings, test reserve funding and successful controlled depeg/no-loss lifecycles in TESTNET_DEPLOYMENT.md. These team-controlled simulations do not establish external users, real capital or partner integration. Public app activation remains outstanding.

## Tranche 1 - focused specification and reproducible release

Weeks 1-3 from project start; USD 12,000.

Outcome: a reviewer can reproduce a minimal one-market build and explain exactly when it pays. Publish the selected asset/issuer, payout denomination and conversion semantics, expiry-based trigger, payout cap, permitted durations, premium breakdown and oracle/backfill policy. Revalidate the now proof-free four-contract release, confirm client compatibility and keep unrelated product surfaces outside the release manifest.

Acceptance evidence: tagged source commit, reproducible build instructions, current CI output, payout examples, reserve/claim invariants, stale/missing-price tests and a reconciled testnet lifecycle with transaction hashes. Publish no-payout, positive-payout, cap, duplicate-claim and withdrawal-safety cases. Build success alone is insufficient.

## Tranche 2 - external users and reserve demand

Weeks 4-7; USD 16,000.

Outcome targets: at least 10 third-party testers open and settle at least 20 testnet positions in aggregate; exercise positive-payout, no-payout and missing-oracle cases; record successful claim and principal withdrawal paths. Controlled depeg scenarios must be visibly labeled simulations, not organic market activity. Testnet funds have no monetary value.

Reserve-demand target: publish at least two named external LP commitment letters totaling at least USD 10,000 of intended real reserve capital, including per-LP amount, asset, conditions, confirmation and permission to publish. This is a proposed target; no commitments are claimed today. Real funds need not be deposited into unaudited contracts to demonstrate interest. Testnet reserve seed is reported separately.

Acceptance evidence: consenting tester records and feedback, independent wallet activity with transaction hashes, position/claim reconciliation, letters and named amounts, monitoring and a public threat model. Unmet targets are reported as unmet; internal wallets do not count as third-party demand.

## Tranche 3 - partner integration and pilot decision

Weeks 8-12; USD 12,000.

Outcome target: one third-party partner runs a live testnet integration of quote, position creation and settlement, with a partner-owned URL/repository and written confirmation. Tael and Fianza/Finza are discussions/testing contacts, not announced integration commitments.

Acceptance evidence: partner confirmation and testnet transactions; independent scoped technical review and remediation status; pilot completion/claim metrics; reserve and oracle incident drill; final build/registry verification; and a published go/no-go report with unit economics and operating runway assumptions. Mainnet release and real-value protection sales are not acceptance criteria.

If external LP or partner targets are not achieved, report the shortfall to reviewers. Do not substitute another internal feature or claim the tranche outcome is complete.

## Team

CoverFi is built by Pallavi Jain, Garvit Arora and Piyush. The team's reported experience covers Soroban development, full-stack engineering, AI tooling and cloud infrastructure. The team also reports earlier DebtAI work and more than 50 users; that historical metric is not CoverFi traction and requires its own evidence if used.

The team does not currently claim insurance, actuarial or protocol underwriting expertise, and no additional specialist member is being added in this revision. Prototype pricing is unvalidated. A qualified external pricing/reserving review remains a gate before any real-value protection launch; engineering tests cannot replace it. This experience concern remains unresolved for the current team.

Garvit Arora: https://www.linkedin.com/in/garvitt-/
Pallavi Jain: https://www.linkedin.com/in/pallavii-/
Piyush: https://www.linkedin.com/in/piyushvats/

## AI disclosure

The team used Cursor, Claude and GPT for development and reports reviewing the code line by line. Codex assisted this revised application, budget, scope, milestones, business plan and documentation. Historical AI-drafted sections of the old application are not yet identified. The team must review this new revision before submission. AI assistance and internal human review are not independent security or actuarial review.

## Business plan and sustainability

The intended customer is a Stellar user or wallet seeking transparent, capped, expiry-based stablecoin price protection. LPs supply risk capital and earn the underwriting portion of matured premiums, with losses borne according to reserve accounting. LP deposits are not revenue and returns are not guaranteed.

The source defaults allocate 12% of risk premium to protocol fees; the remainder funds underwriting, safety and automation. Duration rates and adjustments are prototype settings requiring calibration and customer testing. No paid mainnet pricing or recurring revenue is claimed. Publish the full quote rather than advertising a base rate as the final price.

At an illustrative USD 100,000 monthly protected volume and 1% risk premium, a 12% protocol share yields USD 120 gross monthly fees before partner rewards and costs. This cannot support the team alone. The pilot will measure willingness to pay, LP appetite and service demand before making a scaling decision. Track operating runway separately from restricted claim capital. The detailed model and downside scenarios are in BUSINESS_PLAN.md.

## Architecture link

https://github.com/CoverFI-space/coverfi-contracts/blob/main/FUNDING_SCOPE.md

## Deployment registry link

https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json

## Website and evidence links

Organization: https://github.com/CoverFI-space
App/website previously reported by the team: https://coverfi.space
Documentation source: https://github.com/CoverFI-space/coverfi-docs
Live site uptime and current app deployment are not attested by this revision. Use repository evidence until fresh verification is attached.
