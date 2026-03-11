# polymarket_gateway

Rust VPS gateway for a thin Polymarket mobile client. It ingests BTC 5-minute market data and reference BTC price feeds, exposes a WebSocket API for clients, enforces session auth and basic replay/risk checks, and wraps Polymarket execution behind VPS-side credentials.

For Polymarket credentials, v1 only requires each developer user's wallet private key. The gateway derives or creates the necessary API key material automatically at startup.

## What is included

- Axum server with `/ws`, `/healthz`, and `/readyz`
- Signed-token or configured-token WebSocket auth for two developer users
- Normalized event bus over Tokio `broadcast` plus live snapshot fanout with `watch`
- BTC 5-minute market scheduler with Gamma discovery and rollover events
- Feed adapters for Polymarket CLOB market data, Polymarket RTDS, Binance, Coinbase, Kraken, OKX, and Bitstamp
- Polymarket REST execution client for high-level trade and account commands
- In-memory state cache only, no database
- Deployment examples for `systemd` and Caddy

## Local run

```bash
cp .env.example .env
./scripts/run-local.sh
```

The service listens on `127.0.0.1:8080` by default.

## WebSocket protocol

First message:

```json
{"type":"auth","token":"<token>"}
```

Example trade command:

```json
{
  "type":"place_limit_order",
  "command_id":"3f928d3a-6191-4ea9-b78d-f9f07f7c7b43",
  "timestamp":"2026-03-10T18:00:00Z",
  "side":"buy",
  "outcome":"up",
  "size_type":"shares",
  "size":10.0,
  "price":0.54
}
```

## Deployment

Build:

```bash
cargo build --release
```

Install, enable, and start the systemd service:

```bash
./deploy/install-systemd.sh
```

Restart an existing deployment:

```bash
./deploy/restart-systemd.sh
```

## Notes

- `POLY_CLOB_USER_WS_URL` is exposed in config, but v1 keeps authenticated user state on a simpler REST sync path so the service remains compileable and operable without a full exchange-specific user-stream parser.
- The execution module keeps Polymarket signing and REST payload generation on the VPS. If the upstream auth contract changes, update [src/execution/polymarket.rs](/home/hzyshd/code/pm_gw/src/execution/polymarket.rs).
