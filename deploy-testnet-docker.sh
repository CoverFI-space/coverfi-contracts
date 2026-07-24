#!/usr/bin/env bash
set -euo pipefail

# This runner is intentionally testnet-only. It is invoked inside Docker Desktop
# with the local Stellar CLI identity directory mounted read-only at /root/.config/stellar.
NETWORK="${NETWORK:-testnet}"
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:-alice}"
ADMIN="${ADMIN:-GAWITZOKW5IGYWAYCRLHRMYKDD5SYO3KRWF76DIRT3N4SI2UN2PW2IF2}"
TREASURY="${TREASURY:-GAUA4NE5ELCWHHWNJSNJH3TRHAVZJPK3UMMVL6XSHAOTQQQ2PS2KV7YZ}"
AUTOMATION="${AUTOMATION:-$TREASURY}"
PROTECTED_ASSET="${PROTECTED_ASSET:-CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC}"
PAYOUT_ASSET="${PAYOUT_ASSET:-CCYYFPYEUIZ65AV2VW6L3EDXCN3TXNZJ3VWTPYPRKG327J5DHTCTHZEG}"
ORACLE_SOURCE="${ORACLE_SOURCE:-$ADMIN}"
PUBLISHER_1="${PUBLISHER_1:-$ADMIN}"
PUBLISHER_2="${PUBLISHER_2:-GBQXDOKI5PXHNF7TRURTHTUZC6QYAZMWXFATL4ADBHW756QG53FMAEKW}"
PUBLISHER_3="${PUBLISHER_3:-GAURIMABJITVKDKFMST74M7QFLNLM5QEVIEXPKANIHKONHY5IHQ3PU32}"
RESERVE_FUNDER_ALIAS="${RESERVE_FUNDER_ALIAS:-alice}"
RESERVE_FUNDER="${RESERVE_FUNDER:-$ADMIN}"
# The Docker-mounted signer owns roughly 500 test CFTUSD. Keep the default seed
# below that balance so a repeatable testnet deployment completes without an issuer key.
RESERVE_SEED="${RESERVE_SEED:-40000000000}"
AUTOMATION_SEED="${AUTOMATION_SEED:-2000000000}"

if [[ "$NETWORK" != "testnet" ]]; then
  echo "Only public testnet deployments are supported by this runner." >&2
  exit 2
fi

export XDG_CONFIG_HOME=/root/.config
mkdir -p artifacts

invoke() {
  stellar contract invoke --id "$1" --source-account "$2" --network "$NETWORK" --send=yes -- "${@:3}"
}

deploy() {
  local wasm="$1" output id
  output="$(stellar contract deploy --wasm "$wasm" --source-account "$SOURCE_ACCOUNT" --network "$NETWORK")"
  id="$(printf '%s' "$output" | grep -oE 'C[A-Z0-9]{55}' | tail -n 1)"
  [[ -n "$id" ]] || { echo "Could not extract a contract ID: $output" >&2; exit 1; }
  printf '%s' "$id"
}

stellar contract build
release_dir="/workspace/target/wasm32v1-none/release"

oracleAdapter="$(deploy "$release_dir/oracle_adapter.wasm")"
reserveVault="$(deploy "$release_dir/reserve_vault.wasm")"
protectedBalanceVault="$(deploy "$release_dir/protected_balance_vault.wasm")"
zkVerifier="$(deploy "$release_dir/zk_verifier.wasm")"
protectionEngine="$(deploy "$release_dir/protection_engine.wasm")"
usernameRegistry="$(deploy "$release_dir/username_registry.wasm")"
receiptRegistry="$(deploy "$release_dir/receipt_registry.wasm")"

invoke "$oracleAdapter" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --source "$ORACLE_SOURCE" --source_asset XLM --source_decimals 8 --max_age_seconds 3600 --max_deviation_bps 2000
invoke "$oracleAdapter" "$SOURCE_ACCOUNT" configure_publishers --admin "$ADMIN" --publisher1 "$PUBLISHER_1" --publisher2 "$PUBLISHER_2" --publisher3 "$PUBLISHER_3" --enabled true
invoke "$reserveVault" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --treasury "$TREASURY"
invoke "$protectedBalanceVault" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --engine "$protectionEngine"
invoke "$zkVerifier" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN"

for circuit in \
  ebd8a6b268bff37dd937e243858a2706e3c8675fde4d6d75c7a195bfb1f4e967 \
  d3d3bd01738ac64cc0ad02a074bb24db188df425d1f586af4e7cce50506696eb \
  125f79a1790bf33732d22f3e73343452ea49699c30e6ee37077a36ba77c82eed \
  90532346634f39a224681334fcad836651adb4309bb66c86ee002ef9606f869f \
  0084e5d60fe2404512e3ecd64fc304b85e017388534c31ee52e9b00c8cf91a3e; do
  invoke "$zkVerifier" "$SOURCE_ACCOUNT" register_verifier --admin "$ADMIN" --circuit_id "$circuit" --verifier_hash ad7382453e717eec0da14fd84840f2ad1f7468b4fe8fe70d58dc576016112492 --kind_hash 4abd61919fcf5ad46cb8133e1df6c136a94734afbcb9c5949418ed7ef929d3f9 --enabled true
done

invoke "$protectionEngine" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --protected_balance_vault "$protectedBalanceVault" --reserve_vault "$reserveVault" --oracle_adapter "$oracleAdapter" --zk_verifier "$zkVerifier" --protected_asset "$PROTECTED_ASSET" --payout_asset "$PAYOUT_ASSET" --max_payout_bps 1000 --max_oracle_age_seconds 3600 --high_value_threshold 1000000000
invoke "$reserveVault" "$SOURCE_ACCOUNT" configure_protocol --admin "$ADMIN" --engine "$protectionEngine" --oracle "$oracleAdapter"
invoke "$usernameRegistry" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --fee_token "$PAYOUT_ASSET" --treasury "$TREASURY" --automation "$AUTOMATION"
invoke "$receiptRegistry" "$SOURCE_ACCOUNT" initialize --admin "$ADMIN" --fee_token "$PAYOUT_ASSET" --treasury "$TREASURY" --automation "$AUTOMATION"

invoke "$reserveVault" "$RESERVE_FUNDER_ALIAS" deposit_reserve --provider "$RESERVE_FUNDER" --token "$PAYOUT_ASSET" --amount "$RESERVE_SEED" --min_shares "$RESERVE_SEED"
invoke "$reserveVault" "$RESERVE_FUNDER_ALIAS" fund_automation --funder "$RESERVE_FUNDER" --token "$PAYOUT_ASSET" --amount "$AUTOMATION_SEED"

cat > artifacts/v2-testnet-deployment.generated.json <<EOF
{
  "schemaVersion": "2.2.0",
  "project": "CoverFi",
  "network": "testnet",
  "generatedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployedWith": "Docker Desktop",
  "sourceAccount": "$SOURCE_ACCOUNT",
  "admin": "$ADMIN",
  "treasury": "$TREASURY",
  "assets": {"protectedPrincipal": {"symbol": "XLM", "tokenContract": "$PROTECTED_ASSET"}, "premiumAndPayout": {"symbol": "CFTUSD", "tokenContract": "$PAYOUT_ASSET"}},
  "contracts": {"oracleAdapter": "$oracleAdapter", "reserveVault": "$reserveVault", "protectedBalanceVault": "$protectedBalanceVault", "zkVerifier": "$zkVerifier", "protectionEngine": "$protectionEngine", "usernameRegistry": "$usernameRegistry", "receiptRegistry": "$receiptRegistry"},
  "reserveSeed": {"reserveDepositedStroops": "$RESERVE_SEED", "automationFundedStroops": "$AUTOMATION_SEED"},
  "smokeTestedAt": null
}
EOF

cat artifacts/v2-testnet-deployment.generated.json
