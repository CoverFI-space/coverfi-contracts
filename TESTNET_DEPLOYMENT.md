# Verified testnet deployment

Verified at 2026-11-08T18:20:37.317983+00:00, ledger 5627678. **Testnet only. No mainnet deployment is claimed.**

## Source and contract IDs

Source commit: [6e85036b236df94c550434efdbf7e9287d417921](https://github.com/CoverFI-space/coverfi-contracts/commit/6e85036b236df94c550434efdbf7e9287d417921). The on-chain WASM hashes for all four contracts match the recorded build hashes. Built with Rust/Cargo 1.97.1 for wasm32v1-none and deployed with Stellar CLI 30.0.1 with CLI optimization disabled. Exact tool details are recorded in the deployment report.

| Contract | Testnet ID |
| --- | --- |
| oracle_adapter | [CDMP7NOSFTN77TSRN5GVXML7KQVF4P6O77C5SWF6OACHYXOZVXMHM2BA](https://stellar.expert/explorer/testnet/contract/CDMP7NOSFTN77TSRN5GVXML7KQVF4P6O77C5SWF6OACHYXOZVXMHM2BA) |
| reserve_vault | [CD76MVLI3U2WP4W7WAABLXFI65P2EZXPQ7II42PG2WYIGRCL236XAVFM](https://stellar.expert/explorer/testnet/contract/CD76MVLI3U2WP4W7WAABLXFI65P2EZXPQ7II42PG2WYIGRCL236XAVFM) |
| protected_balance_vault | [CBLNUMR2PE7B4EGKJ3ZH6SU2BL3EWEVEJOAVCSC4Y6MZHG7N7JMSEK3E](https://stellar.expert/explorer/testnet/contract/CBLNUMR2PE7B4EGKJ3ZH6SU2BL3EWEVEJOAVCSC4Y6MZHG7N7JMSEK3E) |
| protection_engine | [CDYOKZH532VB2RHVEFPBEDK3IAZNBXV6JAPHMGHI7GXKCFMECKJIREXB](https://stellar.expert/explorer/testnet/contract/CDYOKZH532VB2RHVEFPBEDK3IAZNBXV6JAPHMGHI7GXKCFMECKJIREXB) |

## Controlled lifecycle evidence

Two team-controlled positions each protected 100 mock USDC units for 120 seconds. A simulated 1.00 to 0.94 price change produced a 6 CFUSD claim. The second position settled without a loss. Principal was returned for both positions, and a repeated claim was rejected in simulation before submission.

| Action | Successful testnet transaction |
| --- | --- |
| Depeg position opened | [205b96e5a34702b256c8fab9989f4c5f14fa186b2b9251b2476a437e4979e9db](https://stellar.expert/explorer/testnet/tx/205b96e5a34702b256c8fab9989f4c5f14fa186b2b9251b2476a437e4979e9db) |
| Depeg settlement | [ac7298356c11e58cd84509d3d83c938ad234fe9864a3ebe17c5526022b73cb7a](https://stellar.expert/explorer/testnet/tx/ac7298356c11e58cd84509d3d83c938ad234fe9864a3ebe17c5526022b73cb7a) |
| 6 CFUSD claim paid | [5c2c28604fcdeba081ae96b440478f88e3c64e10e8e8b5d6ed92e2614d644a7a](https://stellar.expert/explorer/testnet/tx/5c2c28604fcdeba081ae96b440478f88e3c64e10e8e8b5d6ed92e2614d644a7a) |
| Principal returned | [ba69df42b3c60ef1bab673d67de67928fd48addbc3d16c75c1c0660dc4754ddb](https://stellar.expert/explorer/testnet/tx/ba69df42b3c60ef1bab673d67de67928fd48addbc3d16c75c1c0660dc4754ddb) |
| No-loss settlement | [d99d1b895e080374b34262e99a78238808f25602204204eff224fc249322e782](https://stellar.expert/explorer/testnet/tx/d99d1b895e080374b34262e99a78238808f25602204204eff224fc249322e782) |
| No-loss principal returned | [43661dd92f73e14b43ff361e895be91c769efa1253a39be9e7a0ccaa07e008a2](https://stellar.expert/explorer/testnet/tx/43661dd92f73e14b43ff361e895be91c769efa1253a39be9e7a0ccaa07e008a2) |

Seed: 1,000 CFUSD test reserve plus 10 CFUSD automation funds. Final recorded reserve token balance and total_assets both equal 1,004.96 CFUSD (10049600000 seven-decimal base units). Locked liabilities and reserved claims are zero at the verification checkpoint. This balance is a snapshot, not a promise of future capacity.

## Assets, roles and limitations

- Protected token: CAXJTCW55YLLJC4WUTIEHGFMZZCUP7WGDFUJXAFHSKVBGSJWTNWGOSWC, USDC issued by the dedicated test issuer GCW6NZYQ4G3OLKWH7XBT4FBCWJDXODGD4BVZQM4FPZOXUUOV5GQBWDHA. **This is mock USDC, not Circle-issued USDC.**
- Premium/payout token: CAY3UEZ4J3XTD54HFNDOL2LSTBDZ57WK4I5S5HNI53K6YNE2BNXF6KWJ, CFUSD issued by the same test issuer. Both have seven decimals and no monetary value.
- The deployer, admin, issuer, publishers, reserve provider, treasury and buyer are team-controlled test identities. Three publisher addresses do not constitute independent underwriting or oracle operators.
- The oracle has source=null and uses two matching fallback publishers. Prices are controlled simulations; no continuous market-data worker is configured.
- The existing public app/backend have not been repointed to this new ABI/deployment. These transactions verify contract interaction directly, not an integrated consumer launch.
- No external LP commitment, third-party user demand, real capital, independent audit, guaranteed payout or mainnet deployment is claimed.

[Download the exact deployed WASMs and checksums](https://github.com/CoverFI-space/coverfi-contracts/releases/tag/testnet-pilot-2026-11-08). The separate read-only verification report confirms four matching hashes and 34 successful transactions.

## Recheck the evidence

Use [contracts.json](contracts.json), [deployment receipts](artifacts/pilot-testnet-deployment.json), [smoke results](artifacts/pilot-testnet-smoke.json) and [local build hashes](artifacts/pilot-local-build.json).

```sh
python scripts/verify_testnet_evidence.py
```

This verifier reads contract hashes and transaction status without a signing identity. RPC transaction-history retention and testnet resets limit how long a later live verification remains possible. The report preserves the verification time and ledger.
