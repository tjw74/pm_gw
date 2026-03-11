use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{env, time::Duration};
use tracing::info;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub log_level: String,
    pub auth_secret: String,
    pub poly_gamma_base_url: String,
    pub poly_clob_base_url: String,
    pub poly_data_api_base_url: String,
    pub poly_clob_market_ws_url: String,
    pub poly_clob_user_ws_url: String,
    pub poly_rtds_ws_url: String,
    pub enable_binance_feed: bool,
    pub enable_coinbase_feed: bool,
    pub enable_kraken_feed: bool,
    pub enable_okx_feed: bool,
    pub enable_bitstamp_feed: bool,
    pub btc_market_seed_slug: String,
    pub market_rollover_enabled: bool,
    pub command_max_age_secs: i64,
    pub enable_kill_switch: bool,
    pub optional_max_order_size: Option<f64>,
    pub optional_max_notional: Option<f64>,
    pub optional_command_rate_limit_per_sec: Option<u32>,
    pub dev_users: Vec<DevUserConfig>,
}

#[derive(Clone, Debug)]
pub struct DevUserConfig {
    pub id: String,
    pub token: String,
    pub private_key: String,
}

#[derive(Debug, Serialize)]
struct ConfigSummary<'a> {
    server_host: &'a str,
    server_port: u16,
    log_level: &'a str,
    market_rollover_enabled: bool,
    btc_market_seed_slug: &'a str,
    feeds: FeedSummary,
    dev_users: usize,
}

#[derive(Debug, Serialize)]
struct FeedSummary {
    binance: bool,
    coinbase: bool,
    kraken: bool,
    okx: bool,
    bitstamp: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let server_host = env_string("SERVER_HOST", "127.0.0.1");
        let server_port = env_u16("SERVER_PORT", 8080)?;
        let log_level = env_string("LOG_LEVEL", "info");
        let auth_secret = env_required("AUTH_SECRET")?;
        let poly_gamma_base_url =
            env_string("POLY_GAMMA_BASE_URL", "https://gamma-api.polymarket.com");
        let poly_clob_base_url = env_string("POLY_CLOB_BASE_URL", "https://clob.polymarket.com");
        let poly_data_api_base_url =
            env_string("POLY_DATA_API_BASE_URL", "https://data-api.polymarket.com");
        let poly_clob_market_ws_url = env_string(
            "POLY_CLOB_MARKET_WS_URL",
            "wss://ws-subscriptions-clob.polymarket.com/ws/market",
        );
        let poly_clob_user_ws_url = env_string(
            "POLY_CLOB_USER_WS_URL",
            "wss://ws-subscriptions-clob.polymarket.com/ws/user",
        );
        let poly_rtds_ws_url = env_string("POLY_RTDS_WS_URL", "wss://ws-live-data.polymarket.com");

        let dev_users = vec![load_dev_user("DEV_USER1")?, load_dev_user("DEV_USER2")?]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if dev_users.is_empty() {
            bail!("at least one developer user must be configured");
        }

        Ok(Self {
            server_host,
            server_port,
            log_level,
            auth_secret,
            poly_gamma_base_url,
            poly_clob_base_url,
            poly_data_api_base_url,
            poly_clob_market_ws_url,
            poly_clob_user_ws_url,
            poly_rtds_ws_url,
            enable_binance_feed: env_bool("ENABLE_BINANCE_FEED", true),
            enable_coinbase_feed: env_bool("ENABLE_COINBASE_FEED", true),
            enable_kraken_feed: env_bool("ENABLE_KRAKEN_FEED", true),
            enable_okx_feed: env_bool("ENABLE_OKX_FEED", true),
            enable_bitstamp_feed: env_bool("ENABLE_BITSTAMP_FEED", true),
            btc_market_seed_slug: env_string("BTC_MARKET_SEED_SLUG", "bitcoin-up-or-down"),
            market_rollover_enabled: env_bool("MARKET_ROLLOVER_ENABLED", true),
            command_max_age_secs: env_i64("COMMAND_MAX_AGE_SECS", 20)?,
            enable_kill_switch: env_bool("ENABLE_KILL_SWITCH", false),
            optional_max_order_size: env_opt_f64("OPTIONAL_MAX_ORDER_SIZE")?,
            optional_max_notional: env_opt_f64("OPTIONAL_MAX_NOTIONAL")?,
            optional_command_rate_limit_per_sec: env_opt_u32(
                "OPTIONAL_COMMAND_RATE_LIMIT_PER_SEC",
            )?,
            dev_users,
        })
    }

    pub fn command_max_age(&self) -> Duration {
        Duration::from_secs(self.command_max_age_secs.max(1) as u64)
    }

    pub fn log_summary(&self) {
        let summary = ConfigSummary {
            server_host: &self.server_host,
            server_port: self.server_port,
            log_level: &self.log_level,
            market_rollover_enabled: self.market_rollover_enabled,
            btc_market_seed_slug: &self.btc_market_seed_slug,
            feeds: FeedSummary {
                binance: self.enable_binance_feed,
                coinbase: self.enable_coinbase_feed,
                kraken: self.enable_kraken_feed,
                okx: self.enable_okx_feed,
                bitstamp: self.enable_bitstamp_feed,
            },
            dev_users: self.dev_users.len(),
        };
        info!(config = ?summary, "loaded configuration");
    }
}

fn load_dev_user(prefix: &str) -> Result<Option<DevUserConfig>> {
    let token_var = format!("{prefix}_TOKEN");
    let Some(token) = env::var(&token_var).ok().filter(|v| is_real_secret(v)) else {
        return Ok(None);
    };

    let private_key = env::var(format!("{prefix}_PRIVATE_KEY"))
        .ok()
        .filter(|v| is_real_secret(v))
        .with_context(|| format!("missing required env var {prefix}_PRIVATE_KEY"))?;
    Ok(Some(DevUserConfig {
        id: prefix.to_lowercase(),
        token,
        private_key,
    }))
}

fn is_real_secret(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.eq_ignore_ascii_case("replace_me")
        && !trimmed.starts_with("replace_with_")
}

fn env_required(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing required env var {name}"))
}

fn env_string(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .and_then(|v| match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn env_u16(name: &str, default: u16) -> Result<u16> {
    env::var(name)
        .ok()
        .map(|v| v.parse().with_context(|| format!("invalid u16 in {name}")))
        .transpose()?
        .unwrap_or(default)
        .pipe(Ok)
}

fn env_i64(name: &str, default: i64) -> Result<i64> {
    env::var(name)
        .ok()
        .map(|v| v.parse().with_context(|| format!("invalid i64 in {name}")))
        .transpose()?
        .unwrap_or(default)
        .pipe(Ok)
}

fn env_opt_f64(name: &str) -> Result<Option<f64>> {
    env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.parse().with_context(|| format!("invalid f64 in {name}")))
        .transpose()
}

fn env_opt_u32(name: &str) -> Result<Option<u32>> {
    env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.parse().with_context(|| format!("invalid u32 in {name}")))
        .transpose()
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}
