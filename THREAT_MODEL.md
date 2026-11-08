# Testnet pilot threat model

This is an engineering threat model, not an independent audit. Only worthless testnet assets are in scope.

| Threat | Current control | Remaining limitation / operator response |
| --- | --- | --- |
| Caller supplies engine address to drain or mutate reserve | Engine/reward-caller require_auth plus configured-address checks; negative regression tests | Fix applies only to newly built/deployed source. Match WASM hashes; do not infer old IDs are fixed. |
| Duplicate claim or withdrawal consumes another user's capacity | Per-position locks/claims, owner checks, one-time claim flags and queued LP withdrawals | Reconcile token balance against total_assets and restricted balances after every pilot scenario. |
| Wrong price denomination or decimals | Distinct seven-decimal tokens, fixed oracle pair, explicit payout-unit formula | Operators must verify upstream quote currency and issuer/token IDs. Code does not prove token authenticity or redeemability. |
| Publisher collusion or compromised upstream | Two matching fallback votes, epoch isolation, freshness and deviation bounds | Two publishers can collude; upstream refresh is a separate trusted path; admin can rotate publishers. Use separate operators and publish role ownership. |
| Oracle outage, evicted recent price or delayed settlement | Historical buckets retain accepted expiry data; no eligible observation leaves collateral locked | No automatic timeout exit; archived data may need restoration. Alert on AwaitingOracle and missing 600-second expiry coverage. |
| Real depeg filtered by deviation guard | Pilot limit permits 100% downward deviation with strictly positive prices | Very large upward recovery can be rejected; no zero-price observation. Publish outages and avoid claiming universal settlement availability. |
| Reserve admin changes trusted engine while liabilities exist | Admin authorization is required | No timelock or enforced safe migration. Do not reuse old reserves; publish admin actions and reconcile liabilities before changes. |
| LP dilution or loss, correlated claims | Maximum payout reserved before position acceptance, utilization/concentration limits | Full collateralization does not validate pricing or make LP returns safe. Monitor claim losses, NAV and capacity. |
| Misleading readiness or AI output | Historical registry labels, AI disclosure, evidence requirements, no automatic active-registry update | Founder review, independent review and live lifecycle evidence are still required. |

Monitor on the selected testnet deployment: token balance/accounting mismatches (stop sales and investigate), missing or stale price (stop sales; restore feed), outstanding AwaitingOracle positions (investigate expiry coverage), claim/reserve utilization changes, unexpected role/configuration changes and storage expiry. Alert thresholds must be attached to actual deployed config and measured balances; this document does not claim a running alert service.
