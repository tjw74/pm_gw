import { AlertRail } from "@/components/AlertRail";
import { DepthChart } from "@/components/DepthChart";
import { FeedMatrix } from "@/components/FeedMatrix";
import { HealthStrip } from "@/components/HealthStrip";
import { PriceComparisonChart } from "@/components/PriceComparisonChart";
import { StreamingPanel } from "@/components/StreamingPanel";
import { useDashboardStore } from "@/store/useDashboardStore";

export function OverviewPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  return (
    <div className="space-y-4">
      <HealthStrip snapshot={snapshot} />
      <div className="panel-grid">
        <FeedMatrix feeds={snapshot.feeds} />
        <AlertRail alerts={snapshot.alerts} />
        <PriceComparisonChart snapshot={snapshot} />
        <DepthChart orderbook={snapshot.market.orderbook_snapshot} />
        <StreamingPanel snapshot={snapshot} />
      </div>
    </div>
  );
}
