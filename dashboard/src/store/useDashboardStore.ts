import { create } from "zustand";
import { fetchJson, wsUrl } from "@/lib/api";
import type { AdminSnapshot, DashboardSnapshot } from "@/lib/types";
import { normalizeApiDates } from "@/lib/utils";

interface DashboardState {
  publicSnapshot?: DashboardSnapshot;
  adminSnapshot?: AdminSnapshot;
  token?: string;
  booted: boolean;
  loading: boolean;
  error?: string;
  init: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
}

let publicSocket: WebSocket | undefined;
let adminSocket: WebSocket | undefined;

export const useDashboardStore = create<DashboardState>((set, get) => ({
  token: localStorage.getItem("pm_gw_admin_token") ?? undefined,
  booted: false,
  loading: false,
  init: async () => {
    if (get().loading) return;
    set({ loading: true, error: undefined });
    try {
      const publicSnapshot = normalizeApiDates(
        await fetchJson<DashboardSnapshot>("/api/public/status"),
      );
      set({ publicSnapshot, booted: true });
      connectPublicSocket();
      if (get().token) {
        await loadAdminState(get().token!, set);
        connectAdminSocket(get().token!, set);
      }
    } catch (error) {
      set({ error: error instanceof Error ? error.message : "Failed to load dashboard" });
    } finally {
      set({ loading: false });
    }
  },
  login: async (username, password) => {
    const result = await fetchJson<{ token: string }>("/api/admin/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
    localStorage.setItem("pm_gw_admin_token", result.token);
    set({ token: result.token });
    await loadAdminState(result.token, set);
    connectAdminSocket(result.token, set);
  },
  logout: () => {
    localStorage.removeItem("pm_gw_admin_token");
    adminSocket?.close();
    adminSocket = undefined;
    set({ token: undefined, adminSnapshot: undefined });
  },
}));

async function loadAdminState(
  token: string,
  set: (partial: Partial<DashboardState>) => void,
) {
  const adminSnapshot = await fetchJson<AdminSnapshot>("/api/admin/status", {
    headers: { Authorization: `Bearer ${token}` },
  });
  set({ adminSnapshot: normalizeApiDates(adminSnapshot) });
}

function connectPublicSocket() {
  publicSocket?.close();
  publicSocket = new WebSocket(wsUrl("/ws/dashboard/public"));
  publicSocket.onmessage = (event) => {
    const data = normalizeApiDates(JSON.parse(event.data));
    if (data.type === "dashboard_snapshot") {
      useDashboardStore.setState({ publicSnapshot: data.payload });
    }
  };
}

function connectAdminSocket(
  token: string,
  set: (partial: Partial<DashboardState>) => void,
) {
  adminSocket?.close();
  adminSocket = new WebSocket(`${wsUrl("/ws/dashboard/admin")}?token=${encodeURIComponent(token)}`);
  adminSocket.onopen = () => {
    fetchJson<AdminSnapshot>("/api/admin/status", {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then((adminSnapshot) => set({ adminSnapshot: normalizeApiDates(adminSnapshot) }))
      .catch((error) => set({ error: error instanceof Error ? error.message : "Admin stream error" }));
  };
  adminSocket.onmessage = (event) => {
    const data = normalizeApiDates(JSON.parse(event.data));
    if (data.type === "dashboard_snapshot") {
      useDashboardStore.setState({ adminSnapshot: data.payload });
    }
  };
}
