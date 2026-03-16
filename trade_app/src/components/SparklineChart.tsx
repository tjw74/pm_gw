import { ColorType, LineStyle, createChart, type ISeriesApi, type LineData, type UTCTimestamp } from "lightweight-charts";
import { useEffect, useRef } from "react";

interface SparklineChartProps {
  data: Array<{ time: string; value: number }>;
  color: string;
  className?: string;
  showPriceScale?: boolean;
}

export function SparklineChart({ data, color, className, showPriceScale = false }: SparklineChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const seriesRef = useRef<ISeriesApi<"Line"> | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = createChart(containerRef.current, {
      autoSize: true,
      layout: {
        background: { type: ColorType.Solid, color: "transparent" },
        textColor: "rgba(212, 220, 234, 0.55)",
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { visible: false },
      },
      timeScale: {
        visible: false,
      },
      rightPriceScale: {
        visible: false,
      },
      leftPriceScale: {
        visible: showPriceScale,
        borderVisible: false,
      },
      crosshair: {
        vertLine: { visible: false, style: LineStyle.SparseDotted },
        horzLine: { visible: false, style: LineStyle.SparseDotted },
      },
      handleScale: false,
      handleScroll: false,
    });
    const series = chart.addLineSeries({
      color,
      lineWidth: 2,
      crosshairMarkerVisible: false,
      lastValueVisible: false,
      priceLineVisible: false,
    });
    seriesRef.current = series;
    const observer = new ResizeObserver(() => chart.timeScale().fitContent());
    observer.observe(containerRef.current);
    return () => {
      observer.disconnect();
      chart.remove();
    };
  }, [color, showPriceScale]);

  useEffect(() => {
    const points: LineData[] = data.map((entry) => ({
      time: Math.floor(new Date(entry.time).getTime() / 1000) as UTCTimestamp,
      value: entry.value,
    }));
    seriesRef.current?.setData(points);
  }, [data]);

  return <div ref={containerRef} className={className ?? "h-20 w-full"} />;
}
