#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

if [[ ! -f .env ]]; then
  echo ".env is missing. Copy .env.example to .env and fill in your values." >&2
  exit 1
fi

cargo run
