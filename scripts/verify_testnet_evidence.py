"""Read-only verification of published pilot WASM hashes and transaction receipts.

Requires Stellar CLI on PATH. No signing key, deployment or transaction submission.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
RPC = "https://soroban-testnet.stellar.org"
PASSPHRASE = "Test SDF Network ; September 2015"


def transaction(hash_value):
    request = urllib.request.Request(RPC, data=json.dumps({
        "jsonrpc": "2.0", "id": 1, "method": "getTransaction", "params": {"hash": hash_value},
    }).encode(), headers={"Content-Type": "application/json", "User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(request, timeout=30) as response:
        result = json.load(response)
    if "error" in result:
        raise RuntimeError(result["error"])
    return result["result"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stellar", default="stellar")
    options = parser.parse_args()
    deployment = json.loads((ROOT / "artifacts/pilot-testnet-deployment.json").read_text())
    smoke = json.loads((ROOT / "artifacts/pilot-testnet-smoke.json").read_text())
    if deployment["network"] != "testnet" or smoke["network"] != "testnet":
        raise ValueError("Only testnet evidence is accepted")
    results = {"network": "testnet", "wasmHashes": {}, "transactions": []}
    for name, contract_id in deployment["contracts"].items():
        value = subprocess.run([
            options.stellar, "contract", "info", "hash", "--id", contract_id,
            "--rpc-url", RPC, "--network-passphrase", PASSPHRASE,
        ], check=True, capture_output=True, text=True, encoding="utf-8").stdout.strip()
        if value != deployment["wasmSha256"][name]:
            raise ValueError(f"WASM hash mismatch: {name}")
        results["wasmHashes"][name] = value
    hashes = {h for row in deployment["transactionReceipts"] for h in row["hashes"]}
    hashes.update(h for row in smoke["steps"] for h in row["transactionHashes"])
    ordered_hashes = sorted(hashes)
    with ThreadPoolExecutor(max_workers=3) as workers:
        transactions = list(workers.map(transaction, ordered_hashes))
    for hash_value, result in zip(ordered_hashes, transactions):
        if result["status"] != "SUCCESS":
            raise ValueError(f"Transaction not currently verifiable as successful: {hash_value}: {result['status']}")
        results["transactions"].append({"hash": hash_value, "status": result["status"], "ledger": result.get("ledger")})
    results["verifiedAt"] = datetime.now(timezone.utc).isoformat()
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
