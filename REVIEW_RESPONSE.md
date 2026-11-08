# Response to voter feedback

Revision prepared 8 November 2026. Documentation changes are not proof that prospective outcomes have occurred.

| Voter concern | Change and remaining evidence |
| --- | --- |
| Scope too broad | Funding scope now covers one USDC testnet market and necessary dependencies. Unrelated products are excluded. Legacy source is retained outside the default four-contract pilot release; proof gating has been removed from the engine. |
| No underwriting expertise | Explicitly disclosed. No team member added at the founder's request. Specialist pricing/reserving review remains a real-value launch gate. This concern is unresolved. |
| No user demand | Tranches target external positions, named LP letters and one partner-owned testnet integration. No commitments or outcomes are fabricated. Letters and activity evidence remain outstanding. |
| Mainnet contradiction | Mainnet claim removed from the revised application; registry says not deployed. Unsigned transaction artifacts do not establish deployment. |
| Three missing public contracts | Seven-crate public inventory is explicit. Three local-only contracts are removed from the funded architecture and deployment evidence; older docs are labeled historical. They are not claimed as newly published source. |
| Broken registry link | Registry now lives in this existing public repository at contracts.json. Fresh testnet IDs are source-matched and smoke-verified; historical IDs are preserved separately as unverified. |
| USD 94,000 too high | Proposed ask reduced to USD 40,000 with a cost allocation and tranches totaling USD 40,000. Old listed tranches totaled USD 84,600, leaving USD 9,400 unexplained. |
| AI disclosure | Cursor, Claude and GPT development use and team-reported line-by-line review disclosed. This revision's AI-assisted drafting is identified. Exact attribution for the original application remains unconfirmed; new revision still needs founder review. |
| No sustainability plan | BUSINESS_PLAN.md separates fees from LP capital, shows low-volume revenue limitations, describes reserve replenishment and sets 3-12 month decision gates. Prices remain prototype parameters. |
| Timeline too long | Proposed 12 weeks from project start, with no promised mainnet launch. Calendar dates and current program tranche requirements must be confirmed when submitting. |

## Verification performed for this revision

- Reviewed all seven public CoverFI-space repository inventories via reference clones; inspected contracts and documentation in detail and compared them with the separate local workspace.
- Checked the public Cargo workspace inventory and source fee constants. Did not perform an independent security audit or review every source line.
- Checked revised document navigation, JSON, local links and Git diffs before publishing.
- Attempted `cargo test --workspace --locked` locally. The first attempt stopped at a Cargo cache permission error. That access issue was resolved in the code revision; 52 workspace unit tests and 8 deployment-boundary tests now pass locally.
- Live testnet deployment, mock balances and controlled wallet transactions are now verified separately in TESTNET_DEPLOYMENT.md. No mainnet activity, partner integration, award payment or external-user demand is attested.

## Before submitting

Founders must approve the USD 40,000 budget and 12-week commitments, confirm partner spelling and permission to name contacts, attach award and pilot evidence, obtain actual named LP letters, identify historical AI drafting where possible, and review every revised answer. Do not mark future pilot milestones as completed.

The code release is documented in TESTNET_RELEASE.md. The reserve caller authorization fix, distinct-token/decimal checks, oracle binding, voting epochs and durable expiry history are implemented. Fresh testnet deployment, matching WASM hashes, two controlled live testnet lifecycles and reserve reconciliation are now published in TESTNET_DEPLOYMENT.md. External-user demand, real LP commitments and public app integration remain outstanding; no mainnet deployment is claimed.
