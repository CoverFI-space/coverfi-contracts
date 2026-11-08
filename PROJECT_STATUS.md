# CoverFi evidence and release status

Reviewed 8 November 2026. This is a source/documentation review, not an audit or a live ledger attestation.

| Item | Evidence and limitation |
| --- | --- |
| Public source | Seven crates in this repository's Cargo workspace. |
| Local-only source | A separate local workspace contains quorum_oracle_source, payment_lock_engine and floor_shield_engine. Their implementation is not evidence in this public release. |
| Mainnet | No verified deployment supplied. Mainnet is recorded as not deployed; unsigned deployment artifacts are not a deployment. |
| Testnet | Historical IDs are in contracts.json. They have not been revalidated for this revision or matched to this public source revision. |
| Reserve | The historical local registry says USDC reserves were unfunded. Current balances have not been checked. No externally verified LP commitment is published. |
| Users and partners | The team reports Tael and Fianza/Finza founders communicated with it and tried the product. Public confirmation, activity records and integration evidence have not been supplied. Confirm the second partner's name before submitting. |
| Funding support | The team reports more than USD 1,000 from Stellar Build Delhi. Award/payment evidence and exact amount have not been supplied. This is not classified as LP capital or revenue. |
| Team | Pallavi Jain, Garvit Arora and Piyush. No insurance, actuarial or protocol underwriting specialist is claimed. |
| AI | Cursor, Claude and GPT were used; see AI_DISCLOSURE.md. |
| Audit | No independent audit report was supplied. Internal tests/reviews are not an independent audit. |

## Public-source versus local-source drift

The published protection_engine uses oracle_adapter and contains an optional high-value zk_verifier check. The separate local engine uses admin-submitted price observations. Historical deployed IDs cannot be assumed to implement either revision without WASM hash and configuration checks.

The new funding scope is one USDC testnet market. It does not mean that market has been deployed or validated. Narrowing the source into a minimal release, removing the proof dependency, and verifying the payout denomination are milestone work. Do not describe the seven-crate legacy workspace as a finished minimal depeg engine.

## Reviewer evidence checklist

- Release commit, WASM SHA-256 hashes, network, token contract and issuer, engine/vault configuration, privileged roles and deployment transaction hashes.
- Reserve balances, locked liabilities, reserved claims and reconciliation at a recorded ledger/time.
- Position creation, expiry settlement, claim and principal withdrawal transactions from third-party testers, with consent to identify participants.
- Named LP commitment letters with amount, denomination, conditions, capital-at-risk acknowledgement and permission to publish. Testnet tokens and awards must be separate from real committed capital.
- Partner-owned integration URL, partner confirmation and successful end-to-end transaction.
- Current CI run and independent review scope/report. Old test totals are historical claims until reproduced.

Canonical registry: https://github.com/CoverFI-space/coverfi-contracts/blob/main/contracts.json
