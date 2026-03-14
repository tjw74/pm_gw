import { DepthChart } from "@/components/DepthChart";
import { PriceComparisonChart } from "@/components/PriceComparisonChart";
import { Card } from "@/components/ui/card";
import { useDashboardStore } from "@/store/useDashboardStore";
import { formatDateTime, formatDuration, formatSlugWindowTime } from "@/lib/utils";

export function MarketPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  const market = snapshot.market.active;
  return (
    <div className="space-y-4">
      <Card>
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Active window</div>
        <div className="mt-3 grid gap-3 md:grid-cols-4">
          <Info
            label="Slug"
            value={market?.slug ?? "waiting"}
            detail={formatSlugWindowTime(market?.slug) ?? undefined}
          />
          <Info label="Start" value={formatDateTime(market?.window.window_start)} />
          <Info label="End" value={formatDateTime(market?.window.window_end)} />
          <Info label="Time remaining" value={formatDuration(snapshot.global.window_countdown_seconds)} />
        </div>
      </Card>
      <div className="panel-grid">
        <PriceComparisonChart snapshot={snapshot} />
        <DepthChart orderbook={snapshot.market.orderbook_snapshot} />
      </div>
    </div>
  );
}

function Info({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="rounded-2xl border border-white/5 bg-black/10 p-3">
      <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="mt-2 text-sm font-medium">{value}</div>
      {detail ? <div className="mt-1 text-xs text-muted-foreground">{detail}</div> : null}
    </div>
  );
}
