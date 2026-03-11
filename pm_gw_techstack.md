**VPS-side tech stack only**

### Core language/runtime

* **Rust**
* **Tokio** async runtime

### Network server layer

* **Axum** for the VPS API/WebSocket server
* **WebSocket** as the primary mobile/VPS transport
* **HTTPS** for health/admin endpoints if needed

### Exchange connectivity

* **tokio-tungstenite** for outbound WebSocket connections
* **reqwest** for REST calls to Polymarket and related services

### Serialization / protocol

* **serde**
* **serde_json**
* JSON messages for v1 between:

  * mobile app ↔ VPS
  * VPS ↔ internal command/state pipeline

### Internal backend architecture

* **Typed command gateway**
* **Typed event/state broadcaster**
* **Tokio channels**

  * `mpsc` for command pipeline
  * `broadcast` for fanout updates
  * `watch` for latest shared state snapshots where useful

### Shared state / concurrency

* Prefer **message passing first**
* Use `Arc`
* Use `RwLock` / `Mutex` only where actually needed
* Keep lock scope minimal

### Data ingestion design

* VPS maintains live connections to:

  * Polymarket market data
  * account/order/fill data
* Data is primarily **streamed through**, not stored
* Keep an in-memory rolling state only for:

  * latest price
  * target price
  * latest order state
  * latest account state
  * connection/session state

### Command relay design

* Mobile sends **high-level commands**
* VPS performs:

  * auth check
  * schema validation
  * risk checks
  * translation to Polymarket API calls
  * execution
  * ack/result broadcast back to client

### Auth / security

* **Signed session tokens** between mobile clients and VPS
* Polymarket credentials and signing logic stay **only on VPS**
* Never place exchange secrets in client apps
* Per-session auth on WebSocket connect
* Optional device/session whitelist

### Risk / safety layer

* Max order size checks
* Max notional checks
* Market whitelist
* Cooldown / rate limit
* Kill switch
* Duplicate / replay protection with client command IDs and timestamps

### Observability / debugging

* **tracing**
* **tracing-subscriber**
* Structured logs
* Command audit logs in log output
* Connection lifecycle logs
* Error classification logs

### Error handling

* **thiserror**
* **anyhow**

### IDs / time

* **uuid**
* **time**

### Config

* **dotenvy**
* environment-variable driven config
* separate config for:

  * VPS host/port
  * Polymarket endpoints
  * auth secrets
  * rate limits
  * risk limits

### TLS / edge

* **Caddy** in front for:

  * TLS termination
  * domain handling
  * reverse proxy to Rust service

### Process management

* **systemd**

### Build / deploy

* `cargo build --release`
* release binary on Ubuntu VPS
* run as systemd service

### Persistence

* **No database in v1**
* In-memory state only
* Add DB later only if needed for:

  * historical audit logs
  * replay
  * queued commands
  * persistent trade history
  * analytics

### Health / ops endpoints

* `/healthz`
* `/readyz`

---

## Practical architecture

```text
Native Android App / PWA
        ↕
   WebSocket / HTTPS
        ↕
      Caddy
        ↕
 Rust VPS service (Axum)
        ↕
 ┌─────────────────────────────────────┐
 │ command handlers                    │
 │ validation                          │
 │ auth/session layer                  │
 │ risk controls                       │
 │ market data relay                   │
 │ state broadcaster                   │
 └─────────────────────────────────────┘
        ↕
 tokio-tungstenite / reqwest
        ↕
     Polymarket
```

---

## What the VPS service is responsible for

* maintaining Polymarket-facing connections
* ingesting live market/account data
* relaying live data to clients
* receiving client commands
* validating and executing commands
* centralizing secrets, auth, and risk logic
* presenting a stripped-down fast interface layer for your apps

---

## v1 crate/tool list

* `axum`
* `tokio`
* `tokio-tungstenite`
* `reqwest`
* `serde`
* `serde_json`
* `tracing`
* `tracing-subscriber`
* `thiserror`
* `anyhow`
* `uuid`
* `time`
* `dotenvy`

Infra:

* **Ubuntu 24 VPS**
* **systemd**
* **Caddy**

---

## Recommended v1 design principle

* **mobile clients = thin UI clients**
* **VPS = smart trading gateway**
* **no DB**
* **no heavy internal complexity**
* **stream live state, execute commands, enforce safety**

If you want, next step should be the exact **Rust project folder layout and module structure for v1**.
