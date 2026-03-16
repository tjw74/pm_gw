export type TradeSide = "buy" | "sell";
export type Outcome = "up" | "down";
export type OrderType = "market" | "limit";
export type SizeType = "shares" | "dollars";

export interface ActiveMarket {
  market_id: string;
  slug: string;
  question?: string | null;
  yes_token_id?: string | null;
  no_token_id?: string | null;
  window: {
    window_start: string;
    window_end: string;
  };
  status: string;
}

export interface AccountSummary {
  wallet_id: string;
  cash_balance?: number | null;
  portfolio_value?: number | null;
  updated_at?: string | null;
}

export interface Position {
  market_slug: string;
  outcome: string;
  size: number;
  average_price?: number | null;
  unrealized_pnl?: number | null;
}

export interface PositionState {
  wallet_id: string;
  positions: Record<string, Position>;
}

export interface OrderState {
  order_id: string;
  market_slug?: string | null;
  outcome?: string | null;
  side: string;
  price?: number | null;
  size: number;
  filled_size?: number | null;
  status: string;
  updated_at: string;
}

export interface PortfolioPoint {
  timestamp: string;
  portfolio_value: number;
  unrealized_pnl?: number | null;
}

export interface PositionHistoryEntry {
  timestamp: string;
  market_slug: string;
  outcome: string;
  size: number;
  average_price?: number | null;
  unrealized_pnl?: number | null;
  status: string;
}

export interface TradeSnapshot {
  meta: {
    generated_at: string;
    ready: boolean;
    user_id: string;
  };
  market: {
    active?: ActiveMarket | null;
    target_price?: number | null;
    latest_market_price?: number | null;
    latest_reference_prices: Record<string, number>;
    price_history?: Record<string, Array<{ timestamp: string; value: number }>>;
    orderbook_snapshot?: {
      bids?: unknown;
      asks?: unknown;
      [key: string]: unknown;
    } | null;
  };
  account: {
    summary?: AccountSummary | null;
    profitability: {
      portfolio_value?: number | null;
      cash_balance?: number | null;
      unrealized_pnl?: number | null;
    };
    open_orders: OrderState[];
    positions?: PositionState | null;
    portfolio_history: PortfolioPoint[];
    position_history: PositionHistoryEntry[];
  };
}

export interface CommandEnvelopeBase {
  command_id: string;
  timestamp: string;
}
