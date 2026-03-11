use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    PolymarketClob,
    PolymarketRtdsBinance,
    PolymarketRtdsChainlink,
    Binance,
    Coinbase,
    Kraken,
    Okx,
    Bitstamp,
    Gateway,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SizeType {
    Shares,
    Dollars,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connecting,
    Connected,
    Degraded,
    Disconnected,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PriceTick,
    OrderBookSnapshot,
    OrderBookDelta,
    TradePrint,
    MarketStatus,
    MarketRollover,
    AccountUpdate,
    PositionUpdate,
    OrderUpdate,
    FillUpdate,
    Heartbeat,
    ErrorEvent,
    ConnectionStatus,
}
