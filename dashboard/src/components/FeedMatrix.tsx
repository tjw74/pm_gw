import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { FeedItem } from "@/lib/types";
import { formatMs, relativeAgeMs } from "@/lib/utils";

export function FeedMatrix({ feeds }: { feeds: FeedItem[] }) {
  return (
    <Card className="xl:col-span-5">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Upstream matrix</div>
          <h2 className="mt-1.5 text-xl font-semibold">Feed health</h2>
        </div>
      </div>
      <div className="space-y-2">
        {feeds.map((feed) => (
          <div key={feed.adapter} className="rounded-2xl border border-white/5 bg-black/10 p-3">
            <div className="flex items-center justify-between gap-2.5">
              <div>
                <div className="text-sm font-medium">{feed.adapter}</div>
                <div className="text-xs text-muted-foreground">
                  {feed.freshness_expected
                    ? `Last seen ${relativeAgeMs(feed.last_message_age_ms)}`
                    : "Event-driven health"}
                </div>
              </div>
              <Badge className={badgeTone(feed.connection, feed.stale)}>
                {feed.stale ? "stale" : feed.connection}
              </Badge>
            </div>
            <div className="mt-3 grid grid-cols-3 gap-2 text-xs text-muted-foreground">
              <Metric label="Rate" value={`${feed.message_rate_per_sec}/s`} />
              <Metric label="Reconnects" value={`${feed.reconnect_count}`} />
              <Metric label="Lag" value={formatMs(feed.last_latency_ms)} />
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[11px] uppercase tracking-[0.18em]">{label}</div>
      <div className="mt-0.5 text-sm text-foreground">{value}</div>
    </div>
  );
}

function badgeTone(connection: string, stale: boolean) {
  if (connection === "disconnected") return "border-danger/30 bg-danger/10 text-danger";
  if (connection === "degraded" || stale) return "border-warning/30 bg-warning/10 text-warning";
  return "border-success/30 bg-success/10 text-success";
}
