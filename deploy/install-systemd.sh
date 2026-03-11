#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="/opt/polymarket_gateway"
BIN_NAME="polymarket_gateway"
ENV_FILE="/etc/polymarket_gateway.env"
UNIT_FILE="/etc/systemd/system/polymarket-gateway.service"

cd "$ROOT_DIR"

if [[ ! -f .env ]]; then
  echo ".env is missing. Create it from .env.example before deploying." >&2
  exit 1
fi

cargo build --release

sudo install -d "$APP_DIR"
sudo install -m 755 "target/release/$BIN_NAME" "$APP_DIR/$BIN_NAME"
sudo install -m 640 .env "$ENV_FILE"
sudo install -m 644 deploy/polymarket-gateway.service "$UNIT_FILE"
sudo systemctl daemon-reload
sudo systemctl enable --now polymarket-gateway
sudo systemctl restart polymarket-gateway
sudo systemctl status polymarket-gateway --no-pager
