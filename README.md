# CoverFi Smart Contracts (Open Source)

CoverFi is a stablecoin loss-protection and payment protocol for Stellar/Soroban. This workspace contains the open-source smart contracts and tests; it does not include frontend, backend, or AI logic.

## Contracts

- `protection_engine`: Main user-facing contract. It quotes protection, creates positions, locks maximum payout capacity, settles only after expiry, and coordinates payout claims plus protected-principal withdrawals.
- `protected_balance_vault`: Holds protected principal by position ID. Only the configured protection engine can instruct deposits and withdrawals.
- `reserve_vault`: Holds reserve liquidity, mints provider shares, collects premiums, locks position-specific maximum payout capacity, matures unused underwriting premium, records claimable payouts, pays keepers, and manages withdrawal requests.
- `oracle_adapter`: Stores fresh and historical scaled price observations, with source reads and fallback publisher quorum support.
- `username_registry`: Maps lowercase CoverFi usernames to Stellar addresses with paid leases, expiry, renewal, rename, and release behavior.
- `receipt_registry`: Stores receipt proof metadata, rejects duplicate payment hashes, grants receipt access, anchors receipt batches, and links disputes to receipt records.
- `zk_verifier`: Anchors account-bound proof status, registers circuit verifier hashes, exposes subject proof lookups, and includes a BN254 pairing-check entry point for Protocol 25 verifier integrations.

## V2 Protection Flow

For a user protecting `1000` units of XLM principal with a CFTUSD payout asset:

1. The user calls `protection_engine.quote_position`.
2. The quote records the current fresh oracle price as the entry price and returns maximum payout, premium components, utilization, concentration, volatility, safety margin, automation fee, and total due.
3. The user calls `protection_engine.create_position`.
4. Protected principal moves from the user to `protected_balance_vault`.
5. The premium and automation fee move to `reserve_vault`.
6. `reserve_vault` locks the position-specific maximum payout in CFTUSD.
7. The position is stored as `Active` until expiry.

Prices use 8-decimal scaling:

```txt
1.00 = 100_000_000
0.98 = 98_000_000
0.94 = 94_000_000
```

After expiry, anyone can call `settle_position`. The engine reads the latest valid observation at or immediately before expiry:

```txt
loss = entry_price - settlement_price
payout = protected_amount * loss / 100_000_000
final_payout = min(payout, maximum_payout)
```

If no valid expiry observation exists, the position moves to `AwaitingOracle` and can be settled later once an eligible observation is available. If the payout is greater than zero, the owner can call `claim_payout`; the owner can withdraw protected XLM principal after settlement through `withdraw_principal`.

## Interaction Order

1. Build and deploy all seven contracts.
2. Initialize `oracle_adapter` with source, fallback publishers, and oracle policy.
3. Initialize `reserve_vault` with admin and treasury, then configure engine/oracle references after the engine address is known.
4. Initialize `protected_balance_vault` with admin and engine address.
5. Initialize `protection_engine` with admin, vault addresses, oracle address, protected asset, payout asset, max payout bps, and max oracle age.
6. Initialize `username_registry`, `receipt_registry`, and `zk_verifier`.
7. Fund reserve liquidity with CFTUSD and fund the automation balance.
8. Refresh oracle observations.
9. Users create protection through `create_position`.
10. Anyone settles expired positions through `settle_position`.
11. Owners claim payouts and withdraw principal through `claim_payout` and `withdraw_principal`.
12. Reserve providers use `request_withdraw`, `cancel_withdraw`, and `execute_withdraw`.

## Treasury And Protocol Fee Routing

The V2 reserve vault routes the protocol-fee portion automatically during `collect_premium_from`, which is called by `protection_engine.create_position`. This means the treasury transfer happens in the same signed position-creation flow as the user premium payment.

Default routing is:

| Bucket | Default |
| --- | --- |
| Underwriting | `7800` bps of risk premium |
| Protocol treasury | `1200` bps of risk premium |
| Safety | `700` bps of risk premium |
| Automation remainder | `300` bps of risk premium plus fixed automation fee |

The admin can update the treasury wallet with `reserve_vault.set_treasury(admin, treasury)` and can set the protocol fee between `1000` and `1500` bps with `reserve_vault.set_protocol_fee_bps(admin, fee_bps)`. Approved partners receive their reward from the protocol-fee bucket before the remaining protocol amount is sent to treasury.

The current published testnet contract IDs were initialized before these controls existed. To use them on-chain, deploy or upgrade the V2 reserve and protection contracts, initialize the reserve vault with the intended treasury public key, then publish the fresh contract IDs after a smoke test.

## Running Tests

From this folder:

```powershell
cargo test --workspace
```

If Windows Application Control blocks generated Rust build scripts or test binaries, run the same command from a developer-trusted folder, a trusted CI runner, or an environment where the generated binaries are allowed. A policy error such as `4551` is a host execution-policy block, not a contract assertion failure.

## Building Contracts

Install the Stellar CLI and Rust target, then build the workspace:

```powershell
rustup target add wasm32v1-none
cargo build --workspace --release --target wasm32v1-none
```

Or build one contract at a time:

```powershell
cargo build -p protection_engine --release --target wasm32v1-none
cargo build -p protected_balance_vault --release --target wasm32v1-none
cargo build -p reserve_vault --release --target wasm32v1-none
cargo build -p oracle_adapter --release --target wasm32v1-none
cargo build -p username_registry --release --target wasm32v1-none
cargo build -p receipt_registry --release --target wasm32v1-none
cargo build -p zk_verifier --release --target wasm32v1-none
```

No mainnet deployment is performed by this repository. V2 Testnet IDs must be published only after a fresh deployment report and smoke test verify the full lifecycle.
