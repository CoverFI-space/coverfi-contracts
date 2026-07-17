$SourceAccount = "GAUA4NE5ELCWHHWNJSNJH3TRHAVZJPK3UMMVL6XSHAOTQQQ2PS2KV7YZ"
$Network = "testnet"
$ReleaseDir = "D:\Stellar\coverfi-contracts\target\wasm32v1-none\release"

stellar contract deploy `
  --wasm "$ReleaseDir\protected_balance_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000002 `
  --alias depositfree_protected_balance_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\reserve_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000003 `
  --alias coverfi_reserve_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\oracle_adapter.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000004 `
  --alias coverfi_oracle_adapter `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\protection_engine.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000005 `
  --alias depositfree_protection_engine `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\username_registry.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000006 `
  --alias coverfi_username_registry `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\receipt_registry.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000007 `
  --alias coverfi_receipt_registry `
  --sign-with-lab
