# polymarket_gateway

Rust VPS gateway for a thin Polymarket mobile client plus a separate operator dashboard. It ingests BTC 5-minute market data and reference BTC price feeds, exposes client and dashboard WebSocket APIs, enforces session auth and basic replay/risk checks, and wraps Polymarket execution behind VPS-side credentials.

For Polymarket credentials, v1 only requires each developer user's wallet private key. The gateway derives or creates the necessary API key material automatically at startup.

## What is included

- Axum server with `/ws`, `/healthz`, `/readyz`, dashboard APIs, and dashboard WebSocket streams
- Signed-token or configured-token WebSocket auth for two developer users
- Normalized event bus over Tokio `broadcast` plus live snapshot fanout with `watch`
- BTC 5-minute market scheduler with Gamma discovery and rollover events
- Feed adapters for Polymarket CLOB market data, Polymarket RTDS, Binance, Coinbase, Kraken, OKX, and Bitstamp
- Polymarket REST execution client for high-level trade and account commands
- In-memory observability layer for feed freshness, reconnect counts, price history, downstream throughput, alerts, and audit trail
- Public read-only operator dashboard in `dashboard/`
- Separate mobile-first trade PWA in `trade_app/`
- JWT-protected admin mode with runtime kill switch control
- In-memory state cache only, no database
- Deployment examples for `systemd` and Caddy

## Local run

```bash
cp .env.example .env
./scripts/run-local.sh
```

The service listens on `127.0.0.1:8080` by default.

Run the dashboard locally:

```bash
cd dashboard
cp .env.example .env
npm install
npm run dev
```

The dashboard expects the gateway API at `http://localhost:8080` by default and serves on `http://localhost:4173`.

Run the trade PWA locally:

```bash
cd trade_app
cp .env.example .env
npm install
npm run dev
```

The trade app serves on `http://localhost:4174` and expects the gateway at the same host by default.

## Dashboard API

Public:

- `GET /api/public/status`
- `GET /api/public/feeds`
- `GET /api/public/market`
- `GET /api/public/streaming`
- `GET /api/public/alerts`
- `GET /ws/dashboard/public`

Trade:

- `GET /api/trade/session`
- `POST /api/trade/refresh`
- `GET /ws/trade`

Admin:

- `POST /api/admin/login`
- `GET /api/admin/me`
- `GET /api/admin/status`
- `GET /api/admin/sessions`
- `GET /api/admin/audit`
- `POST /api/admin/controls/kill-switch`
- `POST /api/admin/controls/reload-config` (stubbed with explicit v1 error)
- `POST /api/admin/controls/reconnect-feed` (stubbed with explicit v1 error)
- `GET /ws/dashboard/admin?token=<jwt>`

Key observability fields now exposed include:

- upstream connection state
- last message age / staleness
- reconnect counts and recent disconnects
- per-feed update rate
- short rolling price history for charting
- downstream client/session count
- outbound messages/sec and bytes/sec
- dropped outbound messages and auth failures
- command age/latency approximation from gateway ingress
- derived incident/alert list
- operator audit trail

## WebSocket protocol

First message:

```json
{"type":"auth","token":"<token>"}
```

The same user token flow is used by the trade PWA. Tokens map to backend-configured user contexts like `dev_user1` and `dev_user2`, and the backend keeps the Polymarket private keys server-side.

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

Build the dashboard container:

```bash
cd dashboard
npm install
npm run build
docker build -t pm_gw_dashboard .
```

Build the trade app container:

```bash
cd trade_app
npm install
npm run build
docker build -t pm_gw_trade .
```

Run the dashboard container with Compose:

```bash
docker compose -f deploy/docker-compose.dashboard.yml up -d --build
```

Reverse proxy example:

- route `/trade*` to the trade app container
- route `/api/*`, `/ws`, `/ws/dashboard/*`, `/healthz`, and `/readyz` to `pm_gw`
- route all other paths to the dashboard container
- see [deploy/Caddyfile.example](/home/hzyshd/code/pm_gw/deploy/Caddyfile.example)

## Notes

- `ADMIN_PASSWORD_HASH` is preferred in production. For local/dev, `ADMIN_PASSWORD` is accepted and converted into an Argon2 hash at startup.
- `POLY_CLOB_USER_WS_URL` is exposed in config, but v1 still keeps account-state authority on the REST sync path while user-stream parsing is used for realtime operator visibility and event fanout.
- The execution module keeps Polymarket signing and REST payload generation on the VPS. If the upstream auth contract changes, update [src/execution/polymarket.rs](/home/hzyshd/code/pm_gw/src/execution/polymarket.rs).
