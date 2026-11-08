# CoverFi evidence and release status

Reviewed 9 November 2026. This records source review and a timestamped, controlled testnet verification. It is not an independent security audit or third-party product validation.

| Item | Evidence and limitation |
| --- | --- |
| Public source | Seven crates in the workspace; only four pilot contracts are default members and selected by the pilot runner. |
| Local-only source | A separate local workspace contains quorum_oracle_source, payment_lock_engine and floor_shield_engine. Their implementation is not evidence in this public release. |
| Mainnet | No verified deployment supplied. Mainnet is recorded as not deployed; unsigned deployment artifacts are not a deployment. |
| Testnet | A fresh four-contract deployment is hash-matched and smoke-verified at ledger 5627678; see TESTNET_DEPLOYMENT.md. Older IDs are archived separately as unverified. |
| Reserve | The fresh pilot was seeded with 1,000 CFUSD test reserve and 10 CFUSD automation funds; post-smoke balance was reconciled on-chain. These are worthless mock tokens, not external capital. No externally verified LP commitment is published. |
| Users and partners | Founder confirms current usage is internal. Tael is the only claimed acknowledgement contact; a founder-reported X repost URL is pending. No Fianza partnership, external pilot completion or live partner integration is claimed. |
| Funding support | Rise In publicly announced CoverFi the Delhi Build Station winner: https://www.linkedin.com/posts/risein_delhi-build-station-is-a-wrap-21-days-activity-7492576361832816640-ZZEu. The founder reports more than USD 1,000 support; exact amount/payment evidence remains unverified. Not LP capital. |
| Team | Founder reports four members: Pallavi Jain, Garvit Arora, Piyush and a CA with crypto tokenomics experience. CA name/profile, relevant cases and availability are pending. No actuarial/underwriting credentials are invented. |
| AI | Cursor, Claude and GPT were used; see AI_DISCLOSURE.md. |
| Audit | No independent audit report was supplied. Internal tests/reviews are not an independent audit. |

## Public-source versus local-source drift

The current public protection_engine uses the market-bound oracle_adapter and has no zk_verifier dependency. Its initialization ABI differs from the prior public release. The separate local engine uses admin-submitted price observations. Historical deployed IDs cannot be assumed to implement either revision without WASM hash and configuration checks.

The new funding scope is one USDC testnet market. The controlled mock-asset deployment and smoke lifecycle are now verified; this is not external product validation. The default four-contract source release now removes the proof dependency, enforces distinct seven-decimal tokens and specifies payout-token denomination. A fresh on-chain deployment is complete; the public app/backend client integration remains outstanding. Do not describe the seven-crate legacy workspace as a finished minimal depeg engine.

## Reviewer evidence checklist

- Release commit, WASM SHA-256 hashes, network, token contract and issuer, engine/vault configuration, privileged roles and deployment transaction hashes.
- Reserve balances, locked liabilities, reserved claims and reconciliation at a recorded ledger/time.
- Position creation, expiry settlement, claim and principal withdrawal transactions from third-party testers, with consent to identify participants.
- Named LP commitment letters with amount, denomination, conditions, capital-at-risk acknowledgement and permission to publish. Testnet tokens and awards must be separate from real committed capital.
- Partner-owned integration URL, partner confirmation and successful end-to-end transaction.
- Current CI run and independent review scope/report. Old test totals are historical claims until reproduced.

Canonical registry: https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json

## Code verification update

The current code passed 52 workspace unit tests (34 pilot tests) and 8 Python deployment-boundary tests locally on 8 November 2026. The earlier Cargo-cache permission failure was resolved. See TESTNET_RELEASE.md and the release commit CI for build evidence. Native tests alone do not attest live state. Separate on-chain evidence now records source-matched contract hashes, funding, two lifecycles and reconciled balances in TESTNET_DEPLOYMENT.md.

## Resubmission revision

The request is now USD 50,000 with an initial USD 5,000 advance and completion payments of USD 10,000, USD 15,000 and USD 20,000. Future mainnet launch may be planned; no current mainnet deployment is claimed. External users, provider commitments and integration remain future work. See SCF_REQUIREMENTS_CHECK.md, TEAM_RISK_PROFILE.md and PILOT_PLAN.md.
