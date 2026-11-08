# Focused testnet code release

8 November 2026. Source release only; these changes are not represented as deployed to any network. Mainnet is not deployed by this release.

## Release contents

The default Cargo members and deployment runner include only protection_engine, reserve_vault, protected_balance_vault and oracle_adapter. The three legacy crates remain in the workspace for historical source and regression tests, but are not deployed by the pilot runner. The engine has no ZK contract, proof threshold, MFA or KYC call dependency.

Reserve methods now require authorization from the configured engine; keeper rewards require authorization from the configured engine or oracle. Passing a known address as an argument is not authorization. The older public source did not enforce these checks. This source fix does not modify existing contract deployments.

## Market and payout units

The protected token is the configured USDC test asset. The reserve, premiums, automation fees and claims use a distinct seven-decimal test payout token. Both token contracts must expose seven decimals. The oracle is bound to that exact protected/payout pair and the engine validates it at initialization.

Prices are **payout-token units per one protected token**, scaled by 100,000,000. The test payout token is modeled as the quote unit; it is not claimed to be redeemable for USD. An upstream USD price may be used only in a clearly labeled simulation assuming one test payout unit represents one reference USD. Do not treat a USDC/USD feed as a USDC/XLM feed, or as USDC per USDC.

For raw seven-decimal amount A and scaled prices E/P:

```text
entry_notional = floor(A * E / 100000000)         # payout-token base units
maximum_payout = floor(entry_notional * cap_bps / 10000)
payout = min(floor(A * max(E - P, 0) / 100000000), maximum_payout)
```

Example: 100 USDC test units entered at 1.00 and settled at 0.94 pay 6 test payout units, subject to the cap. A 10% cap pays at most 10 test payout units at that entry price. No guaranteed USD reimbursement, licensed insurance, real USDC redemption or LP yield is claimed. Fees are charged separately in the payout token. All testnet assets have no monetary value.

## Oracle policy

The public release retains oracle_adapter; it does not copy the local single-admin engine price feed. The pilot config enables two of three distinct authorized publishers to submit exactly matching price/timestamp observations. Distinct addresses do not establish independent operators. The configured upstream source is also trusted when refresh/refresh_at is used; do not describe the full adapter as quorum-only.

The adapter rejects unknown assets, nonpositive prices, future timestamps, submissions older than the configured freshness limit and movement beyond the configured deviation limit. The testnet runner sets freshness to 3,600 seconds and deviation to 10,000 bps. This permits deep downward moves, but a recovery exceeding 100% relative to the previous observation is rejected; monitoring and explicit operator handling are still needed. These thresholds are operational test settings, not an underwriting model.

Publisher rotation starts a new voting epoch. Old votes cannot combine with votes after reconfiguration. Accepted observations cannot be replaced at an existing timestamp and must arrive in increasing timestamp order. Delayed observations may be accepted only within the freshness bound and only before a later observation is stored. A publisher quorum can still misreport or backdate within that bound; there is no cryptographic proof of observation time or market correctness.

Expiry settlement accepts only the latest observation at or before expiry and within 600 seconds of expiry. Otherwise it stays AwaitingOracle and keeps capacity/principal locked. It does not settle against a post-expiry price. Historical minute buckets retain accepted prices beyond the 64-observation quote ring, with a 120-day storage TTL target when written. Archived data may require restoration. No timeout refund or arbitrary later-price substitution is implemented.

## Compatibility and deployment

Engine and oracle initialization ABIs and stored config shapes changed. This is a fresh-deployment release, not an in-place upgrade. Historical contract IDs and the app/backend local source do not automatically become compatible. Do not repoint an old funded reserve to the new engine or abandon open positions. Deploy a new four-contract suite and verify client compatibility before activating it.

The runner uses fixed public Testnet RPC and passphrase, defaults to dry-run and requires --execute to sign. It does not seed reserves, publish oracle observations, activate the app or edit contracts.json. It records initialized_not_smoke_verified, not a successful pilot. No signing identity or verified test asset configuration has been selected for a live deployment in this revision.

```sh
python scripts/deploy_testnet.py --config pilot-config.local.json
# After configuration and signer review, explicitly execute:
python scripts/deploy_testnet.py --config pilot-config.local.json --execute
```

Use scripts/pilot-config.example.json as a field template. Replace every placeholder with a verified public testnet value and use an existing local Stellar CLI identity alias. Do not put secret keys in the JSON file. The CLI verifies actual address checksums; the runner performs structural checks first. The source account must be able to authorize the selected admin for initialization.

## Verification

Local verification on 8 November 2026: 51 workspace unit tests passed, including 33 tests across the four pilot contracts; 7 Python deployment-boundary tests passed. Test coverage includes depeg payout/cap, no-loss exit, stale entry, missing expiry, history rollover, duplicate claims, withdrawal safety, spoofed engine/reward calls, token decimals, oracle pair mismatch, publisher rotation and future observations. A lifecycle test mocks only buyer authorization and confirms the actual engine authorizes its vault calls.

See GitHub Actions for the release commit's native tests and four-contract WASM build artifacts/hashes. Native tests and WASM builds are not live testnet smoke tests or an independent audit. Generated deployment reports remain unverified until balances and transaction receipts establish the deployed lifecycle.
