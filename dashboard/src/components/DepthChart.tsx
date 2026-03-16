import { useMemo } from "react";

import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { DashboardSnapshot } from "@/lib/types";

type OrderbookLevel = [number, number];
type OrderbookSide = OrderbookLevel[];

type LadderBucket = {
  price: number;
  totalSize: number;
};

type DistributionSeries = {
  bids: LadderBucket[];
  asks: LadderBucket[];
  bestBid: number | null;
  bestAsk: number | null;
  midPrice: number | null;
  maxDistanceFromMid: number;
  maxSize: number;
  bidLevels: number;
  askLevels: number;
};

const VIEWBOX_WIDTH = 720;
const VIEWBOX_HEIGHT = 260;
const MARGIN = { top: 12, right: 20, bottom: 40, left: 22 };
const PLOT_WIDTH = VIEWBOX_WIDTH - MARGIN.left - MARGIN.right;
const PLOT_HEIGHT = VIEWBOX_HEIGHT - MARGIN.top - MARGIN.bottom;
const LADDER_BUCKET_SIZE = 0.01;
const LADDER_BUCKET_ROWS = 10;
const CHART_BUCKET_ROWS = 28;

function normalizeSide(raw: unknown): OrderbookSide {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((level) => {
      if (Array.isArray(level) && level.length >= 2) {
        return [Number(level[0]), Number(level[1])] as OrderbookLevel;
      }
      if (typeof level === "object" && level) {
        const item = level as Record<string, unknown>;
        return [Number(item.price), Number(item.size ?? item.quantity ?? item.s)] as OrderbookLevel;
      }
      return undefined;
    })
    .filter((item): item is OrderbookLevel => {
      return Boolean(item && Number.isFinite(item[0]) && Number.isFinite(item[1]) && item[1] > 0);
    });
}

function extractBook(orderbook: unknown) {
  const root = (orderbook ?? {}) as Record<string, unknown>;
  const nested = typeof root.data === "object" && root.data ? (root.data as Record<string, unknown>) : root;
  const bids = normalizeSide(root.bids ?? nested.bids ?? root.buy ?? nested.buy);
  const asks = normalizeSide(root.asks ?? nested.asks ?? root.sell ?? nested.sell);
  return { bids, asks };
}

function formatVolume(value: number | null) {
  if (value == null || !Number.isFinite(value)) return "--";
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toFixed(2);
}

function roundBucket(value: number) {
  return Math.round(value * 1000) / 1000;
}

function aggregateBuckets(
  levels: OrderbookSide,
  side: "bids" | "asks",
  bucketSize = LADDER_BUCKET_SIZE,
  limit = LADDER_BUCKET_ROWS,
) {
  const buckets = new Map<number, number>();

  for (const [price, size] of levels) {
    const bucket =
      side === "bids"
        ? roundBucket(Math.floor(price / bucketSize) * bucketSize)
        : roundBucket(Math.ceil(price / bucketSize) * bucketSize);
    buckets.set(bucket, (buckets.get(bucket) ?? 0) + size);
  }

  return [...buckets.entries()]
    .map(([price, totalSize]) => ({ price, totalSize }))
    .sort((a, b) => (side === "bids" ? b.price - a.price : a.price - b.price))
    .slice(0, limit);
}

function buildDepthSeries(orderbook: unknown): DistributionSeries {
  const { bids, asks } = extractBook(orderbook);
  const bidBuckets = aggregateBuckets(bids, "bids", LADDER_BUCKET_SIZE, CHART_BUCKET_ROWS);
  const askBuckets = aggregateBuckets(asks, "asks", LADDER_BUCKET_SIZE, CHART_BUCKET_ROWS);
  const bestBid = bidBuckets[0]?.price ?? null;
  const bestAsk = askBuckets[0]?.price ?? null;
  const midPrice =
    bestBid != null && bestAsk != null ? (bestBid + bestAsk) / 2 : null;
  const maxDistanceFromMid = midPrice == null
    ? 1
    : Math.max(
        midPrice - (bidBuckets[bidBuckets.length - 1]?.price ?? bestBid ?? midPrice),
        (askBuckets[askBuckets.length - 1]?.price ?? bestAsk ?? midPrice) - midPrice,
        (midPrice - bestBid),
        (bestAsk - midPrice),
        1e-6,
      );
  const maxSize = Math.max(
    ...bidBuckets.map((bucket) => bucket.totalSize),
    ...askBuckets.map((bucket) => bucket.totalSize),
    1,
  );

  return {
    bids: bidBuckets,
    asks: askBuckets,
    bestBid,
    bestAsk,
    midPrice,
    maxDistanceFromMid,
    maxSize,
    bidLevels: bids.length,
    askLevels: asks.length,
  };
}

export function DepthChart({
  orderbook,
  market,
  className,
}: {
  orderbook: unknown;
  market?: DashboardSnapshot["market"];
  className?: string;
}) {
  const series = useMemo(() => buildDepthSeries(orderbook), [orderbook]);
  const { bids: rawBids, asks: rawAsks } = useMemo(() => extractBook(orderbook), [orderbook]);
  const orderbookRoot = (orderbook ?? {}) as Record<string, unknown>;
  const bookAssetId = typeof orderbookRoot.asset_id === "string" ? orderbookRoot.asset_id : null;
  const hasBook = series.bids.length > 0 && series.asks.length > 0;
  const bidBuckets = useMemo(() => aggregateBuckets(rawBids, "bids"), [rawBids]);
  const askBuckets = useMemo(() => aggregateBuckets(rawAsks, "asks"), [rawAsks]);
  const maxBucketSize = Math.max(
    ...bidBuckets.map((bucket) => bucket.totalSize),
    ...askBuckets.map((bucket) => bucket.totalSize),
    1,
  );

  const centerX = MARGIN.left + PLOT_WIDTH / 2;
  const halfWidth = PLOT_WIDTH / 2;
  const projectX = (price: number) => {
    if (series.midPrice == null) {
      return centerX;
    }
    const distance = price - series.midPrice;
    const normalized = Math.min(Math.abs(distance) / Math.max(series.maxDistanceFromMid, 1e-9), 1);
    if (distance < 0) {
      return centerX - normalized * halfWidth;
    }
    return centerX + normalized * halfWidth;
  };
  const projectY = (size: number) =>
    MARGIN.top + PLOT_HEIGHT - (size / Math.max(series.maxSize, 1e-9)) * PLOT_HEIGHT;
  const barWidth = Math.max((halfWidth / CHART_BUCKET_ROWS) * 0.8, 4);

  const spread =
    series.bestBid != null && series.bestAsk != null ? Math.max(series.bestAsk - series.bestBid, 0) : null;
  const mid = series.midPrice;
  const bidDepth = series.bids.reduce((sum, bucket) => sum + bucket.totalSize, 0) || null;
  const askDepth = series.asks.reduce((sum, bucket) => sum + bucket.totalSize, 0) || null;
  const imbalance =
    bidDepth != null && askDepth != null && bidDepth + askDepth > 0 ? bidDepth / (bidDepth + askDepth) : null;

  const yTicks = [0.25, 0.5, 0.75, 1].map((ratio) => ({
    y: MARGIN.top + PLOT_HEIGHT - PLOT_HEIGHT * ratio,
    value: series.maxSize * ratio,
  }));

  const sharePrices = useMemo(() => {
    const marketPrice = market?.latest_market_price;
    const active = market?.active;
    if (marketPrice == null || !Number.isFinite(marketPrice)) return null;

    const yesPrice =
      bookAssetId && active?.yes_token_id && bookAssetId === active.yes_token_id ? marketPrice
      : bookAssetId && active?.no_token_id && bookAssetId === active.no_token_id ? 1 - marketPrice
      : marketPrice;
    const noPrice = 1 - yesPrice;

    return {
      yes: yesPrice,
      no: noPrice,
    };
  }, [bookAssetId, market]);

  return (
    <Card className={cn("xl:col-span-7", className)}>
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Microstructure</div>
          <h2 className="mt-1.5 text-xl font-semibold">Order book depth</h2>
        </div>
        <div className="text-right text-xs text-muted-foreground">
          {hasBook ? `Resting order distribution across ${series.bidLevels + series.askLevels} levels` : "Awaiting Polymarket depth snapshot"}
        </div>
      </div>
      {sharePrices ? (
        <div className="mb-3 grid gap-2 md:grid-cols-2">
          <SharePriceCard label="YES" value={sharePrices.yes} tone="yes" />
          <SharePriceCard label="NO" value={sharePrices.no} tone="no" />
        </div>
      ) : null}
      {hasBook ? (
        <svg viewBox={`0 0 ${VIEWBOX_WIDTH} ${VIEWBOX_HEIGHT}`} className="h-[220px] w-full">
          {yTicks.map((tick) => (
            <g key={tick.value}>
              <line
                x1={MARGIN.left}
                x2={VIEWBOX_WIDTH - MARGIN.right}
                y1={tick.y}
                y2={tick.y}
                stroke="rgba(255,255,255,0.06)"
                strokeDasharray="3 6"
              />
              <text x={MARGIN.left - 8} y={tick.y + 3} fill="rgba(255,255,255,0.55)" fontSize="11" textAnchor="end">
                {formatVolume(tick.value)}
              </text>
            </g>
          ))}

          {mid != null ? (
            <line
              x1={centerX}
              x2={centerX}
              y1={MARGIN.top}
              y2={VIEWBOX_HEIGHT - MARGIN.bottom}
              stroke="rgba(255,255,255,0.18)"
              strokeDasharray="4 6"
            />
          ) : null}

          {series.bids.map((bucket) => {
            const x = projectX(bucket.price) - barWidth;
            const y = projectY(bucket.totalSize);
            const height = VIEWBOX_HEIGHT - MARGIN.bottom - y;
            return (
              <rect
                key={`bid-bar-${bucket.price}`}
                x={x}
                y={y}
                width={barWidth}
                height={Math.max(height, 1)}
                rx="2"
                fill="rgba(34,197,94,0.7)"
              />
            );
          })}
          {series.asks.map((bucket) => {
            const x = projectX(bucket.price);
            const y = projectY(bucket.totalSize);
            const height = VIEWBOX_HEIGHT - MARGIN.bottom - y;
            return (
              <rect
                key={`ask-bar-${bucket.price}`}
                x={x}
                y={y}
                width={barWidth}
                height={Math.max(height, 1)}
                rx="2"
                fill="rgba(251,113,133,0.7)"
              />
            );
          })}

          <text
            x={series.bestBid != null ? projectX(series.bestBid) : MARGIN.left}
            y={VIEWBOX_HEIGHT - 12}
            fill="rgba(255,255,255,0.7)"
            fontSize="12"
            textAnchor="end"
          >
            {series.bestBid?.toFixed(4) ?? "--"}
          </text>
          <text
            x={centerX}
            y={VIEWBOX_HEIGHT - 12}
            fill="rgba(255,255,255,0.55)"
            fontSize="12"
            textAnchor="middle"
          >
            {mid != null ? `Mid ${mid.toFixed(4)}` : "Mid"}
          </text>
          <text
            x={series.bestAsk != null ? projectX(series.bestAsk) : VIEWBOX_WIDTH - MARGIN.right}
            y={VIEWBOX_HEIGHT - 12}
            fill="rgba(255,255,255,0.7)"
            fontSize="12"
            textAnchor="start"
          >
            {series.bestAsk?.toFixed(4) ?? "--"}
          </text>
        </svg>
      ) : (
        <div className="flex h-[220px] flex-col items-center justify-center rounded-3xl border border-dashed border-white/10 bg-black/10 px-4 text-center">
          <div className="text-sm font-medium">No live order book yet</div>
          <div className="mt-1.5 max-w-sm text-xs text-muted-foreground">
            The gateway is healthy, but no usable Polymarket bid/ask ladder has been received for the active market window.
          </div>
        </div>
      )}
      {hasBook ? (
        <div className="mt-3 grid gap-2 lg:grid-cols-2">
          <LadderColumn
            title={`Bid ladder (${(LADDER_BUCKET_SIZE * 100).toFixed(0)}c buckets)`}
            side="bids"
            buckets={bidBuckets}
            maxBucketSize={maxBucketSize}
          />
          <LadderColumn
            title={`Ask ladder (${(LADDER_BUCKET_SIZE * 100).toFixed(0)}c buckets)`}
            side="asks"
            buckets={askBuckets}
            maxBucketSize={maxBucketSize}
          />
        </div>
      ) : null}
      <div className="mt-3 grid grid-cols-2 gap-2 text-sm text-muted-foreground md:grid-cols-6">
        <DepthMetric label="Best bid" value={series.bestBid?.toFixed(4) ?? "--"} />
        <DepthMetric label="Best ask" value={series.bestAsk?.toFixed(4) ?? "--"} />
        <DepthMetric label="Spread" value={spread != null ? spread.toFixed(4) : "--"} />
        <DepthMetric label="Bid depth" value={formatVolume(bidDepth)} />
        <DepthMetric label="Ask depth" value={formatVolume(askDepth)} />
        <DepthMetric label="Bid imbalance" value={imbalance != null ? `${Math.round(imbalance * 100)}%` : "--"} />
      </div>
    </Card>
  );
}

function SharePriceCard({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "yes" | "no";
}) {
  const toneClass = tone === "yes" ? "border-emerald-400/20 bg-emerald-500/10" : "border-rose-400/20 bg-rose-500/10";
  return (
    <div className={`flex items-center justify-between rounded-lg border px-2.5 py-1 ${toneClass}`}>
      <div className="text-[10px] uppercase tracking-[0.18em] text-muted-foreground">{label}</div>
      <div className="text-sm font-semibold leading-none text-foreground">{Math.round(value * 100)}c</div>
    </div>
  );
}

function DepthMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-white/5 bg-black/10 p-2.5">
      <div className="text-[11px] uppercase tracking-[0.18em]">{label}</div>
      <div className="mt-1.5 text-sm text-foreground">{value}</div>
    </div>
  );
}

function LadderColumn({
  title,
  side,
  buckets,
  maxBucketSize,
}: {
  title: string;
  side: "bids" | "asks";
  buckets: LadderBucket[];
  maxBucketSize: number;
}) {
  const fillClass = side === "bids" ? "bg-emerald-500/70" : "bg-rose-500/70";
  const railClass = side === "bids" ? "bg-emerald-500/8" : "bg-rose-500/8";

  return (
    <div className="rounded-2xl border border-white/5 bg-black/10 p-2.5">
      <div className="mb-2 text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{title}</div>
      <div className="space-y-1.5">
        {buckets.map((bucket) => {
          const width = `${Math.max((bucket.totalSize / maxBucketSize) * 100, 4)}%`;
          return (
            <div key={`${side}-${bucket.price}`} className="grid grid-cols-[56px_1fr] items-center gap-2 text-xs">
              <div className="text-muted-foreground">{bucket.price.toFixed(2)}</div>
              <div className={`relative h-6 overflow-hidden rounded-md border border-white/5 ${railClass}`}>
                <div className={`absolute inset-y-0 left-0 ${fillClass}`} style={{ width }} />
                <div className="absolute inset-0 flex items-center justify-end px-2 text-foreground">
                  {formatVolume(bucket.totalSize)}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
