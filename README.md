# CoverFi testnet protection contracts

One protected USDC test asset, one distinct test payout token, and four pilot contracts: protection_engine, reserve_vault, protected_balance_vault and oracle_adapter. **Testnet only. No mainnet deployment is claimed.**

This source revision has not been deployed or matched to historical contract IDs. It changes initialization ABIs and requires a fresh suite, client compatibility checks, reserve seed and on-chain smoke verification before activation. Do not repoint existing funded reserves or open positions to it.

## Source and verification

- [Code changes, payout units, oracle policy and compatibility](TESTNET_RELEASE.md)
- [Threat model and remaining limitations](THREAT_MODEL.md)
- [Evidence and deployment status](PROJECT_STATUS.md)
- [Historical registry: unverified IDs](contracts.json)

The default Cargo members and pilot deployment tool include only the four contracts above. Username registry, receipt registry and ZK verifier remain as legacy workspace source, outside the funded release. The engine no longer calls a proof verifier. Three other contracts mentioned in older architecture materials exist only in a separate local workspace and are not claimed as public release evidence.

## Test and build

```sh
cargo test --locked
cargo test --workspace --locked
python -m unittest discover -s scripts -p 'test_*.py'
cargo build --locked --release --target wasm32v1-none -p protection_engine -p reserve_vault -p protected_balance_vault -p oracle_adapter
```

The first command tests the pilot; --workspace also tests legacy contracts. Local verification on 8 November 2026 passed 51 workspace unit tests (33 in the pilot) and 7 deployment-boundary tests. GitHub Actions builds the four WASMs and publishes hashes/artifacts. This is not an audit or an on-chain pilot result.

## Deployment

Use the field template in scripts/pilot-config.example.json, replacing placeholders with verified testnet public addresses and a local Stellar CLI identity alias. No private keys belong in that file.

```sh
python scripts/deploy_testnet.py --config pilot-config.local.json
```

This is a dry-run. Explicit --execute signs testnet deployment/initialization transactions. The runner uses a fixed testnet RPC/passphrase and refuses other networks; it never automatically activates registry IDs. It does not seed reserves or submit price observations. See TESTNET_RELEASE.md for the complete verification boundary.

## Funding revision

- [Focused USD 40,000 / 12-week scope](FUNDING_SCOPE.md)
- [Updated application answers](SCF_APPLICATION_REVISED.md)
- [Business and sustainability plan](BUSINESS_PLAN.md)
- [AI assistance disclosure](AI_DISCLOSURE.md)
- [Response to voter feedback](REVIEW_RESPONSE.md)

LP commitments, third-party users and a partner integration remain evidence targets. Current team underwriting experience and independent review gaps remain disclosed. Prototype prices are not actuarially validated; no guaranteed returns or USD reimbursement are claimed.
