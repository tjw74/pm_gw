import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { formatDuration, formatSlugWindowTime } from "@/lib/utils";
import type { DashboardSnapshot } from "@/lib/types";

export function HealthStrip({ snapshot }: { snapshot: DashboardSnapshot }) {
  const global = snapshot.global;
  const meta = snapshot.meta;
  return (
    <Card className="overflow-hidden p-0">
      <div className="grid gap-0 md:grid-cols-6">
        <StripCell label="Gateway" value={global.gateway.toUpperCase()} tone={global.overall_health} />
        <StripCell label="Build" value={`${meta.version} · ${meta.commit}`} />
        <StripCell
          label="Market"
          value={global.active_market_slug ?? "waiting"}
          detail={formatSlugWindowTime(global.active_market_slug) ?? undefined}
        />
        <StripCell label="Rollover" value={formatDuration(global.window_countdown_seconds)} />
        <StripCell label="Upstreams" value={`${global.upstreams_connected}/${global.upstreams_total}`} />
        <StripCell label="Clients" value={`${global.downstream_clients}`} />
      </div>
    </Card>
  );
}

function StripCell({
  label,
  value,
  detail,
  tone = "healthy",
}: {
  label: string;
  value: string;
  detail?: string;
  tone?: "healthy" | "warning" | "critical";
}) {
  const badgeTone =
    tone === "critical" ? "text-danger border-danger/30 bg-danger/10" :
    tone === "warning" ? "text-warning border-warning/30 bg-warning/10" :
    "text-success border-success/30 bg-success/10";
  return (
    <div className="border-b border-white/5 p-3.5 md:border-b-0 md:border-r last:border-r-0">
      <div className="mb-2 text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="flex items-center gap-2.5">
        <div>
          <div className="text-sm font-semibold leading-tight md:text-base">{value}</div>
          {detail ? <div className="mt-0.5 text-[11px] text-muted-foreground">{detail}</div> : null}
        </div>
        {label === "Gateway" ? <Badge className={badgeTone}>{tone}</Badge> : null}
      </div>
    </div>
  );
}
