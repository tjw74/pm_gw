use std::sync::Arc;

use anyhow::Result;
use base64::{Engine, engine::general_purpose::URL_SAFE};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method};
use serde_json::{Value, json};
use sha2::Sha256;
use time::OffsetDateTime;
use tracing::{info, warn};

use crate::{commands::ClientMessage, config::Config, error::GatewayError, state::AppState};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct PolymarketExecutionClient {
    client: Client,
    config: Config,
    state: Arc<AppState>,
}

impl PolymarketExecutionClient {
    pub fn new(config: Config, state: Arc<AppState>) -> Result<Self> {
        Ok(Self {
            client: Client::builder().use_rustls_tls().build()?,
            config,
            state,
        })
    }

    pub async fn sync_user_state(&self) -> Result<()> {
        for user in &self.config.dev_users {
            if let Ok(context) = self.state.auth.polymarket_user(&user.id) {
                let _ = self.fetch_account_state(&context).await;
            }
        }
        Ok(())
    }

    pub async fn handle_command(
        &self,
        user_id: &str,
        message: &ClientMessage,
    ) -> Result<Value, GatewayError> {
        let user = self.state.auth.polymarket_user(user_id)?;
        match message {
            ClientMessage::Ping { .. } => Ok(json!({"status": "pong"})),
            ClientMessage::PlaceLimitOrder {
                side,
                outcome,
                size_type,
                size,
                price,
                ..
            } => {
                self.state.enforce_risk(*size, Some(*price)).await?;
                let market = self.state.active_market_slug().await;
                let payload = json!({
                    "market": market,
                    "side": side,
                    "outcome": outcome,
                    "size_type": size_type,
                    "size": size,
                    "price": price,
                    "order_type": "limit",
                });
                self.signed_request(&user, Method::POST, "/order", Some(payload))
                    .await
            }
            ClientMessage::PlaceMarketOrder {
                side,
                outcome,
                size_type,
                size,
                ..
            } => {
                self.state.enforce_risk(*size, None).await?;
                let payload = json!({
                    "market": self.state.active_market_slug().await,
                    "side": side,
                    "outcome": outcome,
                    "size_type": size_type,
                    "size": size,
                    "order_type": "market",
                });
                self.signed_request(&user, Method::POST, "/order", Some(payload))
                    .await
            }
            ClientMessage::CancelOrder { order_id, .. } => {
                self.signed_request(
                    &user,
                    Method::POST,
                    "/cancel",
                    Some(json!({ "order_id": order_id })),
                )
                .await
            }
            ClientMessage::CancelAll { .. } => {
                self.signed_request(
                    &user,
                    Method::POST,
                    "/cancel-all",
                    Some(json!({ "market": self.state.active_market_slug().await })),
                )
                .await
            }
            ClientMessage::GetOpenOrders { .. } => {
                self.signed_request(&user, Method::GET, "/orders", None)
                    .await
            }
            ClientMessage::GetPositions { .. } => {
                self.signed_request(&user, Method::GET, "/positions", None)
                    .await
            }
            ClientMessage::GetAccountState { .. } => self.fetch_account_state(&user).await,
            ClientMessage::SetTargetPrice { price, .. } => {
                self.state.set_target_price(*price).await;
                Ok(json!({ "target_price": price }))
            }
            ClientMessage::SubscribeMarket { market, .. } => Ok(json!({ "market": market })),
            ClientMessage::UnsubscribeMarket { market, .. } => Ok(json!({ "market": market })),
            ClientMessage::Auth { .. } => Err(GatewayError::Unauthorized),
        }
    }

    async fn fetch_account_state(
        &self,
        user: &crate::auth::PolymarketUserContext,
    ) -> Result<Value, GatewayError> {
        let positions = self.fetch_positions(user).await.unwrap_or_else(|err| {
            warn!(user = %user.user_id, ?err, "position fetch failed");
            json!([])
        });
        let open_orders = self.fetch_open_orders(user).await.unwrap_or_else(|err| {
            warn!(user = %user.user_id, ?err, "open orders fetch failed");
            json!([])
        });
        let value = self.fetch_holdings_value(user).await.unwrap_or_else(|err| {
            warn!(user = %user.user_id, ?err, "holdings value fetch failed");
            json!([])
        });
        let allowance = self
            .signed_request_with_query(
                user,
                Method::GET,
                "/balance-allowance",
                None,
                Some(&[("asset_type", "COLLATERAL"), ("signature_type", "0")]),
            )
            .await
            .unwrap_or_else(|err| {
                warn!(user = %user.user_id, ?err, "balance allowance fetch failed");
                json!({})
            });
        let payload = json!({
            "user": user.signer_address,
            "balance_allowance": allowance,
            "positions": positions,
            "open_orders": open_orders,
            "holdings_value": value,
        });
        self.state.set_account_state(&user.user_id, &payload).await;
        Ok(payload)
    }

    async fn fetch_open_orders(
        &self,
        user: &crate::auth::PolymarketUserContext,
    ) -> Result<Value, GatewayError> {
        self.signed_request(user, Method::GET, "/data/orders", None)
            .await
    }

    async fn fetch_positions(
        &self,
        user: &crate::auth::PolymarketUserContext,
    ) -> Result<Value, GatewayError> {
        self.public_request(
            &self.config.poly_data_api_base_url,
            Method::GET,
            "/positions",
            Some(&[("user", user.signer_address.as_str())]),
        )
        .await
    }

    async fn fetch_holdings_value(
        &self,
        user: &crate::auth::PolymarketUserContext,
    ) -> Result<Value, GatewayError> {
        self.public_request(
            &self.config.poly_data_api_base_url,
            Method::GET,
            "/value",
            Some(&[("user", user.signer_address.as_str())]),
        )
        .await
    }

    async fn signed_request(
        &self,
        user: &crate::auth::PolymarketUserContext,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, GatewayError> {
        self.signed_request_with_query(user, method, path, body, None)
            .await
    }

    async fn signed_request_with_query(
        &self,
        user: &crate::auth::PolymarketUserContext,
        method: Method,
        path: &str,
        body: Option<Value>,
        query: Option<&[(&str, &str)]>,
    ) -> Result<Value, GatewayError> {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
        let body_text = body.as_ref().map(Value::to_string).unwrap_or_default();
        let signature_payload = format!("{}{}{}{}", timestamp, method.as_str(), path, body_text);
        let secret_bytes = URL_SAFE
            .decode(user.api.secret.as_bytes())
            .unwrap_or_else(|_| user.api.secret.as_bytes().to_vec());
        let mut mac = HmacSha256::new_from_slice(&secret_bytes)
            .map_err(|err| GatewayError::internal(err.to_string()))?;
        mac.update(signature_payload.as_bytes());
        let signature = URL_SAFE.encode(mac.finalize().into_bytes());

        let url = format!("{}{}", self.config.poly_clob_base_url, path);
        let mut request = self
            .client
            .request(method, &url)
            .header("POLY_ADDRESS", &user.signer_address)
            .header("POLY_API_KEY", &user.api.api_key)
            .header("POLY_PASSPHRASE", &user.api.passphrase)
            .header("POLY_SIGNATURE", signature)
            .header("POLY_TIMESTAMP", timestamp);

        if let Some(query) = query {
            request = request.query(query);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|err| GatewayError::Upstream(err.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| GatewayError::Upstream(err.to_string()))?;
        info!(user = %user.user_id, path, %status, "polymarket REST request completed");

        if !status.is_success() {
            return Err(GatewayError::Upstream(format!(
                "status {} body {}",
                status, text
            )));
        }

        serde_json::from_str(&text)
            .or_else(|_| Ok(json!({ "raw": text, "status": status.as_u16() })))
            .map_err(|err: serde_json::Error| GatewayError::internal(err.to_string()))
    }

    async fn public_request(
        &self,
        base_url: &str,
        method: Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<Value, GatewayError> {
        let mut request = self.client.request(method, format!("{}{}", base_url, path));
        if let Some(query) = query {
            request = request.query(query);
        }
        let response = request
            .send()
            .await
            .map_err(|err| GatewayError::Upstream(err.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| GatewayError::Upstream(err.to_string()))?;
        if !status.is_success() {
            return Err(GatewayError::Upstream(format!(
                "status {} body {}",
                status, text
            )));
        }
        serde_json::from_str(&text)
            .or_else(|_| Ok(json!({ "raw": text, "status": status.as_u16() })))
            .map_err(|err: serde_json::Error| GatewayError::internal(err.to_string()))
    }
}
