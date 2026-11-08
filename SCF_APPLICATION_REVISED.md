# Revised SCF application answers - USD 50,000

Updated 9 November 2026. Founder-selected request: USD 50,000. Four-person team reported; CA identity/profile and Tael post URL will be supplied later. Fill those fields and review this AI-assisted draft before submitting. This is a revised proposal for the same CoverFi project. Nothing here claims an existing mainnet deployment.

## Project title

CoverFi: one-market stablecoin protection and reserve engine

## Products & services

CoverFi is building one capped, short-duration USDC protection market on Stellar/Soroban. A wallet-signed protection engine quotes the premium, segregates principal in a custody vault and locks the maximum claim capacity in a reserve vault before accepting a position. A market-bound oracle adapter supplies an eligible observation at or before expiry; settlement releases unused capacity or reserves the capped payout. Missing expiry data leaves the position awaiting an oracle observation rather than inventing a later price.

This request funds only that primitive and the app, price reliability, reserve diagnostics, monitoring and one technical partner integration needed to use it. Username/receipt registries, payment locks, floor shields, ZK proof flows, invoice APIs and AI assistants are excluded. The four custom contracts are protection_engine, reserve_vault, protected_balance_vault and oracle_adapter; legacy crates are not funded products.

The current implementation uses distinct seven-decimal protected and payout tokens. Prices mean payout-token units per protected token, scaled by 100,000,000. Payout equals protected amount times the positive entry-to-expiry price difference, divided by that scale, capped by the position's reserved maximum. It is expiry-based protection, not compensation for every temporary depeg or guaranteed dollar recovery.

Current status is testnet only, with explicitly mock USDC and CFUSD assets. A future capped mainnet launch is planned after independent security review, verified production assets/price units, governance, reserve capacity, operational readiness and applicable legal requirements. No mainnet contracts, paid volume, independently validated pricing or licensed insurance status are claimed.

## Market need and differentiation

The initial hypothesis is that Stellar wallet users/applications want short-duration capped stablecoin protection without building reserve/claim infrastructure themselves. Current demand is not yet independently validated. We will test premium acceptance, principal-lock tolerance and completion/abandonment through the proposed pilot. Nexus Mutual's documentation describes membership and evidence-based claim assessment; InsurAce's depeg documentation describes investigation and assessment/voting. CoverFi's focused design instead calculates the capped expiry-price result on Soroban and locks capacity upfront. This moves trust to contract/oracle operation and does not imply broader coverage, better security or lower prices. Selling/reducing exposure is also an alternative. See MARKET_POSITIONING.md for primary sources and remaining validation.

## Requested budget

USD 50,000, reduced from the original USD 94,000 request. Future-work allocations: core engineering/invariant remediation USD 16,000; wallet app/backend integration USD 12,000; oracle reliability, monitoring and partner integration USD 12,000; CA-led testable reserve accounting/risk diagnostics USD 6,000; documentation/evidence USD 2,000; direct test/CI infrastructure USD 2,000.

Planning labor assumptions are 400 hours at USD 40/hour, two engineering allocations of 400 hours at USD 30/hour, 240 CA-led hours at USD 25/hour, and 100 additional documentation hours at USD 20/hour. Individual assignment/availability must be confirmed. No audit-provider fees, legal costs, marketing, giveaways, general overhead or reserve capital are included. Existing code and deployment are baseline evidence; this ask does not reimburse them.

Payments: USD 5,000 at award acceptance; USD 10,000 after MVP completion; USD 15,000 after testnet completion; USD 20,000 after launch completion. Phase 1 work costs USD 15,000 including the advance; Phase 2 costs USD 15,000; Phase 3 costs USD 20,000. See BUDGET_50000.md for the reconciled matrix.

## Traction evidence

The founder reports that current app use is limited to the team; no external active-user count or customer revenue is claimed. Rise In publicly announced CoverFi as the Delhi Build Station winner: https://www.linkedin.com/posts/risein_delhi-build-station-is-a-wrap-21-days-activity-7492576361832816640-ZZEu. The founder reports more than USD 1,000 support; the exact amount/payment evidence is not independently verified here. The founder also reports that Tael reposted CoverFi's X announcement; [INSERT POST/REPOST URL] is pending. This is an acknowledgement signal, not a claimed live integration. Fianza is not claimed as a confirmed partner.

Verified technical baseline: 52 contract unit tests; eight deployment-boundary tests; four deterministic reserve-model tests; four source-matched custom contracts on public Testnet; two team-controlled on-chain smoke positions and 34 confirmed successful deployment/smoke transactions. The simulated depeg paid 6 CFUSD and returned principal; the no-loss position also returned principal. Post-smoke reserve assets matched the token balance at 1,004.96 mock CFUSD with no remaining locked liabilities or claims at the recorded checkpoint. Team-controlled wallets and mock reserve tokens do not count as external demand or real LP capital.

Source commit deployed: 6e85036b236df94c550434efdbf7e9287d417921. Testnet IDs:

- Protection engine: CDYOKZH532VB2RHVEFPBEDK3IAZNBXV6JAPHMGHI7GXKCFMECKJIREXB
- Reserve vault: CD76MVLI3U2WP4W7WAABLXFI65P2EZXPQ7II42PG2WYIGRCL236XAVFM
- Principal vault: CBLNUMR2PE7B4EGKJ3ZH6SU2BL3EWEVEJOAVCSC4Y6MZHG7N7JMSEK3E
- Oracle adapter: CDMP7NOSFTN77TSRN5GVXML7KQVF4P6O77C5SWF6OACHYXOZVXMHM2BA

Example paid claim transaction on Testnet: 5c2c28604fcdeba081ae96b440478f88e3c64e10e8e8b5d6ed92e2614d644a7a. It transferred worthless test CFUSD, not real capital. Full hashes/receipts and the historical archive are at https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json. The existing app integration is being updated by the founder and is not yet attested as complete.

## Tranche 1 - connected MVP

Weeks 1-3 from award start. Payment on completion USD 10,000; Phase 1 cost USD 15,000 including the USD 5,000 advance.

1. Integrated wallet app/backend for the four-contract ABI, including quote, create, expiry settlement, claim and principal return. Cost USD 12,000. Evidence: public build, release commit and a recorded wallet-signed walkthrough with matching transaction links. Existing isolated contract deployment is not billed as new work.
2. Basic market health and CA-reviewed accounting acceptance specifications implemented as testable diagnostics. Cost USD 2,000. Evidence: market IDs/units, fee/cap examples, reconciliation results and assigned review ownership.
3. Reproducible user/reviewer instructions and directly consumed test infrastructure. Cost USD 1,000. Evidence: instructions, cost record and runnable release checks.

## Tranche 2 - usable testnet pilot and observability

Weeks 4-7. Payment and Phase 2 cost USD 15,000.

1. Extend invariant, withdrawal/claim and oracle-fault coverage and remediate identified issues. Cost USD 5,000. Evidence: current CI, documented failure scenarios and resolution records.
2. Instrumented external pilot plus the core partner adapter, market feed and monitoring implementation. Cost USD 7,000. Target ten independent testers and twenty completed testnet positions; publish actual results, feedback and failures. Evidence includes a threat model, monitoring plan, tested alert/response path and consented transaction records. These targets are future outcomes, not existing traction or universal SCF entry thresholds.
3. Implement CA-led reserve stress/reconciliation diagnostics and a capital-provider demand report. Cost USD 2,000. Evidence: assumptions and dated human review; at least three provider interviews; consented conditional letters/amounts where obtained. No USD 10,000 mandatory threshold, pre-funding deposit or signed LP letter is claimed today. Report zero commitments if that is the result.
4. Pilot evidence and directly consumed infrastructure. Cost USD 1,000. No acquisition incentives or participant prizes.

## Tranche 3 - gated capped mainnet launch

Engineering target weeks 8-12; payment and Phase 3 cost USD 20,000. External review scheduling is a dependency, not a guaranteed date. Notify SCF and agree any necessary schedule/acceptance change before a tranche deadline; do not bypass launch gates.

1. Complete production asset/payout-pair/oracle configuration, governance controls, security remediation and operational launch checks. Cost USD 7,000. Evidence: reviewed release, issuer/price-unit verification, role ownership and resolved blocker log. Independent audit-provider fees are outside this budget; seek Audit Bank support if eligible after funding.
2. Release the connected app and one partner-owned core integration, then perform a capped mainnet lifecycle only after all launch gates and actual reserve funding are met. Cost USD 8,000. Evidence: partner confirmation, live integration URL and verifiable contract/configuration/transaction records. Tael is a prospective integration contact, not a promised completed integration.
3. Publish reserve reconciliation, exposure limits, calibrated-pricing limitations and financial operating dashboards. Cost USD 3,000. CA review must be recorded; no guaranteed returns or actuarial certification is implied.
4. Release documentation, handover, verification and directly consumed infrastructure. Cost USD 2,000. Publish launch metrics and incident drill evidence. If security, legal, oracle or capital gates are unmet, report the blocked launch and obtain SCF guidance rather than substituting a mainnet claim.

## Team

The founder reports a four-person team: Pallavi Jain, Garvit Arora, Piyush and [FOURTH MEMBER NAME - PENDING]. The existing team covers Soroban development, full-stack engineering, AI-assisted implementation and cloud infrastructure. The fourth member is reported to be a Chartered Accountant with crypto tokenomics experience; [PUBLIC PROFILE / CREDENTIAL BACKGROUND / RELEVANT EXAMPLES / WEEKLY AVAILABILITY] will be supplied before submission.

His proposed role is Finance and Risk Modeling Lead, owning reserve reconciliation, fee distribution, correlated-depeg scenarios, liquidity/capital-limit analysis and protocol unit economics. He strengthens the financial domain work. We do not claim actuarial credentials or prior insurance/protocol underwriting experience without evidence. The deterministic model is a starting point for his review, not an already validated risk price.

Garvit Arora: https://www.linkedin.com/in/garvitt-/
Pallavi Jain: https://www.linkedin.com/in/pallavii-/
Piyush: https://www.linkedin.com/in/piyushvats/

## AI disclosure

The founder states that all code was generated with Cursor, Claude, GPT and other AI assistance, and that humans manually reviewed each part of the contracts and backend and ran multiple tests. Codex also assisted recent code, testing, deployment and this application. Internal review/testing is not an independent audit. All revised submission wording requires final team review; exact historical AI-drafted application sections remain to be identified. The team owns the final implementation and claims.

## Business plan and sustainability

Buyers pay a disclosed premium for capped expiry-based protection; LPs provide claim capacity and bear losses. The source defaults allocate 78 percent of risk premium to underwriting, 12 percent to protocol treasury, 7 percent to safety and the remainder plus a fixed charge to automation. Quote components vary with duration and risk settings; prototype rates are not actuarially validated. Capital deposits are not protocol revenue and no returns are guaranteed.

At an illustrative USD 100,000 monthly protected notional, a 1 percent premium and 12 percent protocol share yield USD 120 gross protocol fees before costs/rewards. This is a sensitivity assumption, not actual volume or a sales forecast. We will measure willingness to pay, partner demand, LP appetite and actual operating cost before expanding. Build funding does not seed the claims reserve. A separately funded, risk-accepted reserve and safe operating setup are gates for real-value sales. See BUSINESS_PLAN.md for the model and downside limitations.

## Architecture and evidence links

Architecture: https://github.com/CoverFI-space/coverfi-contracts/blob/main/FUNDING_SCOPE.md
Registry: https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json
Deployment evidence: https://github.com/CoverFI-space/coverfi-contracts/blob/main/TESTNET_DEPLOYMENT.md
Current website reported by team: https://coverfi.space
Docs source: https://github.com/CoverFI-space/coverfi-docs

## Resubmission changes

Reduced scope to one market; reduced request from USD 94,000 to USD 50,000; added a reported CA/finance role; published source-matched testnet deployment and receipts; corrected deployment status; disclosed full AI code generation and human review; added reserve economics, monitoring, pilot and security-readiness work. Current usage remains internal, no external LP commitment is fabricated, and the future mainnet launch is not represented as already deployed.
