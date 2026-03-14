import { Card } from "@/components/ui/card";
import type { AlertItem } from "@/lib/types";

export function AlertRail({ alerts }: { alerts: AlertItem[] }) {
  return (
    <Card className="xl:col-span-5">
      <div className="mb-3">
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Incidents</div>
        <h2 className="mt-1.5 text-xl font-semibold">Current alerts</h2>
      </div>
      <div className="space-y-2">
        {alerts.length === 0 ? (
          <div className="rounded-2xl border border-success/20 bg-success/10 p-3 text-sm text-success">
            No active incidents. Feeds and scheduler look stable.
          </div>
        ) : alerts.map((alert) => (
          <div key={alert.id} className={`rounded-2xl border p-3 ${alert.severity === "critical" ? "border-danger/30 bg-danger/10" : "border-warning/30 bg-warning/10"}`}>
            <div className="text-sm font-medium">{alert.title}</div>
            <div className="mt-1.5 text-xs text-muted-foreground">{alert.detail}</div>
          </div>
        ))}
      </div>
    </Card>
  );
}
