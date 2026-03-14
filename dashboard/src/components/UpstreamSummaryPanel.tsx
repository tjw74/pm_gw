import { Card } from "@/components/ui/card";
import type { FeedItem } from "@/lib/types";
import { formatMs } from "@/lib/utils";

export function UpstreamSummaryPanel({ feeds }: { feeds: FeedItem[] }) {
  const connected = feeds.filter((feed) => feed.connection === "connected" && !feed.stale).length;
  const degraded = feeds.filter((feed) => feed.connection === "degraded").length;
  const stale = feeds.filter((feed) => feed.stale).length;
  const disconnected = feeds.filter((feed) => feed.connection === "disconnected").length;
  const totalRate = feeds.reduce((sum, feed) => sum + feed.message_rate_per_sec, 0);
  const reconnects60s = feeds.reduce((sum, feed) => sum + feed.recent_disconnects_60s, 0);
  const worstLag = feeds.reduce<number | null>((current, feed) => {
    if (feed.last_latency_ms == null) return current;
    return current == null ? feed.last_latency_ms : Math.max(current, feed.last_latency_ms);
  }, null);

  return (
    <Card className="xl:col-span-6">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Upstream</div>
          <h2 className="mt-1.5 text-xl font-semibold">Feed health</h2>
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-3 xl:grid-cols-6">
        <Metric label="Connected" value={`${connected}/${feeds.length}`} tone="healthy" />
        <Metric label="Degraded" value={`${degraded}`} tone={degraded > 0 ? "warning" : "neutral"} />
        <Metric label="Stale" value={`${stale}`} tone={stale > 0 ? "warning" : "neutral"} />
        <Metric label="Disconnected" value={`${disconnected}`} tone={disconnected > 0 ? "critical" : "neutral"} />
        <Metric label="Msg rate" value={`${totalRate}/s`} tone="neutral" />
        <Metric label="Worst lag" value={formatMs(worstLag)} tone="neutral" />
      </div>
      <div className="mt-3 grid gap-3 md:grid-cols-2">
        <div className="rounded-2xl border border-white/5 bg-black/10 p-3">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Reconnect pressure</div>
          <div className="mt-2 text-base font-medium">{reconnects60s} events / 60s</div>
        </div>
        <div className="rounded-2xl border border-white/5 bg-black/10 p-3">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Detail view</div>
          <div className="mt-2 text-sm text-muted-foreground">
            Open <span className="text-foreground">Feeds</span> for per-adapter freshness, reconnect history, and latency detail.
          </div>
        </div>
      </div>
    </Card>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "neutral" | "healthy" | "warning" | "critical";
}) {
  const toneClass =
    tone === "healthy"
      ? "border-success/20 bg-success/10"
      : tone === "warning"
        ? "border-warning/20 bg-warning/10"
        : tone === "critical"
          ? "border-danger/20 bg-danger/10"
          : "border-white/5 bg-black/10";

  return (
    <div className={`rounded-2xl border p-3 ${toneClass}`}>
      <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="mt-2 text-base font-medium">{value}</div>
    </div>
  );
}
