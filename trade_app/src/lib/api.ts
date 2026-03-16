import { normalizeApiDates } from "@/lib/utils";
import type { TradeSnapshot } from "@/lib/types";

const apiBase = (import.meta.env.VITE_GATEWAY_HTTP_URL as string | undefined) ?? "";

function withBase(path: string) {
  if (!apiBase) return path;
  return `${apiBase}${path}`;
}

export async function fetchTradeSession(token: string): Promise<TradeSnapshot> {
  const response = await fetch(withBase("/api/trade/session"), {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!response.ok) {
    throw new Error(`session bootstrap failed (${response.status})`);
  }
  const payload = await response.json();
  return normalizeApiDates(payload);
}

export async function refreshTradeSession(token: string): Promise<TradeSnapshot> {
  const response = await fetch(withBase("/api/trade/refresh"), {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  if (!response.ok) {
    throw new Error(`session refresh failed (${response.status})`);
  }
  const payload = await response.json();
  return normalizeApiDates(payload.snapshot);
}

export function tradeWsUrl() {
  const override = import.meta.env.VITE_GATEWAY_WS_URL as string | undefined;
  if (override) return override;
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/ws/trade`;
}
