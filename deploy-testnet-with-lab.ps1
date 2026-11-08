param(
  [ValidateSet("testnet")][string]$Network = "testnet",
  [Parameter(Mandatory=$true)][string]$Config,
  [switch]$Execute
)
$ErrorActionPreference = "Stop"
$pilotScript = Join-Path $PSScriptRoot "scripts/deploy_testnet.py"
$pilotArguments = @($pilotScript, "--config", $Config)
if ($Execute) { $pilotArguments += "--execute" }
& python @pilotArguments
exit $LASTEXITCODE
