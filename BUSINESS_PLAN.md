# One-market business and sustainability plan

Planning draft, 8 November 2026. No paid mainnet activity, contracted recurring revenue or validated underwriting price is claimed.

## Customer and product

Validate one short-duration USDC testnet depeg protection market with Stellar wallet users and one distribution partner. The buyer sees the reference price, expiry, payout cap, full premium and settlement rules before signing. This is expiry-price protection: a temporary depeg that recovers before expiry may produce no payout. It does not compensate every loss or track the worst price during the term.

The funded primitive is the protection engine and reserve vault, with only necessary principal custody and price-feed support. Username payments, receipts, payment locks, floor shields, ZK verification, partner invoice APIs and AI assistants are excluded from the funding deliverables.

## Pricing and revenue

The current public source has duration base rates of 30/100/150/250 basis points for up to 1/7/14/30 days, plus volatility, utilization, concentration and safety adjustments, a minimum/cap and fixed automation charge. These are prototype parameters, not actuarially validated depeg prices. A 1% seven-day base rate is not the final all-in quote. The quote returned by the selected release must be tested and disclosed in token units.

The public reserve source defaults to 78% of risk premium for underwriting, 12% for protocol treasury, 7% for safety, and the 3% remainder plus the fixed charge for automation. Partner rewards, when configured, come from the protocol portion. Validate the actual release and on-chain settings before quoting these as deployed terms.

Illustrative revenue only: USD 100,000 of monthly protected notional at an assumed 1% risk premium generates USD 1,000 of premium and USD 120 gross protocol fees at a 12% share, before partner rewards and operating costs. That volume does not fund a team. At an assumed USD 2,000 monthly operating cost, the same assumptions require about USD 1.67 million monthly protected notional before partner rewards. These are sensitivity assumptions, not sales forecasts or approved operating budgets.

## Reserve funding

External LP capital supplies claim capacity; it is not protocol revenue. Publish each provider's permitted name, committed amount, asset, conditions and funding transaction separately. No verified LP commitments are currently published. Team-reported Stellar Build Delhi support of more than USD 1,000 is funding support, not automatically reserve capital.

Premium assigned to underwriting matures according to settlement rules. Paid claims reduce provider assets; premiums and new LP deposits replenish capacity. Publish reserve NAV, liabilities, claims, utilization, withdrawals and loss experience. Pause new sales when capacity is insufficient; never use new buyer principal as claim capital. Do not promise LP returns or treat grant funding as recurring income.

## Payout denomination and oracle risk

If the protected coin and payout coin are both USDC, payout purchasing power falls during a USDC depeg. A nominal 60 USDC payout at USD 0.94 is worth USD 56.40. The legacy engine's price-difference formula assumes the quote currency and payout units align; using a depegging payout token requires explicit conversion semantics. The one-market release must resolve and test this before a pilot claims USD loss protection. A different settlement asset introduces its own issuer, liquidity and depeg risks. Publish the chosen specification and cap; do not silently switch assets.

The current local price path trusts an admin publisher; the older public source uses an adapter. Neither source presence nor a quorum contract proves independent deployed oracle operation. Freeze a source revision, disclose authority and backfill policy, and test missing/stale/manipulated observations. Price correctness and correlated losses remain risks even with maximum payouts collateralized.

## Months 3 to 12

- Months 1-3: complete the testnet pilot, measure quote-to-purchase conversion, user comprehension, settlement completion, cost per position, LP interest and partner demand. No real protection sales in this grant plan.
- Months 4-6: assess pilot results and obtain specialist pricing/reserving review, independent security review, operational readiness and appropriate legal review before any real-value launch. No specialist team member is claimed as hired.
- Months 7-12: consider a separately approved capped real-value pilot only if those gates and actual LP capital are met. Reinvest defined operating surplus in operations and, if approved and disclosed, protocol-owned reserve capital. Consider paid integration/support only after demonstrated demand; no enterprise contracts are claimed.

If customers will not pay a price that covers risk and operations, or LPs will not accept the loss profile, stop expansion and publish findings. Track operating runway separately from restricted reserve capital. The proposed grant budget covers this pilot; ongoing support would require earned fees, confirmed service revenue or separately disclosed additional funding.
