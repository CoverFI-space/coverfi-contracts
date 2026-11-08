"""Prepare or deploy the four-contract pilot; dry-run unless --execute is set.

Only public testnet RPC/passphrase are supported. Input contains public addresses
and local CLI identity aliases, never secret keys. Does not publish active IDs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ("oracle_adapter", "reserve_vault", "protected_balance_vault", "protection_engine")
NETWORK = ["--rpc-url", "https://soroban-testnet.stellar.org",
           "--network-passphrase", "Test SDF Network ; September 2015"]
TRANSACTION_RECEIPTS = []
CLI_CONFIG = ROOT / ".stellar"


def validate(config):
    if config.get("network") != "testnet":
        raise ValueError("Only testnet is supported")
    for field in ("admin", "treasury", "publisher1", "publisher2", "publisher3"):
        if not re.fullmatch(r"G[A-Z2-7]{55}", config.get(field, "")):
            raise ValueError(f"{field} must be a public Stellar account address")
    for field in ("protected_asset", "payout_asset"):
        if not re.fullmatch(r"C[A-Z2-7]{55}", config.get(field, "")):
            raise ValueError(f"{field} must be a contract address")
    if config.get("oracle_source") is not None and not re.fullmatch(r"C[A-Z2-7]{55}", config["oracle_source"]):
        raise ValueError("oracle_source must be null or a contract address")
    if config["protected_asset"] == config["payout_asset"]:
        raise ValueError("Protected and payout assets must differ")
    if len({config[f"publisher{i}"] for i in (1, 2, 3)}) != 3:
        raise ValueError("Three distinct publisher accounts are required")
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,40}", config.get("source_account", "")):
        raise ValueError("source_account must be a local CLI identity alias, not a key")
    if config.get("source_asset") != "USDC":
        raise ValueError("The pilot protects only the configured USDC test asset")
    if config.get("source_decimals") != 8:
        raise ValueError("Pilot feed observations must use eight decimals")
    # No configurable RPC or passphrase: an alias cannot redirect this runner.
    if "rpc_url" in config or "network_passphrase" in config:
        raise ValueError("Custom network endpoints are not supported")


def run(args):
    operation = args[1:3]
    if Path(args[0]).stem == "stellar":
        args = [args[0], "--config-dir", str(CLI_CONFIG), *args[1:]]
    result = subprocess.run(args, cwd=ROOT, text=True,
                            encoding="utf-8", errors="replace",
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    sys.stderr.write(result.stderr)
    result.check_returncode()
    if Path(args[0]).stem == "stellar":
        hashes = sorted(set(re.findall(r"/tx/([a-f0-9]{64})", result.stderr)))
        if hashes:
            TRANSACTION_RECEIPTS.append({"command": operation, "hashes": hashes,
                                         "result": result.stdout.strip()})
    return result.stdout.strip()


def main():
    global CLI_CONFIG
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target")
    parser.add_argument("--stellar-config-dir", type=Path, default=ROOT / ".stellar")
    options = parser.parse_args()
    CLI_CONFIG = options.stellar_config_dir.resolve()
    config = json.loads(options.config.read_text(encoding="utf-8-sig"))
    validate(config)
    transcript = []

    def invoke(contract, method, **kwargs):
        cmd = ["stellar", "contract", "invoke", "--id", contract,
               "--source-account", config["source_account"], *NETWORK,
               "--send=yes", "--", method]
        for name, value in kwargs.items():
            encoded = "null" if value is None else str(value).lower() if isinstance(value, bool) else str(value)
            cmd += [f"--{name}", encoded]
        if options.execute:
            transcript.append({"contract": contract, "method": method, "cliOutput": run(cmd)})
            journal("initialization_in_progress")
        else:
            print(json.dumps(cmd))

    build = ["cargo", "build", "--locked", "--release", "--target", "wasm32v1-none",
             "--target-dir", str(options.target_dir.resolve())]
    for package in PACKAGES:
        build += ["-p", package]
    ids = {package: f"<pending:{package}>" for package in PACKAGES}
    hashes = {}
    commit = None

    def journal(status):
        # Preserve successful IDs if a later deploy/init transaction fails.
        output = ROOT / "artifacts/pilot-testnet-deployment.generated.json"
        output.parent.mkdir(exist_ok=True)
        output.write_text(json.dumps({
            "network": "testnet", "status": status, "sourceCommit": commit,
            "contracts": ids, "wasmSha256": hashes, "configuration": config,
            "invocations": transcript, "smokeTestedAt": None,
            "transactionReceipts": TRANSACTION_RECEIPTS,
        }, indent=2) + "\n", encoding="utf-8")

    if options.execute:
        if run(["git", "status", "--porcelain"]):
            raise ValueError("Commit source changes before executing a release")
        commit = run(["git", "rev-parse", "HEAD"])
        run(build)
        for package in PACKAGES:
            wasm = options.target_dir / "wasm32v1-none/release" / f"{package}.wasm"
            hashes[package] = hashlib.sha256(wasm.read_bytes()).hexdigest()
            result = run(["stellar", "contract", "deploy", "--wasm", str(wasm), "--optimize=false",
                          "--source-account", config["source_account"], *NETWORK])
            matches = re.findall(r"\bC[A-Z2-7]{55}\b", result)
            if len(set(matches)) != 1:
                raise ValueError(f"Could not unambiguously identify deployed {package}")
            ids[package] = matches[0]
            journal("deployment_in_progress")
    else:
        print(json.dumps(build))
        print("DRY RUN: no build, signing, deployment, funding or registry write")

    invoke(ids["oracle_adapter"], "initialize", admin=config["admin"],
           source=config["oracle_source"], source_asset="USDC", source_decimals=8,
           max_age_seconds=3600, max_deviation_bps=10000,
           protected_asset=config["protected_asset"], payout_asset=config["payout_asset"])
    invoke(ids["oracle_adapter"], "configure_publishers", admin=config["admin"],
           publisher1=config["publisher1"], publisher2=config["publisher2"],
           publisher3=config["publisher3"], enabled=True)
    invoke(ids["reserve_vault"], "initialize", admin=config["admin"], treasury=config["treasury"])
    invoke(ids["protected_balance_vault"], "initialize", admin=config["admin"], engine=ids["protection_engine"])
    invoke(ids["protection_engine"], "initialize", admin=config["admin"],
           protected_balance_vault=ids["protected_balance_vault"], reserve_vault=ids["reserve_vault"],
           oracle_adapter=ids["oracle_adapter"], protected_asset=config["protected_asset"],
           payout_asset=config["payout_asset"], max_payout_bps=1000, max_oracle_age_seconds=3600)
    invoke(ids["reserve_vault"], "configure_protocol", admin=config["admin"],
           engine=ids["protection_engine"], oracle=ids["oracle_adapter"])

    if options.execute:
        report = {
            "network": "testnet", "status": "initialized_not_smoke_verified",
            "sourceCommit": commit, "generatedAt": datetime.now(timezone.utc).isoformat(),
            "contracts": ids, "wasmSha256": hashes, "configuration": config,
            "reserveFunded": False, "smokeTestedAt": None, "invocations": transcript,
            "transactionReceipts": TRANSACTION_RECEIPTS,
            "warning": "No oracle observations or liquidity are seeded by this runner. "
                       "Publish funded balances and lifecycle transactions before activating IDs. "
                       "All assets are testnet only. No mainnet deployment is performed.",
        }
        output = ROOT / "artifacts/pilot-testnet-deployment.generated.json"
        output.parent.mkdir(exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {output}; the public registry was not modified.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from error
