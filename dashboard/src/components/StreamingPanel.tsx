import { Card } from "@/components/ui/card";
import type { DashboardSnapshot } from "@/lib/types";
import { formatMs } from "@/lib/utils";

export function StreamingPanel({ snapshot }: { snapshot: DashboardSnapshot }) {
  const s = snapshot.streaming;
  return (
    <Card className="xl:col-span-6">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Downstream</div>
          <h2 className="mt-1.5 text-xl font-semibold">Streaming health</h2>
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-5">
        <StreamMetric label="Active clients" value={`${s.active_clients}`} />
        <StreamMetric label="Outbound rate" value={`${s.outbound_messages_per_sec}/s`} />
        <StreamMetric label="Bytes/sec" value={`${s.outbound_bytes_per_sec}`} />
        <StreamMetric label="Command latency" value={formatMs(s.avg_command_latency_ms)} />
        <StreamMetric label="Drops" value={`${s.dropped_messages_total}`} />
      </div>
    </Card>
  );
}

function StreamMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-white/5 bg-black/10 p-3">
      <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="mt-2 text-base font-medium">{value}</div>
    </div>
  );
}
