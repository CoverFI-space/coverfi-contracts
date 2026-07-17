# CoverFi Smart Contracts (Open Source)

CoverFi is a stablecoin loss protection and payment protocol for Stellar/Soroban. This workspace contains the open-source smart contracts and tests; it does not include frontend, backend, or AI logic.

## Contracts

- `protection_engine`: Main user-facing contract. It creates protection positions, calculates fees, locks reserve capacity, checks oracle prices, triggers claims, expires positions, and coordinates principal and payout withdrawals.
- `protected_balance_vault`: Holds protected principal by position ID. Only the configured protection engine can instruct deposits and withdrawals.
- `reserve_vault`: Holds reserve liquidity, accepts deposits, collects non-refundable protection premiums, locks maximum payout capacity for active positions, releases unused capacity, records claims into epochs, and reserves allocated withdrawals.
- `oracle_adapter`: Stores scaled stablecoin prices set by the configured admin/oracle source.
- `username_registry`: Maps lowercase CoverFi usernames to Stellar addresses.
- `receipt_registry`: Stores receipt proof metadata, rejects duplicate payment hashes, grants receipt access, anchors receipt batches, and links disputes to receipt records.

## Money Flow

For a user protecting `1000` units with a `1%` fee and a `10%` max payout cap:

1. The user calls `protection_engine.create_position`.
2. `1000` principal moves from the user to `protected_balance_vault`.
3. `10` fee moves from the user directly to `reserve_vault` as premium accounting.
4. `reserve_vault` locks the maximum payout capacity, for example `100`.
5. The position is stored as `Active`.

Prices use 8-decimal scaling:

```txt
1.00 = 100_000_000
0.98 = 98_000_000
0.94 = 94_000_000
```

If the oracle price drops below the entry price before expiry:

```txt
payout = protected_amount * (100_000_000 - current_price) / 100_000_000
final_payout = min(payout, protected_amount * max_payout_bps / 10_000)
```

The user can approve a claim, then withdraw only the amount allocated to that claim through the reserve epoch policy. If no trigger happens before expiry, the position expires, locked capacity is released, and the user can withdraw principal only.

## Interaction Order

1. Deploy/register all contracts.
2. Initialize `protection_engine` with admin and all vault/oracle addresses.
3. Initialize `protected_balance_vault` with admin and engine address.
4. Initialize `reserve_vault` with admin and engine address.
5. Initialize `oracle_adapter` with admin.
6. Initialize `receipt_registry` with admin.
7. Admin funds `reserve_vault.deposit_reserve`.
8. Admin/oracle sets initial price through `oracle_adapter.set_price`.
9. Users create protection with `protection_engine.create_position`.
10. Anyone can call `check_and_trigger` while the position is active and unexpired.
11. Users claim payouts and/or withdraw principal through `protection_engine`.
12. Operators close or allocate reserve epochs when claims require reserved withdrawals.

## Running Tests

From this folder:

```powershell
cargo test
```

If Windows Application Control blocks build scripts in the workspace target directory, clean the old build outputs after updating Rust and retry:

```powershell
cargo clean
cargo test
```

If your machine still blocks generated build scripts, run the same command from a developer-trusted folder or allow the generated Rust build-script binaries in Windows Security / App Control. The contracts themselves do not require deployment to run unit tests.

## Building Contracts

Install the Stellar/Soroban CLI for your local toolchain, then build the workspace:

```powershell
rustup target add wasm32v1-none
cargo build --release --target wasm32v1-none
```

Or build one contract at a time:

```powershell
cargo build -p protection_engine --release --target wasm32v1-none
cargo build -p protected_balance_vault --release --target wasm32v1-none
cargo build -p reserve_vault --release --target wasm32v1-none
cargo build -p oracle_adapter --release --target wasm32v1-none
cargo build -p receipt_registry --release --target wasm32v1-none
```

No deployment is performed by this repository.
