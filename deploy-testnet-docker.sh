#!/usr/bin/env bash
set -euo pipefail
if [[ "${NETWORK:-testnet}" != "testnet" ]]; then
  echo "Only public testnet is supported." >&2
  exit 2
fi
: "${PILOT_CONFIG:?Set PILOT_CONFIG to the public-address JSON configuration path}"
exec python3 /workspace/scripts/deploy_testnet.py --config "$PILOT_CONFIG" "$@"
