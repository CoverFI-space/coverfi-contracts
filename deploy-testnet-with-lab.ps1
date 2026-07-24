param(
  [string]$Network = "testnet",
  [string]$SourceAccount = "",
  [string]$Admin = "",
  [string]$Treasury = "",
  [string]$Automation = "",
  [string]$ProtectedAssetContractId = "",
  [string]$PayoutAssetContractId = "",
  [string]$OracleSourceContractId = "",
  [string]$OracleSourceAsset = "XLM",
  [int]$OracleSourceDecimals = 14,
  [int]$OracleMaxAgeSeconds = 3600,
  [int]$OracleMaxDeviationBps = 500,
  [int]$MaxPayoutBps = 1000,
  [string]$HighValueVerificationThreshold = "1000000000",
  [string]$Publisher1 = "",
  [string]$Publisher2 = "",
  [string]$Publisher3 = "",
  [string]$ZkVerifierHash = "ad7382453e717eec0da14fd84840f2ad1f7468b4fe8fe70d58dc576016112492",
  [string]$ZkKindHash = "4abd61919fcf5ad46cb8133e1df6c136a94734afbcb9c5949418ed7ef929d3f9",
  [switch]$SkipBuild,
  [switch]$SignWithLab,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$Workspace = Split-Path -Parent $MyInvocation.MyCommand.Path
$ReleaseDir = Join-Path $Workspace "target\wasm32v1-none\release"
$ArtifactDir = Join-Path $Workspace "artifacts"
$ReportPath = Join-Path $ArtifactDir "v2-testnet-deployment.generated.json"

$Contracts = @(
  @{ key = "oracleAdapter"; package = "oracle_adapter"; wasm = "oracle_adapter.wasm"; alias = "coverfi_v2_oracle_adapter" },
  @{ key = "reserveVault"; package = "reserve_vault"; wasm = "reserve_vault.wasm"; alias = "coverfi_v2_reserve_vault" },
  @{ key = "protectedBalanceVault"; package = "protected_balance_vault"; wasm = "protected_balance_vault.wasm"; alias = "coverfi_v2_protected_balance_vault" },
  @{ key = "zkVerifier"; package = "zk_verifier"; wasm = "zk_verifier.wasm"; alias = "coverfi_v2_zk_verifier" },
  @{ key = "protectionEngine"; package = "protection_engine"; wasm = "protection_engine.wasm"; alias = "coverfi_v2_protection_engine" },
  @{ key = "usernameRegistry"; package = "username_registry"; wasm = "username_registry.wasm"; alias = "coverfi_v2_username_registry" },
  @{ key = "receiptRegistry"; package = "receipt_registry"; wasm = "receipt_registry.wasm"; alias = "coverfi_v2_receipt_registry" }
)

function Require-Value($Name, $Value) {
  if ([string]::IsNullOrWhiteSpace($Value)) {
    throw "$Name is required for a real V2 deployment. Re-run with -DryRun to print commands without executing."
  }
}

function Invoke-CoverFiCommand([string[]]$CliArgs) {
  $line = "stellar " + ($CliArgs -join " ")
  Write-Host $line
  if ($DryRun) {
    return ""
  }
  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  $output = & stellar @CliArgs 2>&1 | ForEach-Object { $_.ToString() }
  $exitCode = $LASTEXITCODE
  $ErrorActionPreference = $previousErrorActionPreference
  $text = ($output | Out-String).Trim()
  if ($exitCode -ne 0) {
    throw $text
  }
  if ($text) {
    Write-Host $text
  }
  return $text
}

function Parse-ContractId([string]$Text) {
  $match = [regex]::Match($Text, "C[A-Z0-9]{55}")
  if (-not $match.Success) {
    throw "Could not parse contract ID from Stellar CLI output: $Text"
  }
  return $match.Value
}

function Invoke-Contract($ContractId, $Fn, [string[]]$FnArgs) {
  $base = @("contract", "invoke", "--id", $ContractId, "--source-account", $SourceAccount, "--network", $Network)
  if ($SignWithLab) {
    $base += "--sign-with-lab"
  }
  $base += "--"
  $base += $Fn
  $base += $FnArgs
  Invoke-CoverFiCommand $base | Out-Null
}

function Register-ZkCircuit($CircuitId) {
  Invoke-Contract $ids.zkVerifier "register_verifier" @(
    "--admin", $Admin,
    "--circuit_id", $CircuitId,
    "--verifier_hash", $ZkVerifierHash,
    "--kind_hash", $ZkKindHash,
    "--enabled", "true"
  )
}

if (-not $DryRun) {
  Require-Value "SourceAccount" $SourceAccount
  Require-Value "Admin" $Admin
  Require-Value "Treasury" $Treasury
  Require-Value "Automation" $Automation
  Require-Value "ProtectedAssetContractId" $ProtectedAssetContractId
  Require-Value "PayoutAssetContractId" $PayoutAssetContractId
  Require-Value "OracleSourceContractId" $OracleSourceContractId
}

New-Item -ItemType Directory -Force -Path $ArtifactDir | Out-Null

if (-not $SkipBuild) {
  Invoke-CoverFiCommand @("contract", "build") | Out-Null
}

$hashes = @{}
foreach ($contract in $Contracts) {
  $path = Join-Path $ReleaseDir $contract.wasm
  if (-not $DryRun -and -not (Test-Path $path)) {
    throw "Missing WASM: $path"
  }
  if (Test-Path $path) {
    $hashes[$contract.key] = (Get-FileHash -Algorithm SHA256 -Path $path).Hash.ToLowerInvariant()
  } else {
    $hashes[$contract.key] = $null
  }
}

$ids = @{}
foreach ($contract in $Contracts) {
  $wasmPath = Join-Path $ReleaseDir $contract.wasm
  $args = @("contract", "deploy", "--wasm", $wasmPath, "--source-account", $SourceAccount, "--network", $Network, "--alias", $contract.alias)
  if ($SignWithLab) {
    $args += "--sign-with-lab"
  }
  $output = Invoke-CoverFiCommand $args
  $ids[$contract.key] = if ($DryRun) { "<pending:$($contract.key)>" } else { Parse-ContractId $output }
}

Invoke-Contract $ids.oracleAdapter "initialize" @(
  "--admin", $Admin,
  "--source", $OracleSourceContractId,
  "--source_asset", $OracleSourceAsset,
  "--source_decimals", "$OracleSourceDecimals",
  "--max_age_seconds", "$OracleMaxAgeSeconds",
  "--max_deviation_bps", "$OracleMaxDeviationBps"
)

if ($Publisher1 -and $Publisher2 -and $Publisher3) {
  Invoke-Contract $ids.oracleAdapter "configure_publishers" @(
    "--admin", $Admin,
    "--publisher1", $Publisher1,
    "--publisher2", $Publisher2,
    "--publisher3", $Publisher3,
    "--enabled", "true"
  )
}

Invoke-Contract $ids.reserveVault "initialize" @("--admin", $Admin, "--treasury", $Treasury)
Invoke-Contract $ids.protectedBalanceVault "initialize" @("--admin", $Admin, "--engine", $ids.protectionEngine)
Invoke-Contract $ids.zkVerifier "initialize" @("--admin", $Admin)

Register-ZkCircuit "ebd8a6b268bff37dd937e243858a2706e3c8675fde4d6d75c7a195bfb1f4e967"
Register-ZkCircuit "d3d3bd01738ac64cc0ad02a074bb24db188df425d1f586af4e7cce50506696eb"
Register-ZkCircuit "125f79a1790bf33732d22f3e73343452ea49699c30e6ee37077a36ba77c82eed"
Register-ZkCircuit "90532346634f39a224681334fcad836651adb4309bb66c86ee002ef9606f869f"
Register-ZkCircuit "0084e5d60fe2404512e3ecd64fc304b85e017388534c31ee52e9b00c8cf91a3e"

Invoke-Contract $ids.protectionEngine "initialize" @(
  "--admin", $Admin,
  "--protected_balance_vault", $ids.protectedBalanceVault,
  "--reserve_vault", $ids.reserveVault,
  "--oracle_adapter", $ids.oracleAdapter,
  "--zk_verifier", $ids.zkVerifier,
  "--protected_asset", $ProtectedAssetContractId,
  "--payout_asset", $PayoutAssetContractId,
  "--max_payout_bps", "$MaxPayoutBps",
  "--max_oracle_age_seconds", "$OracleMaxAgeSeconds",
  "--high_value_threshold", $HighValueVerificationThreshold
)

Invoke-Contract $ids.reserveVault "configure_protocol" @("--admin", $Admin, "--engine", $ids.protectionEngine, "--oracle", $ids.oracleAdapter)
Invoke-Contract $ids.usernameRegistry "initialize" @("--admin", $Admin, "--fee_token", $PayoutAssetContractId, "--treasury", $Treasury, "--automation", $Automation)
Invoke-Contract $ids.receiptRegistry "initialize" @("--admin", $Admin, "--fee_token", $PayoutAssetContractId, "--treasury", $Treasury, "--automation", $Automation)

$report = [ordered]@{
  schemaVersion = "2.0.0"
  project = "CoverFi"
  network = $Network
  generatedAt = (Get-Date).ToString("o")
  dryRun = [bool]$DryRun
  sourceAccount = $SourceAccount
  admin = $Admin
  assets = [ordered]@{
    protectedPrincipal = [ordered]@{ symbol = "XLM"; tokenContract = $ProtectedAssetContractId }
    premiumAndPayout = [ordered]@{ symbol = "CFTUSD"; tokenContract = $PayoutAssetContractId; disclaimer = "Worthless Stellar Testnet asset. Not Circle-issued USDC." }
  }
  contracts = $ids
  compliance = [ordered]@{
    highValueVerificationThreshold = $HighValueVerificationThreshold
    verifierHash = $ZkVerifierHash
    kindHash = $ZkKindHash
    circuits = @(
      [ordered]@{ id = "coverfi.profile.email.verified.v1"; hash = "ebd8a6b268bff37dd937e243858a2706e3c8675fde4d6d75c7a195bfb1f4e967" },
      [ordered]@{ id = "coverfi.profile.mfa.enabled.v1"; hash = "d3d3bd01738ac64cc0ad02a074bb24db188df425d1f586af4e7cce50506696eb" },
      [ordered]@{ id = "coverfi.receipt_ownership.v0"; hash = "125f79a1790bf33732d22f3e73343452ea49699c30e6ee37077a36ba77c82eed" },
      [ordered]@{ id = "coverfi.partner_kyb.status.v0"; hash = "90532346634f39a224681334fcad836651adb4309bb66c86ee002ef9606f869f" },
      [ordered]@{ id = "coverfi.user_kyc.status.v0"; hash = "0084e5d60fe2404512e3ecd64fc304b85e017388534c31ee52e9b00c8cf91a3e" }
    )
  }
  wasmHashes = $hashes
  smokeTestedAt = $null
  smokeTest = "Run quote_position, create_position, settle_position, claim_payout, withdraw_principal, reserve withdrawal, username lease, and receipt anchor before publishing IDs."
}

$report | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 -Path $ReportPath
Write-Host "Wrote deployment report: $ReportPath"
