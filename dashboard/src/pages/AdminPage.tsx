import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { fetchJson } from "@/lib/api";
import { formatDateTime } from "@/lib/utils";
import { useDashboardStore } from "@/store/useDashboardStore";

export function AdminPage() {
  const token = useDashboardStore((state) => state.token);
  const adminSnapshot = useDashboardStore((state) => state.adminSnapshot);
  const init = useDashboardStore((state) => state.init);
  const [busy, setBusy] = useState(false);

  if (!token || !adminSnapshot) {
    return (
      <Card>
        <h1 className="text-xl font-semibold">Admin mode locked</h1>
        <p className="mt-2 text-sm text-muted-foreground">Login is required to access controls and sensitive runtime detail.</p>
      </Card>
    );
  }

  const setKillSwitch = async (enabled: boolean) => {
    setBusy(true);
    try {
      await fetchJson("/api/admin/controls/kill-switch", {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        body: JSON.stringify({ enabled }),
      });
      await init();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <Card>
        <div className="flex items-center justify-between">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Protected controls</div>
            <h1 className="mt-1.5 text-xl font-semibold">Runtime control plane</h1>
          </div>
          <div className="text-sm text-muted-foreground">
            Kill switch: {adminSnapshot.admin.runtime_kill_switch ? "enabled" : "disabled"}
          </div>
        </div>
        <div className="mt-4 flex gap-2.5">
          <Button disabled={busy} onClick={() => setKillSwitch(true)}>Enable kill switch</Button>
          <Button disabled={busy} variant="subtle" onClick={() => setKillSwitch(false)}>Disable kill switch</Button>
        </div>
      </Card>
      <Card>
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Audit</div>
        <div className="mt-3 space-y-2">
          {adminSnapshot.admin.audit.map((entry, index) => (
            <div key={`${entry.timestamp}-${index}`} className="rounded-2xl border border-white/5 bg-black/10 p-3 text-sm">
              <div className="font-medium">{entry.action}</div>
              <div className="mt-1 text-muted-foreground">{entry.detail}</div>
              <div className="mt-2 text-xs text-muted-foreground">{formatDateTime(entry.timestamp)}</div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
