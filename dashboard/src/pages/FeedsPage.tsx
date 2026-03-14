import { FeedMatrix } from "@/components/FeedMatrix";
import { useDashboardStore } from "@/store/useDashboardStore";

export function FeedsPage() {
  const snapshot = useDashboardStore((state) => state.publicSnapshot);
  if (!snapshot) return null;
  return <FeedMatrix feeds={snapshot.feeds} />;
}
