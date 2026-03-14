import { AlertRail } from "@/components/AlertRail";
import { Card } from "@/components/ui/card";
import { formatDateTime } from "@/lib/utils";
import { useDashboardStore } from "@/store/useDashboardStore";

export function AlertsPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  return (
    <div className="space-y-4">
      <AlertRail alerts={snapshot.alerts} />
      <Card>
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Audit trail</div>
        <div className="mt-3 space-y-2">
          {snapshot.logs.map((entry, index) => (
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
