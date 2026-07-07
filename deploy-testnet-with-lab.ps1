param(
  [Parameter(Mandatory = $true)]
  [string]$SourceAccount,

  [string]$Network = "testnet",
  [string]$ReleaseDir = "$PSScriptRoot\target\wasm32v1-none\release"
)

stellar contract deploy `
  --wasm "$ReleaseDir\oracle_adapter.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000001 `
  --alias coverfi_oracle_adapter `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\protected_balance_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000002 `
  --alias coverfi_protected_balance_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\premium_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000003 `
  --alias coverfi_premium_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\reserve_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000004 `
  --alias coverfi_reserve_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\protection_engine.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000005 `
  --alias coverfi_protection_engine `
  --sign-with-lab
