use std::sync::Arc;

use serde::{Deserialize, Deserializer};
use time::{Duration, OffsetDateTime};
use tracing::{error, info, warn};

use crate::{
    error::GatewayError,
    models::market::{ActiveMarket, MarketWindow},
    state::AppState,
};

#[derive(Debug, Deserialize)]
struct GammaMarket {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "conditionId", alias = "condition_id")]
    condition_id: Option<String>,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    question: Option<String>,
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    active: bool,
    #[serde(
        default,
        rename = "clobTokenIds",
        alias = "clob_token_ids",
        deserialize_with = "deserialize_token_ids"
    )]
    clob_token_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GammaResponse {
    List(Vec<GammaMarket>),
    Single(GammaMarket),
}

#[derive(Debug, Deserialize)]
struct GammaEvent {
    #[serde(default)]
    markets: Vec<GammaMarket>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GammaEventResponse {
    List(Vec<GammaEvent>),
    Single(GammaEvent),
}

pub async fn spawn_market_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        if let Err(err) = scheduler_loop(state).await {
            error!(?err, "market scheduler exited");
        }
    });
}

async fn scheduler_loop(state: Arc<AppState>) -> anyhow::Result<()> {
    loop {
        match resolve_active_market(&state).await {
            Ok(market) => {
                state.record_feed_message("market_scheduler", None).await;
                let changed = state.active_market_slug().await.as_deref() != Some(&market.slug);
                if changed {
                    info!(slug = %market.slug, "resolved active BTC 5m market");
                    state.set_active_market(market.clone()).await;
                }
                state
                    .record_connection(
                        "market_scheduler",
                        crate::types::ConnectionState::Connected,
                        None,
                    )
                    .await;
            }
            Err(err) => {
                warn!(?err, "failed to resolve active market");
                state
                    .record_connection(
                        "market_scheduler",
                        crate::types::ConnectionState::Degraded,
                        Some(err.to_string()),
                    )
                    .await;
            }
        }

        let now = OffsetDateTime::now_utc();
        let next = current_window(now).window_end + Duration::seconds(2);
        let sleep_for = (next - now).max(Duration::seconds(5));
        tokio::time::sleep(std::time::Duration::from_secs(
            sleep_for.whole_seconds() as u64
        ))
        .await;
    }
}

pub fn current_window(now: OffsetDateTime) -> MarketWindow {
    let epoch = now.unix_timestamp();
    let start = epoch - (epoch.rem_euclid(300));
    let window_start = OffsetDateTime::from_unix_timestamp(start).unwrap_or(now);
    let window_end = window_start + Duration::minutes(5);
    MarketWindow {
        window_start,
        window_end,
    }
}

async fn resolve_active_market(state: &AppState) -> Result<ActiveMarket, GatewayError> {
    let now = OffsetDateTime::now_utc();
    let window = current_window(now);
    let client = reqwest::Client::new();
    let seed = state.config.btc_market_seed_slug.replace(' ', "-");
    let expected_suffix = window.window_start.unix_timestamp().to_string();
    let family_candidates = market_family_candidates(&seed);
    let mut markets = Vec::new();

    for family in &family_candidates {
        let exact_slug = format!("{family}-{}", window.window_start.unix_timestamp());
        markets = fetch_gamma_markets(
            &client,
            &state.config.poly_gamma_base_url,
            &format!("/markets/slug/{exact_slug}"),
        )
        .await
        .unwrap_or_default();
        if !markets.is_empty() {
            break;
        }

        markets = fetch_gamma_event_markets(
            &client,
            &state.config.poly_gamma_base_url,
            &format!("/events/slug/{exact_slug}"),
        )
        .await
        .unwrap_or_default();
        if !markets.is_empty() {
            break;
        }
    }

    if markets.is_empty() {
        for family in &family_candidates {
            markets = fetch_gamma_markets(
                &client,
                &state.config.poly_gamma_base_url,
                &format!("/markets?slug={family}"),
            )
            .await
            .unwrap_or_default();
            if !markets.is_empty() {
                break;
            }

            markets = fetch_gamma_event_markets(
                &client,
                &state.config.poly_gamma_base_url,
                &format!("/events?slug={family}"),
            )
            .await
            .unwrap_or_default();
            if !markets.is_empty() {
                break;
            }
        }
    }

    if markets.is_empty() {
        for family in &family_candidates {
            markets = fetch_gamma_markets(
                &client,
                &state.config.poly_gamma_base_url,
                &format!("/markets?search={family}&limit=100"),
            )
            .await
            .unwrap_or_default();
            if !markets.is_empty() {
                break;
            }
        }
    }

    let best = markets
        .into_iter()
        .filter(|market| market.active && !market.closed)
        .max_by_key(|market| market_rank(market, &seed, &expected_suffix))
        .filter(|market| market_rank(market, &seed, &expected_suffix) > 0)
        .or_else(|| {
            let expected = format!("{}-{}", seed, window.window_start.unix_timestamp());
            Some(GammaMarket {
                id: expected.clone(),
                condition_id: None,
                slug: expected,
                question: Some("Derived BTC 5m market".to_string()),
                closed: false,
                active: true,
                clob_token_ids: vec![],
            })
        })
        .ok_or_else(|| GatewayError::Upstream("no BTC 5m market found".to_string()))?;

    Ok(ActiveMarket {
        market_id: best.id,
        slug: best.slug,
        condition_id: best.condition_id,
        question: best.question,
        yes_token_id: best.clob_token_ids.first().cloned(),
        no_token_id: best.clob_token_ids.get(1).cloned(),
        window,
        status: "active".to_string(),
    })
}

fn market_family_candidates(seed: &str) -> Vec<String> {
    let mut candidates = vec![seed.to_ascii_lowercase()];
    if seed.contains("bitcoin") || seed.contains("btc") {
        candidates.push("btc-updown-5m".to_string());
        candidates.push("bitcoin-up-or-down".to_string());
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn market_rank(market: &GammaMarket, seed: &str, expected_suffix: &str) -> u8 {
    let slug = market.slug.to_ascii_lowercase();
    let seed = seed.to_ascii_lowercase();
    let family_match = slug.contains("btc-updown-5m")
        || slug.contains("bitcoin-up-or-down")
        || slug.contains(&seed);
    if !family_match {
        return 0;
    }
    let exact_window_match = slug.ends_with(expected_suffix);
    let has_token_ids = market.clob_token_ids.len() >= 2;
    match (exact_window_match, has_token_ids) {
        (true, true) => 4,
        (true, false) => 3,
        (false, true) => 2,
        (false, false) => 1,
    }
}

fn deserialize_token_ids<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TokenIds {
        Array(Vec<String>),
        Stringified(String),
    }

    let value = Option::<TokenIds>::deserialize(deserializer)?;
    Ok(match value {
        Some(TokenIds::Array(values)) => values,
        Some(TokenIds::Stringified(values)) => serde_json::from_str::<Vec<String>>(&values).unwrap_or_default(),
        None => Vec::new(),
    })
}

async fn fetch_gamma_markets(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
) -> Result<Vec<GammaMarket>, GatewayError> {
    let response = client
        .get(format!("{}{}", base_url, path))
        .send()
        .await
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !status.is_success() {
        return Err(GatewayError::Upstream(format!(
            "gamma status {} body {}",
            status, body
        )));
    }
    let parsed = serde_json::from_str::<GammaResponse>(&body)
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    Ok(match parsed {
        GammaResponse::List(markets) => markets,
        GammaResponse::Single(market) => vec![market],
    })
}

async fn fetch_gamma_event_markets(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
) -> Result<Vec<GammaMarket>, GatewayError> {
    let response = client
        .get(format!("{}{}", base_url, path))
        .send()
        .await
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !status.is_success() {
        return Err(GatewayError::Upstream(format!(
            "gamma status {} body {}",
            status, body
        )));
    }
    let parsed = serde_json::from_str::<GammaEventResponse>(&body)
        .map_err(|err| GatewayError::Upstream(err.to_string()))?;
    Ok(match parsed {
        GammaEventResponse::List(events) => {
            events.into_iter().flat_map(|event| event.markets).collect()
        }
        GammaEventResponse::Single(event) => event.markets,
    })
}
