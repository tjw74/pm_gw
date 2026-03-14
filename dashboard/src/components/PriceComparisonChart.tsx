import { useEffect, useMemo, useRef } from "react";
import { ColorType, LineStyle, UTCTimestamp, createChart } from "lightweight-charts";
import { Card } from "@/components/ui/card";
import type { DashboardSnapshot } from "@/lib/types";
import { cn, relativeAgeMs } from "@/lib/utils";

const palette = ["#f7931a", "#38bdf8", "#22c55e", "#f59e0b", "#fb7185", "#a78bfa", "#34d399"];

type SeriesMeta = {
  key: string;
  color: string;
  latestValue: number;
  lastAgeMs: number | null;
  points: Array<{ time: UTCTimestamp; value: number }>;
};

export function PriceComparisonChart({
  snapshot,
  className,
}: {
  snapshot: DashboardSnapshot;
  className?: string;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const seriesData = useMemo<SeriesMeta[]>(() => {
    const feedAgeByAdapter = new Map(snapshot.feeds.map((feed) => [feed.adapter, feed.last_message_age_ms]));
    return Object.entries(snapshot.market.price_history)
      .map(([key, points], index): SeriesMeta | null => {
        const buckets = new Map<number, number>();
        points.forEach((point) => {
          const time = Math.floor(new Date(point.timestamp).getTime() / 1000);
          if (Number.isFinite(time)) {
            buckets.set(time, point.value);
          }
        });
        const parsed = [...buckets.entries()]
          .sort((a, b) => a[0] - b[0])
          .map((point) => {
            return { time: point[0] as UTCTimestamp, value: point[1] };
          })
          .filter((point) => Number.isFinite(point.value));
        if (!parsed.length) return null;
        return {
          key,
          color: palette[index % palette.length],
          latestValue: parsed.at(-1)?.value ?? 0,
          lastAgeMs: feedAgeByAdapter.get(key.replace("polymarket_clob", "polymarket_clob_market")) ?? feedAgeByAdapter.get(key) ?? null,
          points: parsed,
        };
      })
      .filter((entry): entry is SeriesMeta => entry !== null)
      .sort((a, b) => a.key.localeCompare(b.key));
  }, [snapshot.feeds, snapshot.market.price_history]);

  const anchor = useMemo(() => {
    if (!seriesData.length) return null;
    const values = seriesData.map((entry) => entry.latestValue).sort((a, b) => a - b);
    return values[Math.floor(values.length / 2)] ?? null;
  }, [seriesData]);

  const divergence = useMemo(() => {
    if (seriesData.length < 2) return null;
    const values = seriesData.map((entry) => entry.latestValue);
    return Math.max(...values) - Math.min(...values);
  }, [seriesData]);

  useEffect(() => {
    if (!containerRef.current || !seriesData.length) return;
    const chart = createChart(containerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: "transparent" },
        textColor: "#7f8da3",
      },
      grid: {
        horzLines: { color: "rgba(255,255,255,0.05)" },
        vertLines: { color: "rgba(255,255,255,0.03)" },
      },
      rightPriceScale: { borderVisible: false },
      timeScale: { borderVisible: false, timeVisible: true, secondsVisible: false },
      crosshair: { vertLine: { style: LineStyle.Dashed }, horzLine: { style: LineStyle.Dashed } },
      width: containerRef.current.clientWidth,
      height: 280,
    });

    seriesData.forEach((entry) => {
      const series = chart.addLineSeries({
        color: entry.color,
        lineWidth: entry.key === "polymarket_clob" ? 3 : 2,
        priceLineVisible: false,
        lastValueVisible: false,
      });
      series.setData(
        entry.points.map((point) => ({
          time: point.time,
          value: anchor ? ((point.value - anchor) / anchor) * 10_000 : point.value,
        })),
      );
    });
    chart.timeScale().fitContent();

    const resizeObserver = new ResizeObserver(() => {
      chart.applyOptions({ width: containerRef.current?.clientWidth ?? 600 });
      chart.timeScale().fitContent();
    });
    resizeObserver.observe(containerRef.current);
    return () => {
      resizeObserver.disconnect();
      chart.remove();
    };
  }, [anchor, seriesData]);

  return (
    <Card className={cn("xl:col-span-7", className)}>
      <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Price alignment</div>
          <h2 className="mt-1.5 text-xl font-semibold">Source comparison</h2>
          <div className="mt-1 text-xs text-muted-foreground">Relative basis-point offset versus the live cross-source median.</div>
        </div>
        <div className="rounded-2xl border border-white/5 bg-black/10 px-3 py-2 text-right">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Divergence</div>
          <div className="mt-1.5 text-base font-semibold">{divergence == null ? "--" : divergence.toFixed(2)}</div>
        </div>
      </div>
      {seriesData.length ? (
        <div ref={containerRef} />
      ) : (
          <div className="flex h-[240px] items-center justify-center rounded-3xl border border-dashed border-white/10 bg-black/10 text-sm text-muted-foreground">
          Waiting for valid price history from upstream sources.
        </div>
      )}
      {anchor ? (
        <div className="mt-3 space-y-2 rounded-3xl border border-white/5 bg-black/10 p-3">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Alignment rail</div>
          {seriesData.map((entry) => {
            const deltaBps = ((entry.latestValue - anchor) / anchor) * 10_000;
            const widthPct = Math.min(Math.abs(deltaBps) * 6, 100);
            return (
              <div key={`${entry.key}-rail`} className="space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <div className="flex items-center gap-2">
                    <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: entry.color }} />
                    <span>{entry.key}</span>
                  </div>
                  <span className={deltaBps >= 0 ? "text-success" : "text-danger"}>
                    {deltaBps >= 0 ? "+" : ""}
                    {deltaBps.toFixed(2)} bps
                  </span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-white/5">
                  <div
                    className="h-full rounded-full"
                    style={{
                      width: `${Math.max(widthPct, 4)}%`,
                      backgroundColor: entry.color,
                      opacity: 0.85,
                    }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      ) : null}
      <div className="mt-3 grid gap-2 md:grid-cols-2 xl:grid-cols-3">
        {seriesData.map((entry) => (
          <div key={entry.key} className="rounded-2xl border border-white/5 bg-black/10 px-3 py-2.5">
            <div className="flex items-center gap-2 text-xs">
              <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: entry.color }} />
              <span className="font-medium">{entry.key}</span>
            </div>
            <div className="mt-1.5 text-base font-semibold">{entry.latestValue.toFixed(2)}</div>
            <div className="text-xs text-muted-foreground">Updated {relativeAgeMs(entry.lastAgeMs)}</div>
          </div>
        ))}
      </div>
    </Card>
  );
}
