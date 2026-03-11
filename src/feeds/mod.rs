pub mod binance;
pub mod bitstamp;
pub mod coinbase;
pub mod kraken;
pub mod okx;
pub mod polymarket_clob_market;
pub mod polymarket_clob_user;
pub mod polymarket_gamma;
pub mod polymarket_rtds;

use std::sync::Arc;

use crate::{execution::polymarket::PolymarketExecutionClient, state::AppState};

pub async fn spawn_all(state: Arc<AppState>, execution: Arc<PolymarketExecutionClient>) {
    polymarket_gamma::spawn(state.clone()).await;
    polymarket_clob_market::spawn(state.clone()).await;
    polymarket_clob_user::spawn(state.clone(), execution.clone()).await;
    polymarket_rtds::spawn(state.clone()).await;

    if state.config.enable_binance_feed {
        binance::spawn(state.clone()).await;
    }
    if state.config.enable_coinbase_feed {
        coinbase::spawn(state.clone()).await;
    }
    if state.config.enable_kraken_feed {
        kraken::spawn(state.clone()).await;
    }
    if state.config.enable_okx_feed {
        okx::spawn(state.clone()).await;
    }
    if state.config.enable_bitstamp_feed {
        bitstamp::spawn(state).await;
    }
}
