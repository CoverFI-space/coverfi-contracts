$SourceAccount = "GAUA4NE5ELCWHHWNJSNJH3TRHAVZJPK3UMMVL6XSHAOTQQQ2PS2KV7YZ"
$Network = "testnet"
$ReleaseDir = "D:\Stellar\logic-pages\contracts\target\wasm32v1-none\release"

stellar contract deploy `
  --wasm "$ReleaseDir\oracle_adapter.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000001 `
  --alias depositfree_oracle_adapter `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\protected_balance_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000002 `
  --alias depositfree_protected_balance_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\premium_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000003 `
  --alias depositfree_premium_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\reserve_vault.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000004 `
  --alias depositfree_reserve_vault `
  --sign-with-lab

stellar contract deploy `
  --wasm "$ReleaseDir\protection_engine.wasm" `
  --source-account $SourceAccount `
  --network $Network `
  --salt 0000000000000000000000000000000000000000000000000000000000000005 `
  --alias depositfree_protection_engine `
  --sign-with-lab
