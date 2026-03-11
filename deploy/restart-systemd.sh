#!/usr/bin/env bash
set -euo pipefail

sudo systemctl restart polymarket-gateway
sudo systemctl status polymarket-gateway --no-pager
