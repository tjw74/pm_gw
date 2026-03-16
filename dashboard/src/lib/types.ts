export type Severity = "healthy" | "warning" | "critical";

export interface FeedItem {
  adapter: string;
  connection: "connecting" | "connected" | "degraded" | "disconnected";
  last_message_at?: string | null;
  last_message_age_ms?: number | null;
  reconnect_count: number;
  message_rate_per_sec: number;
  recent_disconnects_60s: number;
  last_latency_ms?: number | null;
  last_error?: string | null;
  stale: boolean;
  freshness_expected: boolean;
}

export interface AlertItem {
  id: string;
  severity: "warning" | "critical";
  title: string;
  detail: string;
  timestamp: string;
}

export interface DashboardSnapshot {
  meta: {
    version: string;
    commit: string;
    uptime_seconds: number;
    generated_at: string;
    ready: boolean;
  };
  global: {
    gateway: string;
    overall_health: Severity;
    active_market_family: string;
    active_market_slug?: string | null;
    window_countdown_seconds?: number | null;
    upstreams_connected: number;
    upstreams_total: number;
    downstream_clients: number;
    alert_level: Severity;
  };
  feeds: FeedItem[];
  market: {
    active?: {
      slug: string;
      question?: string | null;
      yes_token_id?: string | null;
      no_token_id?: string | null;
      window: {
        window_start: string;
        window_end: string;
      };
    } | null;
    target_price?: number | null;
    latest_market_price?: number | null;
    latest_reference_prices: Record<string, number>;
    price_history: Record<string, { timestamp: string; value: number }[]>;
    orderbook_snapshot?: unknown;
  };
  streaming: {
    active_clients: number;
    authenticated_clients: number;
    outbound_messages_per_sec: number;
    outbound_bytes_per_sec: number;
    dropped_messages_total: number;
    auth_failures_total: number;
    disconnects_total: number;
    avg_command_latency_ms?: number | null;
    sessions: Array<{
      id: string;
      user_id: string;
      created_at: string;
      last_seen_at: string;
      subscriptions: string[];
    }>;
  };
  alerts: AlertItem[];
  logs: Array<{
    timestamp: string;
    actor: string;
    action: string;
    detail: string;
  }>;
}

export interface AdminSnapshot {
  public: DashboardSnapshot;
  admin: {
    runtime_kill_switch: boolean;
    accounts: Record<string, unknown>;
    active_orders: Record<string, unknown>;
    positions: Record<string, unknown>;
    audit: Array<{
      timestamp: string;
      actor: string;
      action: string;
      detail: string;
    }>;
  };
}
