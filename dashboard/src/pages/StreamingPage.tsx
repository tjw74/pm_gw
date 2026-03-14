import { Card } from "@/components/ui/card";
import { formatDateTime } from "@/lib/utils";
import { StreamingPanel } from "@/components/StreamingPanel";
import { useDashboardStore } from "@/store/useDashboardStore";

export function StreamingPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  return (
    <div className="space-y-4">
      <StreamingPanel snapshot={snapshot} />
      <Card>
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Sessions</div>
        <div className="mt-3 space-y-2">
          {snapshot.streaming.sessions.map((session) => (
            <div key={session.id} className="rounded-2xl border border-white/5 bg-black/10 p-3 text-sm">
              <div className="font-medium">{session.user_id}</div>
              <div className="mt-1 text-xs text-muted-foreground">Last seen {formatDateTime(session.last_seen_at)}</div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
