import { AlertRail } from "@/components/AlertRail";
import { DepthChart } from "@/components/DepthChart";
import { HealthStrip } from "@/components/HealthStrip";
import { PriceComparisonChart } from "@/components/PriceComparisonChart";
import { StreamingPanel } from "@/components/StreamingPanel";
import { UpstreamSummaryPanel } from "@/components/UpstreamSummaryPanel";
import { useDashboardStore } from "@/store/useDashboardStore";

export function OverviewPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  return (
    <div className="space-y-4">
      <HealthStrip snapshot={snapshot} />
      <div className="panel-grid">
        <PriceComparisonChart snapshot={snapshot} />
        <DepthChart orderbook={snapshot.market.orderbook_snapshot} className="xl:col-span-5" />
        <UpstreamSummaryPanel feeds={snapshot.feeds} />
        <StreamingPanel snapshot={snapshot} />
        <AlertRail alerts={snapshot.alerts} />
      </div>
    </div>
  );
}
