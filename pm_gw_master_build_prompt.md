You are an expert Rust backend engineer building a production-quality, low-latency Polymarket gateway on an Ubuntu 24 VPS.

Your task is to ONE-SHOT build the full backend system in this repo with minimal follow-up edits.

The system is a smart VPS-side gateway for:
1. ingesting Polymarket BTC 5-minute Up/Down market data
2. ingesting duplicate/reference BTC price feeds from Polymarket RTDS and major exchanges
3. streaming normalized live data to mobile clients over WebSocket
4. receiving mobile trade commands and relaying them to Polymarket
5. handling market-window rollover automatically for the active BTC 5m market
6. running as a systemd service behind Caddy

Important working style:
- Do not ask for approval.
- Make reasonable, production-safe choices.
- Prefer clean, modular code over clever code.
- Prefer correctness and maintainability without adding unnecessary complexity.
- Keep latency low, locking minimal, and message flow explicit.
- Before coding, print a short implementation plan.
- Then implement everything.
- After implementation, run cargo check and fix any compile errors.
- Also create deployment/config files and print run instructions.

==================================================
HIGH-LEVEL PRODUCT GOAL
==================================================

This service is the backend for a stripped-down Polymarket mobile trading app.

The VPS backend must:
- maintain Polymarket-facing market-data connections
- maintain Polymarket-facing authenticated user-data connections
- maintain execution capability for trading on Polymarket
- provide a normalized internal event bus
- stream live state to mobile clients over WebSocket
- accept high-level mobile commands for trading and order management
- enforce authentication, replay protection, and basic safety checks
- keep all live state in memory in v1
- avoid any database in v1

The mobile clients are thin clients.
The VPS is the smart trading gateway.

==================================================
TECH STACK (MANDATORY)
==================================================

Language/runtime:
- Rust
- Tokio

Crates/tools:
- axum
- tokio
- tokio-tungstenite
- reqwest
- serde
- serde_json
- tracing
- tracing-subscriber
- thiserror
- anyhow
- uuid
- time
- dotenvy

Infra assumptions:
- Ubuntu 24
- systemd
- Caddy in front for TLS termination and reverse proxy

==================================================
POLYMARKET / EXTERNAL API REQUIREMENTS
==================================================

Use the latest official documented endpoints and API formats from the providers.

Polymarket endpoint classes to support:
- Gamma API for market/event discovery and metadata
- CLOB market websocket for public market/orderbook/trade stream
- CLOB user websocket for authenticated user/order/fill/account stream
- CLOB REST for trading / execution / cancellations
- RTDS websocket for crypto reference prices

Also implement external BTC/USD or BTC/USDT price feed adapters for:
- Binance
- Coinbase
- Kraken
- OKX
- Bitstamp

Design the external feed layer so more exchanges can be added later with minimal changes.

Important:
- Do not hardcode stale endpoints if docs have changed.
- During implementation, verify the latest official docs and use the latest documented production endpoints and subscription formats.
- If an exchange feed requires symbol normalization, normalize internally.
- The backend must expose the source exchange/oracle in normalized events.

==================================================
CORE MARKET DESIGN
==================================================

The initial target is the Polymarket BTC 5-minute Up/Down market.

Important domain behavior:
- The active market changes every 5 minutes.
- The backend must maintain its own market-window tracker.
- Seed the system with a BTC 5m slug pattern / market discovery strategy.
- Use current time to determine the active 5-minute window.
- Resolve/fetch the correct active market metadata.
- Roll forward automatically when the market window expires.
- Prepare the design so additional BTC markets can be added later.

Implement a market scheduler / market clock module that:
- calculates the current 5-minute window
- discovers/resolves the current active Polymarket market
- loads metadata needed for subscription/trading
- detects rollover
- switches subscriptions cleanly to the new market
- publishes market_rollover events to clients

==================================================
ARCHITECTURE (MANDATORY)
==================================================

Implement this as one Rust service with clear modules.

Use this architecture:

mobile apps
    ↕ websocket/json
Axum server
    ↕ internal channels
gateway core
    ├ auth/session layer
    ├ command parser/router
    ├ validation/replay protection
    ├ execution engine
    ├ market scheduler
    ├ polymarket ingestion
    ├ external feed ingestion
    ├ state cache
    └ broadcaster
    ↕
Polymarket + external feeds

Concurrency rules:
- prefer message passing first
- use tokio channels
- use Arc only when needed
- use Mutex/RwLock only where unavoidable
- keep lock scope minimal
- avoid heavy shared mutable state
- use separate async tasks for feed adapters, command handling, and client broadcasting

Tokio channel guidance:
- mpsc for command pipeline
- broadcast for fanout updates
- watch for latest-state snapshots where useful

==================================================
NORMALIZED INTERNAL DATA MODEL
==================================================

Create a normalized event model so all feeds publish into the same internal bus.

At minimum implement normalized event types for:
- PriceTick
- OrderBookSnapshot
- OrderBookDelta
- TradePrint
- MarketStatus
- MarketRollover
- AccountUpdate
- PositionUpdate
- OrderUpdate
- FillUpdate
- Heartbeat
- ErrorEvent
- ConnectionStatus

Each normalized event should include:
- event_type
- source (polymarket_clob, polymarket_rtds_binance, polymarket_rtds_chainlink, binance, coinbase, kraken, okx, bitstamp, gateway)
- symbol / market slug / token identifiers when applicable
- timestamp
- payload fields relevant to that event type

For BTC reference pricing, normalize symbols internally so mobile clients receive one coherent shape.

==================================================
STATE MODEL (IN-MEMORY ONLY)
==================================================

Maintain in-memory rolling state for v1 only.

At minimum keep:
- active market identity
- current 5-minute window metadata
- latest Polymarket market price
- latest reference prices by source
- last trade per source
- current orderbook snapshot for active market
- latest account summary for each authenticated dev wallet
- active orders by wallet
- positions by wallet
- client sessions
- connection health/status per feed
- target price (UI overlay only, not trading logic)

Do NOT add a database in v1.

==================================================
MOBILE WEBSOCKET SERVER
==================================================

Build an Axum server with:
- /ws
- /healthz
- /readyz

The main client transport is WebSocket.
Use JSON messages for v1.

Behavior:
- client connects to /ws
- first client message must be auth
- reject unauthenticated clients
- after auth, client can subscribe and send commands
- server streams normalized events and execution results back to that client
- market data may be broadcast to all authenticated clients
- account/order/fill events must be scoped to the authenticated logical user

Use a clean connection/session abstraction.

Set TCP_NODELAY where applicable and avoid unnecessary buffering.

Implement heartbeat / keepalive behavior.

Implement backpressure handling:
- if a client falls behind, prefer dropping stale market-data messages and sending fresh state
- do not let one slow client block the system
- if needed, disconnect a pathological slow client cleanly

==================================================
CLIENT AUTH / SESSION MODEL
==================================================

For v1:
- use signed session tokens between mobile client and VPS
- token is provided in the first websocket message
- do not use query-string auth
- map each token to a logical user identity
- for v1, logical users are just two developer users / wallets
- later this can be replaced by a proper multi-user wallet/auth service

Auth message shape:
{
  "type": "auth",
  "token": "<token>"
}

Implement:
- signed token verification using AUTH_SECRET
- session creation
- user binding
- optional per-user/device whitelist structure in code, but keep it simple in v1

==================================================
TRADING / WALLET MODEL
==================================================

Important:
- In v1, support only developer-mode trading using two configured developer wallets.
- Private keys must live only on the VPS in environment variables.
- Do not put secrets in client code.
- The service chooses which wallet/account to use based on the authenticated session token.

Use env vars like:
- DEV_USER1_TOKEN
- DEV_USER1_PRIVATE_KEY
- DEV_USER1_POLY_API_KEY
- DEV_USER1_POLY_API_SECRET
- DEV_USER1_POLY_API_PASSPHRASE
- DEV_USER2_TOKEN
- DEV_USER2_PRIVATE_KEY
- DEV_USER2_POLY_API_KEY
- DEV_USER2_POLY_API_SECRET
- DEV_USER2_POLY_API_PASSPHRASE

Design the code so this can be replaced later by a real secure wallet/auth subsystem.

==================================================
COMMAND MODEL
==================================================

The mobile app should be able to do everything essential for interacting with BTC Up/Down markets like the Polymarket web UI.

Implement high-level command messages over WebSocket.

Required commands:
- ping
- subscribe_market
- unsubscribe_market
- place_limit_order
- place_market_order
- cancel_order
- cancel_all
- get_open_orders
- get_positions
- get_account_state
- set_target_price

Command rules:
- commands must include:
  - type
  - command_id (uuid string)
  - timestamp
- implement replay protection using command_id + timestamp
- reject stale or duplicate commands
- target price is UI-only, not used for execution logic

Support both size units:
- shares
- dollars

Support both trade intents:
- buy
- sell

Support both outcomes:
- up
- down

Example command shapes:
{
  "type": "place_limit_order",
  "command_id": "...",
  "timestamp": "...",
  "side": "buy",
  "outcome": "up",
  "size_type": "shares",
  "size": 10.0,
  "price": 0.54
}

{
  "type": "place_market_order",
  "command_id": "...",
  "timestamp": "...",
  "side": "sell",
  "outcome": "down",
  "size_type": "dollars",
  "size": 25.0
}

{
  "type": "cancel_order",
  "command_id": "...",
  "timestamp": "...",
  "order_id": "..."
}

==================================================
COMMAND PIPELINE
==================================================

Implement the pipeline as:

client ws
→ auth/session validation
→ command parser
→ schema validation
→ replay protection
→ command router
→ execution handler
→ result/ack/update events back to client

Behavior:
- immediately ack accepted commands structurally
- then send final execution result/update asynchronously
- include command_id in responses so mobile can correlate

Implement server response message types such as:
- auth_ok
- auth_error
- command_ack
- command_rejected
- order_update
- fill_update
- account_update
- position_update
- market_update
- market_rollover
- heartbeat
- error

==================================================
RISK / SAFETY LAYER
==================================================

For v1, keep risk checks minimal and configurable, since users are manually trading and do not want restrictive limits.

Implement config-driven checks but keep defaults permissive:
- optional max order size
- optional max notional
- optional market whitelist
- optional cooldown
- optional kill switch

Use env-driven config with the ability to disable these checks.
Do not hardcode aggressive limitations.

==================================================
POLYMARKET EXECUTION LAYER
==================================================

Implement a dedicated execution module that:
- encapsulates Polymarket authenticated REST interactions
- can place orders
- cancel orders
- cancel all
- fetch open orders / positions / account state if needed
- handles signing/auth requirements correctly
- keeps exchange-facing payload generation on the VPS only

Do not let clients send raw Polymarket API payloads.
Clients only send high-level intents.

If official auth flows require specific headers/signing steps, implement them according to the latest official docs.

==================================================
INGESTION LAYER
==================================================

Implement dedicated feed adapters/modules for:

Polymarket:
- gamma discovery adapter
- clob market websocket adapter
- clob user websocket adapter
- rtds websocket adapter

External BTC price feed adapters:
- Binance
- Coinbase
- Kraken
- OKX
- Bitstamp

Adapter requirements:
- connect
- subscribe
- parse
- normalize
- reconnect with backoff
- publish connection status
- publish heartbeats if applicable
- handle ping/pong rules correctly
- re-subscribe after reconnect
- surface errors cleanly

For Binance specifically, account for official websocket behavior like required pong handling and the 24-hour disconnect lifecycle.

Design adapters behind a common trait/interface where practical.

==================================================
PROJECT STRUCTURE
==================================================

Create a clean project layout like:

hz_pm_bot_gateway/
  Cargo.toml
  .env.example
  README.md
  deploy/
    polymarket-gateway.service
    Caddyfile.example
  src/
    main.rs
    config.rs
    error.rs
    types.rs
    auth.rs
    session.rs
    commands.rs
    responses.rs
    state.rs
    broadcast.rs
    market_scheduler.rs
    ws_server.rs
    health.rs
    execution/
      mod.rs
      polymarket.rs
    feeds/
      mod.rs
      traits.rs
      polymarket_gamma.rs
      polymarket_clob_market.rs
      polymarket_clob_user.rs
      polymarket_rtds.rs
      binance.rs
      coinbase.rs
      kraken.rs
      okx.rs
      bitstamp.rs
    models/
      mod.rs
      normalized.rs
      market.rs
      account.rs
      order.rs
    util/
      mod.rs
      time.rs
      ids.rs
      retry.rs
      websocket.rs

You may adjust filenames slightly if needed, but keep the structure modular and obvious.

==================================================
CONFIG
==================================================

Use dotenvy and env-based config.

Include at minimum:
- SERVER_HOST
- SERVER_PORT
- LOG_LEVEL
- AUTH_SECRET
- POLY_GAMMA_BASE_URL
- POLY_CLOB_BASE_URL
- POLY_CLOB_MARKET_WS_URL
- POLY_CLOB_USER_WS_URL
- POLY_RTDS_WS_URL
- ENABLE_BINANCE_FEED
- ENABLE_COINBASE_FEED
- ENABLE_KRAKEN_FEED
- ENABLE_OKX_FEED
- ENABLE_BITSTAMP_FEED
- BTC_MARKET_SEED_SLUG
- MARKET_ROLLOVER_ENABLED
- COMMAND_MAX_AGE_SECS
- ENABLE_KILL_SWITCH
- OPTIONAL_MAX_ORDER_SIZE
- OPTIONAL_MAX_NOTIONAL
- OPTIONAL_COMMAND_RATE_LIMIT_PER_SEC
- DEV_USER1_TOKEN
- DEV_USER1_PRIVATE_KEY
- DEV_USER1_POLY_API_KEY
- DEV_USER1_POLY_API_SECRET
- DEV_USER1_POLY_API_PASSPHRASE
- DEV_USER2_TOKEN
- DEV_USER2_PRIVATE_KEY
- DEV_USER2_POLY_API_KEY
- DEV_USER2_POLY_API_SECRET
- DEV_USER2_POLY_API_PASSPHRASE

Populate .env.example with placeholders and comments.

==================================================
OBSERVABILITY
==================================================

Use tracing + tracing-subscriber.

Log:
- startup config summary (without secrets)
- feed connections/disconnections
- reconnect attempts
- market rollover
- client auth/connect/disconnect
- command intake and outcomes
- execution success/failure
- parsing failures
- readiness state

Keep logs structured and useful.

==================================================
HEALTH / READINESS
==================================================

/healthz
- process alive

/readyz
- server ready
- critical core tasks started
- active market resolved
- at least Polymarket core ingestion initialized

Return simple JSON.

==================================================
DEPLOYMENT FILES
==================================================

Create:
1. systemd service file
2. Caddyfile example
3. README with build/run/deploy steps

Assume:
- Rust service listens on 127.0.0.1:8080
- Caddy handles TLS and reverse proxies /ws, /healthz, /readyz
- public host example can be hzyshd.io

systemd service should:
- run the compiled release binary
- restart on failure
- load env file from a predictable path
- run safely in production

==================================================
IMPLEMENTATION PRIORITIES
==================================================

Priority order:
1. compileable Rust project
2. config loading
3. normalized models
4. market scheduler
5. Polymarket ingestion
6. external feed ingestion
7. WebSocket mobile server
8. command pipeline
9. Polymarket execution layer
10. deploy files and docs

If some specific live trading/auth edge is hard to fully complete from docs alone:
- still create the correct module structure and interfaces
- implement as much real behavior as possible
- leave very small, clearly marked TODOs only where absolutely unavoidable
- do NOT leave the project as a vague scaffold

==================================================
QUALITY BAR
==================================================

The final result should be:
- real code, not pseudocode
- compileable with cargo check
- well-structured and modular
- ready for iterative extension
- strong enough that a follow-up pass is about refinement, not basic construction

==================================================
FINAL STEPS
==================================================

After coding:
1. run cargo fmt if needed
2. run cargo check
3. fix errors
4. print a concise summary of what was built
5. print exact commands to run the service locally
6. print exact commands to install/deploy the systemd unit
7. print any remaining TODOs, but only if truly necessary
