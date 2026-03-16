import { create } from "zustand";

import { fetchTradeSession, refreshTradeSession, tradeWsUrl } from "@/lib/api";
import type { OrderType, Outcome, SizeType, TradeSide, TradeSnapshot } from "@/lib/types";
import { normalizeApiDates } from "@/lib/utils";

const storedTokenKey = "pm_trade_token";

let ws: WebSocket | null = null;
let refreshTimer: number | null = null;
let reconnectTimer: number | null = null;
let allowReconnect = false;

interface TradeStore {
  token?: string;
  snapshot?: TradeSnapshot;
  connected: boolean;
  syncState: "connecting" | "live" | "recovering";
  loading: boolean;
  sending: boolean;
  authError?: string | null;
  lastCommandMessage?: string | null;
  activePage: number;
  bootstrapFromStorage: () => Promise<void>;
  login: (token: string) => Promise<void>;
  logout: () => void;
  setActivePage: (index: number) => void;
  placeOrder: (params: {
    orderType: OrderType;
    side: TradeSide;
    outcome: Outcome;
    sizeType: SizeType;
    size: number;
    price?: number;
  }) => void;
  cancelOrder: (orderId: string) => void;
  cancelAll: () => void;
  refresh: () => Promise<void>;
}

function commandEnvelope(type: string, body: Record<string, unknown>) {
  return JSON.stringify({
    type,
    command_id: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    ...body,
  });
}

function closeSockets() {
  allowReconnect = false;
  if (refreshTimer != null) {
    window.clearInterval(refreshTimer);
    refreshTimer = null;
  }
  if (reconnectTimer != null) {
    window.clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  ws?.close();
  ws = null;
}

function scheduleReconnect(token: string, set: (partial: Partial<TradeStore>) => void) {
  if (!allowReconnect) return;
  if (reconnectTimer != null) return;
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null;
    connectTradeWs(token, set);
  }, 2000);
}

function connectTradeWs(token: string, set: (partial: Partial<TradeStore>) => void) {
  allowReconnect = true;
  closeSockets();
  allowReconnect = true;
  ws = new WebSocket(tradeWsUrl());
  ws.addEventListener("open", () => {
    ws?.send(JSON.stringify({ type: "auth", token }));
  });
  ws.addEventListener("message", (event) => {
    const payload = normalizeApiDates(JSON.parse(event.data));
    if (payload.type === "auth_ok") {
      set({ connected: true, syncState: "live", authError: null });
      ws?.send(commandEnvelope("subscribe_market", { market: null }));
      ws?.send(commandEnvelope("get_account_state", {}));
      return;
    }
    if (payload.type === "snapshot") {
      const next = payload.payload?.snapshot ?? payload.payload;
      set({ snapshot: next as TradeSnapshot });
      return;
    }
    if (payload.type === "auth_error") {
      set({ authError: payload.error ?? "authentication failed", connected: false, syncState: "connecting" });
      return;
    }
    if (payload.type === "order_update") {
      set({ lastCommandMessage: "Order request submitted" });
      return;
    }
  });
  ws.addEventListener("close", () => {
    set({ syncState: "recovering" });
    scheduleReconnect(token, set);
  });
  ws.addEventListener("error", () => {
    set({ syncState: "recovering" });
    scheduleReconnect(token, set);
  });
  refreshTimer = window.setInterval(() => {
    ws?.send(commandEnvelope("get_account_state", {}));
  }, 15000);
}

export const useTradeStore = create<TradeStore>((set, get) => ({
  connected: false,
  syncState: "connecting",
  loading: false,
  sending: false,
  activePage: 0,
  bootstrapFromStorage: async () => {
    const token = window.localStorage.getItem(storedTokenKey);
    if (!token) return;
    try {
      set({ loading: true, token });
      const snapshot = await fetchTradeSession(token);
      set({ snapshot, authError: null, syncState: "connecting" });
      connectTradeWs(token, (partial) => set(partial));
    } catch {
      window.localStorage.removeItem(storedTokenKey);
      set({ token: undefined, snapshot: undefined, authError: "Stored key is no longer valid" });
    } finally {
      set({ loading: false });
    }
  },
  login: async (token) => {
    try {
      set({ loading: true, authError: null });
      const snapshot = await fetchTradeSession(token);
      window.localStorage.setItem(storedTokenKey, token);
      set({ token, snapshot, activePage: 0, syncState: "connecting" });
      connectTradeWs(token, (partial) => set(partial));
    } catch (error) {
      set({ authError: error instanceof Error ? error.message : "Login failed" });
    } finally {
      set({ loading: false });
    }
  },
  logout: () => {
    closeSockets();
    window.localStorage.removeItem(storedTokenKey);
    set({
      token: undefined,
      snapshot: undefined,
      connected: false,
      syncState: "connecting",
      loading: false,
      sending: false,
      authError: null,
      lastCommandMessage: null,
      activePage: 0,
    });
  },
  setActivePage: (index) => set({ activePage: index }),
  placeOrder: (params) => {
    if (!ws) return;
    set({ sending: true, lastCommandMessage: null });
    const message =
      params.orderType === "limit"
        ? commandEnvelope("place_limit_order", {
            side: params.side,
            outcome: params.outcome,
            size_type: params.sizeType,
            size: params.size,
            price: params.price,
          })
        : commandEnvelope("place_market_order", {
            side: params.side,
            outcome: params.outcome,
            size_type: params.sizeType,
            size: params.size,
          });
    ws.send(message);
    window.setTimeout(() => set({ sending: false }), 800);
  },
  cancelOrder: (orderId) => {
    if (!ws) return;
    ws.send(commandEnvelope("cancel_order", { order_id: orderId }));
  },
  cancelAll: () => {
    if (!ws) return;
    ws.send(commandEnvelope("cancel_all", {}));
  },
  refresh: async () => {
    const token = get().token;
    if (!token) return;
    try {
      const snapshot = await refreshTradeSession(token);
      set({ snapshot, authError: null });
    } catch (error) {
      set({ authError: error instanceof Error ? error.message : "Refresh failed" });
    }
  },
}));
