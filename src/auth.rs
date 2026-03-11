use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ethers_core::types::{
    Address, H256, U256,
    transaction::eip712::{EIP712Domain, Eip712, Eip712DomainType, TypedData, Types},
};
use ethers_core::utils::to_checksum;
use ethers_signers::{LocalWallet, Signer};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use time::OffsetDateTime;
use tracing::info;

use crate::{
    config::{Config, DevUserConfig},
    error::GatewayError,
};

type HmacSha256 = Hmac<Sha256>;
const POLY_CHAIN_ID: u64 = 137;
const DEFAULT_NONCE: u64 = 0;
const ATTEST_MESSAGE: &str = "This message attests that I control the given wallet";

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: String,
}

#[derive(Clone, Debug)]
pub struct PolymarketApiCredentials {
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

#[derive(Clone, Debug)]
pub struct PolymarketUserContext {
    pub user_id: String,
    pub signer_address: String,
    pub api: PolymarketApiCredentials,
}

#[derive(Clone)]
pub struct AuthService {
    secret: Arc<Vec<u8>>,
    token_users: Arc<HashMap<String, String>>,
    polymarket_users: Arc<HashMap<String, PolymarketUserContext>>,
}

impl AuthService {
    pub async fn new(config: &Config) -> Result<Self> {
        let token_users = config
            .dev_users
            .iter()
            .map(|user| (user.token.clone(), user.id.clone()))
            .collect();
        let client = reqwest::Client::builder().use_rustls_tls().build()?;
        let mut polymarket_users = HashMap::new();
        for user in &config.dev_users {
            let context =
                bootstrap_polymarket_user(&client, &config.poly_clob_base_url, user).await?;
            info!(user = %user.id, signer_address = %context.signer_address, "bootstrapped polymarket api credentials");
            polymarket_users.insert(user.id.clone(), context);
        }
        Ok(Self {
            secret: Arc::new(config.auth_secret.as_bytes().to_vec()),
            token_users: Arc::new(token_users),
            polymarket_users: Arc::new(polymarket_users),
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<AuthenticatedUser, GatewayError> {
        if let Some(user_id) = self.token_users.get(token) {
            return Ok(AuthenticatedUser {
                user_id: user_id.clone(),
            });
        }

        let mut parts = token.split('.');
        let user_id = parts.next().ok_or(GatewayError::Unauthorized)?;
        let exp = parts.next().ok_or(GatewayError::Unauthorized)?;
        let nonce = parts.next().ok_or(GatewayError::Unauthorized)?;
        let sig = parts.next().ok_or(GatewayError::Unauthorized)?;
        if parts.next().is_some() {
            return Err(GatewayError::Unauthorized);
        }

        let exp_ts = exp.parse::<i64>().map_err(|_| GatewayError::Unauthorized)?;
        if OffsetDateTime::now_utc().unix_timestamp() > exp_ts {
            return Err(GatewayError::Unauthorized);
        }

        let message = format!("{user_id}.{exp}.{nonce}");
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| GatewayError::Unauthorized)?;
        mac.update(message.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        if expected != sig {
            return Err(GatewayError::Unauthorized);
        }

        if !self.token_users.values().any(|known| known == user_id) {
            return Err(GatewayError::Unauthorized);
        }

        Ok(AuthenticatedUser {
            user_id: user_id.to_string(),
        })
    }

    pub fn polymarket_user(&self, user_id: &str) -> Result<PolymarketUserContext, GatewayError> {
        self.polymarket_users
            .get(user_id)
            .cloned()
            .ok_or(GatewayError::Unauthorized)
    }
}

#[derive(Debug, Deserialize)]
struct ApiKeyResponse {
    #[serde(rename = "apiKey")]
    api_key: String,
    secret: String,
    passphrase: String,
}

async fn bootstrap_polymarket_user(
    client: &reqwest::Client,
    base_url: &str,
    user: &DevUserConfig,
) -> Result<PolymarketUserContext> {
    let signer_address = derive_eth_address(&user.private_key)?;
    let api =
        derive_or_create_api_credentials(client, base_url, &user.private_key, &signer_address)
            .await?;
    Ok(PolymarketUserContext {
        user_id: user.id.clone(),
        signer_address,
        api,
    })
}

async fn derive_or_create_api_credentials(
    client: &reqwest::Client,
    base_url: &str,
    private_key: &str,
    signer_address: &str,
) -> Result<PolymarketApiCredentials> {
    if let Ok(creds) = request_api_credentials(
        client,
        base_url,
        private_key,
        signer_address,
        reqwest::Method::POST,
        "/auth/api-key",
    )
    .await
    {
        return Ok(creds);
    }

    request_api_credentials(
        client,
        base_url,
        private_key,
        signer_address,
        reqwest::Method::GET,
        "/auth/derive-api-key",
    )
    .await
}

async fn request_api_credentials(
    client: &reqwest::Client,
    base_url: &str,
    private_key: &str,
    signer_address: &str,
    method: reqwest::Method,
    path: &str,
) -> Result<PolymarketApiCredentials> {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let signature =
        sign_l1_auth_message(private_key, signer_address, &timestamp, DEFAULT_NONCE).await?;
    let response = client
        .request(method, format!("{base_url}{path}"))
        .header("POLY_ADDRESS", signer_address)
        .header("POLY_SIGNATURE", signature)
        .header("POLY_TIMESTAMP", &timestamp)
        .header("POLY_NONCE", DEFAULT_NONCE.to_string())
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("polymarket auth bootstrap failed with status {status}: {body}");
    }
    let parsed: ApiKeyResponse = serde_json::from_str(&body)?;
    Ok(PolymarketApiCredentials {
        api_key: parsed.api_key,
        secret: parsed.secret,
        passphrase: parsed.passphrase,
    })
}

async fn sign_l1_auth_message(
    private_key: &str,
    signer_address: &str,
    timestamp: &str,
    nonce: u64,
) -> Result<String> {
    let wallet: LocalWallet = private_key.parse()?;
    let signer: Address = signer_address.parse()?;
    let mut types = Types::new();
    types.insert(
        "EIP712Domain".to_string(),
        vec![
            Eip712DomainType {
                name: "name".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "version".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "chainId".to_string(),
                r#type: "uint256".to_string(),
            },
        ],
    );
    types.insert(
        "ClobAuth".to_string(),
        vec![
            Eip712DomainType {
                name: "address".to_string(),
                r#type: "address".to_string(),
            },
            Eip712DomainType {
                name: "timestamp".to_string(),
                r#type: "string".to_string(),
            },
            Eip712DomainType {
                name: "nonce".to_string(),
                r#type: "uint256".to_string(),
            },
            Eip712DomainType {
                name: "message".to_string(),
                r#type: "string".to_string(),
            },
        ],
    );
    let domain = EIP712Domain {
        name: Some("ClobAuthDomain".to_string()),
        version: Some("1".to_string()),
        chain_id: Some(POLY_CHAIN_ID.into()),
        verifying_contract: None,
        salt: None,
    };
    let mut message = BTreeMap::new();
    message.insert(
        "address".to_string(),
        serde_json::json!(format!("{:#x}", signer)),
    );
    message.insert("timestamp".to_string(), serde_json::json!(timestamp));
    message.insert("nonce".to_string(), serde_json::json!(nonce));
    message.insert("message".to_string(), serde_json::json!(ATTEST_MESSAGE));
    let payload = TypedData {
        types,
        primary_type: "ClobAuth".to_string(),
        domain,
        message,
    };
    let encoded = payload.encode_eip712()?;
    let mut signature = wallet.sign_hash(H256::from(encoded))?;
    if signature.v < 27 {
        signature.v += 27;
    }
    Ok(encode_signature_hex(signature.r, signature.s, signature.v))
}

fn derive_eth_address(private_key: &str) -> Result<String> {
    let wallet: LocalWallet = private_key.parse()?;
    Ok(to_checksum(&wallet.address(), None))
}

fn encode_signature_hex(r: U256, s: U256, v: u64) -> String {
    let mut bytes = [0u8; 65];
    r.to_big_endian(&mut bytes[0..32]);
    s.to_big_endian(&mut bytes[32..64]);
    bytes[64] = v as u8;
    format!("0x{}", hex::encode(bytes))
}
