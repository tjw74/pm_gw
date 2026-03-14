import { useMemo } from "react";

import { Card } from "@/components/ui/card";

type OrderbookLevel = [number, number];
type OrderbookSide = OrderbookLevel[];

type DepthPoint = {
  price: number;
  cumulative: number;
};

type LadderBucket = {
  price: number;
  totalSize: number;
};

type DepthSeries = {
  bids: DepthPoint[];
  asks: DepthPoint[];
  bestBid: number | null;
  bestAsk: number | null;
  midPrice: number | null;
  maxDistanceFromMid: number;
  maxDepth: number;
  bidLevels: number;
  askLevels: number;
};

const VIEWBOX_WIDTH = 720;
const VIEWBOX_HEIGHT = 260;
const MARGIN = { top: 12, right: 10, bottom: 28, left: 10 };
const PLOT_WIDTH = VIEWBOX_WIDTH - MARGIN.left - MARGIN.right;
const PLOT_HEIGHT = VIEWBOX_HEIGHT - MARGIN.top - MARGIN.bottom;
const MAX_RENDER_POINTS_PER_SIDE = 220;
const LADDER_BUCKET_SIZE = 0.01;
const LADDER_BUCKET_ROWS = 10;

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

function buildCumulativeBids(levels: OrderbookSide): DepthPoint[] {
  const sorted = [...levels].sort((a, b) => b[0] - a[0]);
  let cumulative = 0;
  return sorted.map(([price, size]) => {
    cumulative += size;
    return { price, cumulative };
  });
}

function buildCumulativeAsks(levels: OrderbookSide): DepthPoint[] {
  const sorted = [...levels].sort((a, b) => a[0] - b[0]);
  let cumulative = 0;
  return sorted.map(([price, size]) => {
    cumulative += size;
    return { price, cumulative };
  });
}

function compressDepthPoints(
  points: DepthPoint[],
  projectX: (price: number) => number,
  side: "bids" | "asks",
) {
  if (points.length <= MAX_RENDER_POINTS_PER_SIDE) {
    return points;
  }

  const compressed: DepthPoint[] = [];
  let lastBucket: number | null = null;

  for (const point of points) {
    const x = projectX(point.price);
    const bucket = Math.round((x / PLOT_WIDTH) * MAX_RENDER_POINTS_PER_SIDE);
    if (bucket === lastBucket) {
      compressed[compressed.length - 1] = point;
      continue;
    }
    compressed.push(point);
    lastBucket = bucket;
  }

  const first = points[0];
  const last = points[points.length - 1];
  if (compressed[0]?.price !== first.price) {
    compressed.unshift(first);
  }
  if (compressed[compressed.length - 1]?.price !== last.price) {
    compressed.push(last);
  }

  return side === "bids" ? compressed.sort((a, b) => b.price - a.price) : compressed.sort((a, b) => a.price - b.price);
}

function buildStepAreaPath(
  points: DepthPoint[],
  projectX: (price: number) => number,
  projectY: (depth: number) => number,
) {
  if (!points.length) return "";

  const first = points[0];
  let path = `M ${projectX(first.price)} ${VIEWBOX_HEIGHT - MARGIN.bottom} L ${projectX(first.price)} ${projectY(first.cumulative)}`;
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    path += ` H ${projectX(point.price)} V ${projectY(point.cumulative)}`;
  }
  const last = points[points.length - 1];
  path += ` L ${projectX(last.price)} ${VIEWBOX_HEIGHT - MARGIN.bottom} Z`;
  return path;
}

function buildStepLinePath(
  points: DepthPoint[],
  projectX: (price: number) => number,
  projectY: (depth: number) => number,
) {
  if (!points.length) return "";

  const first = points[0];
  let path = `M ${projectX(first.price)} ${projectY(first.cumulative)}`;
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    path += ` H ${projectX(point.price)} V ${projectY(point.cumulative)}`;
  }
  return path;
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
    .slice(0, LADDER_BUCKET_ROWS);
}

function buildDepthSeries(orderbook: unknown): DepthSeries {
  const { bids, asks } = extractBook(orderbook);
  const bidCurve = buildCumulativeBids(bids);
  const askCurve = buildCumulativeAsks(asks);
  const bestBid = bidCurve[0]?.price ?? null;
  const bestAsk = askCurve[0]?.price ?? null;
  const midPrice =
    bestBid != null && bestAsk != null ? (bestBid + bestAsk) / 2 : null;
  const maxDistanceFromMid = midPrice == null
    ? 1
    : Math.max(
        midPrice - (bidCurve[bidCurve.length - 1]?.price ?? bestBid ?? midPrice),
        (askCurve[askCurve.length - 1]?.price ?? bestAsk ?? midPrice) - midPrice,
        (midPrice - bestBid),
        (bestAsk - midPrice),
        1e-6,
      );
  const maxDepth = Math.max(
    bidCurve[bidCurve.length - 1]?.cumulative ?? 0,
    askCurve[askCurve.length - 1]?.cumulative ?? 0,
    1,
  );
  const centerX = MARGIN.left + PLOT_WIDTH / 2;
  const halfWidth = PLOT_WIDTH / 2;
  const projectX = (price: number) => {
    if (midPrice == null) {
      return centerX;
    }
    const distance = price - midPrice;
    const normalized = Math.min(Math.abs(distance) / maxDistanceFromMid, 1);
    if (distance < 0) {
      return centerX - normalized * halfWidth;
    }
    return centerX + normalized * halfWidth;
  };

  return {
    bids: compressDepthPoints(bidCurve, projectX, "bids"),
    asks: compressDepthPoints(askCurve, projectX, "asks"),
    bestBid,
    bestAsk,
    midPrice,
    maxDistanceFromMid,
    maxDepth,
    bidLevels: bidCurve.length,
    askLevels: askCurve.length,
  };
}

export function DepthChart({ orderbook }: { orderbook: unknown }) {
  const series = useMemo(() => buildDepthSeries(orderbook), [orderbook]);
  const { bids: rawBids, asks: rawAsks } = useMemo(() => extractBook(orderbook), [orderbook]);
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
  const projectY = (depth: number) =>
    MARGIN.top + PLOT_HEIGHT - (depth / Math.max(series.maxDepth, 1e-9)) * PLOT_HEIGHT;

  const bidAreaPath = hasBook ? buildStepAreaPath(series.bids, projectX, projectY) : "";
  const askAreaPath = hasBook ? buildStepAreaPath(series.asks, projectX, projectY) : "";
  const bidLinePath = hasBook ? buildStepLinePath(series.bids, projectX, projectY) : "";
  const askLinePath = hasBook ? buildStepLinePath(series.asks, projectX, projectY) : "";

  const spread =
    series.bestBid != null && series.bestAsk != null ? Math.max(series.bestAsk - series.bestBid, 0) : null;
  const mid = series.midPrice;
  const bidDepth = series.bids[series.bids.length - 1]?.cumulative ?? null;
  const askDepth = series.asks[series.asks.length - 1]?.cumulative ?? null;
  const imbalance =
    bidDepth != null && askDepth != null && bidDepth + askDepth > 0 ? bidDepth / (bidDepth + askDepth) : null;

  const yTicks = [0.25, 0.5, 0.75, 1].map((ratio) => ({
    y: MARGIN.top + PLOT_HEIGHT - PLOT_HEIGHT * ratio,
    value: series.maxDepth * ratio,
  }));

  return (
    <Card className="xl:col-span-7">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Microstructure</div>
          <h2 className="mt-1.5 text-xl font-semibold">Order book depth</h2>
        </div>
        <div className="text-right text-xs text-muted-foreground">
          {hasBook ? `Cumulative depth across ${series.bidLevels + series.askLevels} levels` : "Awaiting Polymarket depth snapshot"}
        </div>
      </div>
      {hasBook ? (
        <svg viewBox={`0 0 ${VIEWBOX_WIDTH} ${VIEWBOX_HEIGHT}`} className="h-[220px] w-full">
          <defs>
            <linearGradient id="bidFill" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="#22c55e" stopOpacity="0.38" />
              <stop offset="100%" stopColor="#22c55e" stopOpacity="0.04" />
            </linearGradient>
            <linearGradient id="askFill" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="#fb7185" stopOpacity="0.38" />
              <stop offset="100%" stopColor="#fb7185" stopOpacity="0.04" />
            </linearGradient>
          </defs>

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
              <text x={MARGIN.left + 4} y={tick.y - 4} fill="rgba(255,255,255,0.45)" fontSize="10">
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

          <path d={bidAreaPath} fill="url(#bidFill)" />
          <path d={askAreaPath} fill="url(#askFill)" />
          <path d={bidLinePath} fill="none" stroke="#22c55e" strokeWidth="2.2" strokeLinejoin="round" />
          <path d={askLinePath} fill="none" stroke="#fb7185" strokeWidth="2.2" strokeLinejoin="round" />

          <text
            x={series.bestBid != null ? projectX(series.bestBid) : MARGIN.left}
            y={VIEWBOX_HEIGHT - 8}
            fill="rgba(255,255,255,0.7)"
            fontSize="10"
            textAnchor="end"
          >
            {series.bestBid?.toFixed(4) ?? "--"}
          </text>
          <text
            x={centerX}
            y={VIEWBOX_HEIGHT - 8}
            fill="rgba(255,255,255,0.55)"
            fontSize="10"
            textAnchor="middle"
          >
            {mid != null ? `Mid ${mid.toFixed(4)}` : "Mid"}
          </text>
          <text
            x={series.bestAsk != null ? projectX(series.bestAsk) : VIEWBOX_WIDTH - MARGIN.right}
            y={VIEWBOX_HEIGHT - 8}
            fill="rgba(255,255,255,0.7)"
            fontSize="10"
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
  const fillClass =
    side === "bids" ? "bg-emerald-500/18 border-emerald-400/20" : "bg-rose-500/18 border-rose-400/20";

  return (
    <div className="rounded-2xl border border-white/5 bg-black/10 p-2.5">
      <div className="mb-2 text-[11px] uppercase tracking-[0.18em] text-muted-foreground">{title}</div>
      <div className="space-y-1.5">
        {buckets.map((bucket) => {
          const width = `${Math.max((bucket.totalSize / maxBucketSize) * 100, 4)}%`;
          return (
            <div key={`${side}-${bucket.price}`} className="grid grid-cols-[72px_1fr_72px] items-center gap-2 text-xs">
              <div className="text-muted-foreground">{bucket.price.toFixed(2)}</div>
              <div className="relative h-6 overflow-hidden rounded-xl border border-white/5 bg-white/[0.03]">
                <div className={`absolute inset-y-0 ${side === "bids" ? "right-0" : "left-0"} rounded-xl border ${fillClass}`} style={{ width }} />
              </div>
              <div className="text-right text-foreground">{formatVolume(bucket.totalSize)}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
