import { Activity, LogOut, RefreshCcw, WalletCards } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { SparklineChart } from "@/components/SparklineChart";
import { TradeLogin } from "@/components/TradeLogin";
import { Button } from "@/components/ui/Button";
import type { OrderType, Outcome, SizeType, TradeSide } from "@/lib/types";
import {
  cn,
  estimateWindowCountdown,
  formatCompactNumber,
  formatCountdown,
  formatCurrency,
  formatDateTime,
  formatSharePrice,
  formatSignedCurrency,
  median,
  parseBookLevels,
} from "@/lib/utils";
import { useTradeStore } from "@/store/useTradeStore";

const pageIds = ["trade", "portfolio"] as const;

function App() {
  const {
    token,
    snapshot,
    loading,
    sending,
    connected,
    syncState,
    authError,
    activePage,
    bootstrapFromStorage,
    login,
    logout,
    setActivePage,
    placeOrder,
    cancelOrder,
    cancelAll,
    refresh,
  } = useTradeStore();
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [orderType, setOrderType] = useState<OrderType>("market");
  const [side, setSide] = useState<TradeSide>("buy");
  const [outcome, setOutcome] = useState<Outcome>("up");
  const [sizeType, setSizeType] = useState<SizeType>("shares");
  const [size, setSize] = useState("10");
  const [price, setPrice] = useState("0.50");
  const [countdown, setCountdown] = useState<number | null>(null);

  useEffect(() => {
    bootstrapFromStorage();
  }, [bootstrapFromStorage]);

  useEffect(() => {
    if (!snapshot?.market.active?.window.window_end) return;
    const update = () => setCountdown(estimateWindowCountdown(snapshot.market.active?.window.window_end));
    update();
    const timer = window.setInterval(update, 1000);
    return () => window.clearInterval(timer);
  }, [snapshot?.market.active?.window.window_end]);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    const pageWidth = node.clientWidth;
    node.scrollTo({ left: activePage * pageWidth, behavior: "smooth" });
  }, [activePage]);

  const referencePrices = Object.values(snapshot?.market.latest_reference_prices ?? {});
  const btcSpot = useMemo(() => median(referencePrices) ?? null, [referencePrices]);
  const targetPrice = snapshot?.market.target_price ?? null;
  const yesLevels = useMemo(() => parseBookLevels(snapshot?.market.orderbook_snapshot?.bids), [snapshot?.market.orderbook_snapshot]);
  const noLevels = useMemo(() => parseBookLevels(snapshot?.market.orderbook_snapshot?.asks), [snapshot?.market.orderbook_snapshot]);
  const bestYes = yesLevels[0]?.[0] ?? snapshot?.market.latest_market_price ?? null;
  const bestNo = noLevels[0]?.[0] ?? (bestYes != null ? Math.max(0, 1 - bestYes) : null);
  const portfolioPoints = (snapshot?.account.portfolio_history ?? []).map((entry) => ({
    time: entry.timestamp,
    value: entry.portfolio_value,
  }));
  const btcPoints = useMemo(() => {
    const sources = ["binance", "coinbase", "kraken", "okx", "bitstamp"];
    for (const source of sources) {
      const history = snapshot?.market.price_history?.[source];
      if (history?.length) {
        return history.map((entry) => ({ time: entry.timestamp, value: entry.value }));
      }
    }
    return [];
  }, [snapshot?.market.price_history]);
  const positions = Object.values(snapshot?.account.positions?.positions ?? {});
  const openOrders = snapshot?.account.open_orders ?? [];
  const recentHistory = [...(snapshot?.account.position_history ?? [])].reverse().slice(0, 8);
  const liveWindow = countdown != null && countdown > 0;

  if (!token || !snapshot) {
    return <TradeLogin loading={loading} error={authError} onSubmit={login} />;
  }

  const submitOrder = () => {
    const parsedSize = Number(size);
    const parsedPrice = Number(price);
    if (!Number.isFinite(parsedSize) || parsedSize <= 0) return;
    placeOrder({
      orderType,
      side,
      outcome,
      sizeType,
      size: parsedSize,
      price: orderType === "limit" ? parsedPrice : undefined,
    });
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen max-w-md flex-col px-4 pb-6 pt-safe">
        <header className="mb-2 flex items-center justify-between pt-2">
          <div className="min-w-0">
            <p className="text-[10px] uppercase tracking-[0.28em] text-muted-foreground">PM Trade</p>
            <h1 className="text-base font-semibold">BTC 5m</h1>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="flex items-center gap-1 rounded-full bg-white/5 px-2 py-1 text-[10px] text-muted-foreground">
              <Activity className={cn("h-3 w-3", syncState === "live" && connected ? "text-success" : "text-warning")} />
              <span>{syncState === "live" && connected ? "Live" : "Syncing"}</span>
            </div>
            <button
              className="flex h-8 w-8 items-center justify-center rounded-xl bg-white/5 text-muted-foreground"
              onClick={() => refresh()}
            >
              <RefreshCcw className="h-3.5 w-3.5" />
            </button>
            <button
              className="flex h-8 w-8 items-center justify-center rounded-xl bg-white/5 text-muted-foreground"
              onClick={() => logout()}
            >
              <LogOut className="h-3.5 w-3.5" />
            </button>
          </div>
        </header>

        <div
          ref={scrollRef}
          className="flex flex-1 snap-x snap-mandatory gap-4 overflow-x-auto pb-4 scrollbar-none"
          onScroll={(event) => {
            const pageWidth = event.currentTarget.clientWidth;
            if (pageWidth === 0) return;
            setActivePage(Math.round(event.currentTarget.scrollLeft / pageWidth));
          }}
        >
          <section className="flex min-w-full snap-start flex-col gap-3">
            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-2 flex items-center justify-between">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">BTC price</p>
                  <p className="text-2xl font-semibold leading-none">{formatCurrency(btcSpot, 0)}</p>
                </div>
                <div className="text-right">
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Target</p>
                  <p className="text-lg font-semibold leading-none">{formatCurrency(targetPrice, 0)}</p>
                </div>
              </div>
              <div className="mb-3 flex items-center justify-between text-xs text-muted-foreground">
                <span className={cn("rounded-full px-2.5 py-1", liveWindow ? "bg-success/12 text-success" : "bg-warning/12 text-warning")}>
                  {liveWindow ? "Live 5m window" : "Window rolling"}
                </span>
                <span className="font-mono text-foreground">{formatCountdown(countdown)}</span>
              </div>
              <p className="mb-3 truncate text-xs text-muted-foreground">{snapshot.market.active?.slug}</p>
              <SparklineChart className="h-44 w-full" data={btcPoints} color="#f7931a" showPriceScale />
            </div>

            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-3 grid grid-cols-2 gap-3">
                <div className="rounded-[1.5rem] border border-success/15 bg-success/10 px-3 py-3">
                  <p className="text-[11px] uppercase tracking-[0.22em] text-success/80">Buy Up</p>
                  <p className="mt-1 text-2xl font-semibold leading-none text-success">{formatSharePrice(bestYes)}</p>
                </div>
                <div className="rounded-[1.5rem] border border-danger/15 bg-danger/10 px-3 py-3">
                  <p className="text-[11px] uppercase tracking-[0.22em] text-danger/80">Buy Down</p>
                  <p className="mt-1 text-2xl font-semibold leading-none text-danger">{formatSharePrice(bestNo)}</p>
                </div>
              </div>

              <div className="mb-3 flex items-center justify-between">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Trade ticket</p>
                  <p className="text-sm font-semibold">Place order</p>
                </div>
                <span className="rounded-full bg-white/5 px-3 py-1 text-xs text-muted-foreground">
                  {snapshot.meta.user_id}
                </span>
              </div>

              <Segmented
                compact
                label="Type"
                options={[
                  { value: "market", label: "Market" },
                  { value: "limit", label: "Limit" },
                ]}
                value={orderType}
                onChange={(value) => setOrderType(value as OrderType)}
              />
              <Segmented
                compact
                label="Side"
                options={[
                  { value: "buy", label: "Buy" },
                  { value: "sell", label: "Sell" },
                ]}
                value={side}
                onChange={(value) => setSide(value as TradeSide)}
              />
              <Segmented
                compact
                label="Outcome"
                options={[
                  { value: "up", label: "Up" },
                  { value: "down", label: "Down" },
                ]}
                value={outcome}
                onChange={(value) => setOutcome(value as Outcome)}
              />
              <Segmented
                compact
                label="Size"
                options={[
                  { value: "shares", label: "Shares" },
                  { value: "dollars", label: "Dollars" },
                ]}
                value={sizeType}
                onChange={(value) => setSizeType(value as SizeType)}
              />

              <div className="mt-2 grid grid-cols-2 gap-3">
                <Field compact label={sizeType === "shares" ? "Shares" : "Dollars"} value={size} onChange={setSize} />
                <Field
                  compact
                  disabled={orderType !== "limit"}
                  label="Limit price"
                  value={price}
                  onChange={setPrice}
                  placeholder="0.50"
                />
              </div>

              <Button className="mt-3 h-12 w-full" disabled={sending} onClick={submitOrder}>
                {sending ? "Submitting..." : `${side === "buy" ? "Buy" : "Sell"} ${outcome === "up" ? "Up" : "Down"}`}
              </Button>
            </div>
          </section>

          <section className="flex min-w-full snap-start flex-col gap-4">
            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-4 flex items-center justify-between">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Portfolio</p>
                  <p className="text-2xl font-semibold">
                    {formatCurrency(snapshot.account.profitability.portfolio_value ?? undefined)}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Unrealized</p>
                  <p
                    className={cn(
                      "text-lg font-semibold",
                      (snapshot.account.profitability.unrealized_pnl ?? 0) >= 0 ? "text-success" : "text-danger",
                    )}
                  >
                    {formatSignedCurrency(snapshot.account.profitability.unrealized_pnl ?? undefined)}
                  </p>
                </div>
              </div>
              <SparklineChart data={portfolioPoints} color="#4ade80" />
              <div className="mt-3 flex items-center justify-between text-sm text-muted-foreground">
                <span>Cash {formatCurrency(snapshot.account.profitability.cash_balance ?? undefined)}</span>
                <span>{positions.length} live positions</span>
              </div>
            </div>

            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-3 flex items-center justify-between">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Open orders</p>
                  <p className="text-base font-semibold">{openOrders.length}</p>
                </div>
                <Button variant="ghost" size="sm" onClick={() => cancelAll()}>
                  Cancel all
                </Button>
              </div>
              <div className="space-y-2">
                {openOrders.length === 0 ? (
                  <EmptyState text="No working orders" />
                ) : (
                  openOrders.slice(0, 5).map((order) => (
                    <button
                      key={order.order_id}
                      className="flex w-full items-center justify-between rounded-2xl border border-white/5 bg-background/70 px-3 py-3 text-left"
                      onClick={() => cancelOrder(order.order_id)}
                    >
                      <div>
                        <p className="text-sm font-medium">
                          {order.side.toUpperCase()} {order.outcome?.toUpperCase() ?? "--"}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {formatSharePrice(order.price ?? undefined)} · {formatCompactNumber(order.size)}
                        </p>
                      </div>
                      <span className="text-xs text-warning">Tap to cancel</span>
                    </button>
                  ))
                )}
              </div>
            </div>

            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-3 flex items-center justify-between">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Positions</p>
                  <p className="text-base font-semibold">Running exposure</p>
                </div>
                <WalletCards className="h-4 w-4 text-muted-foreground" />
              </div>
              <div className="space-y-2">
                {positions.length === 0 ? (
                  <EmptyState text="No active positions" />
                ) : (
                  positions.map((position, index) => (
                    <div key={`${position.market_slug}-${position.outcome}-${index}`} className="rounded-2xl border border-white/5 bg-background/70 px-3 py-3">
                      <div className="flex items-center justify-between">
                        <p className="font-medium">{position.outcome.toUpperCase()}</p>
                        <p className={cn("text-sm font-medium", (position.unrealized_pnl ?? 0) >= 0 ? "text-success" : "text-danger")}>
                          {formatSignedCurrency(position.unrealized_pnl ?? undefined)}
                        </p>
                      </div>
                      <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
                        <span>{formatCompactNumber(position.size)} shares</span>
                        <span>avg {formatSharePrice(position.average_price ?? undefined)}</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>

            <div className="rounded-[2rem] border border-white/6 bg-card/90 p-4 shadow-panel">
              <div className="mb-3">
                <p className="text-[11px] uppercase tracking-[0.24em] text-muted-foreground">Recent position history</p>
                <p className="text-base font-semibold">Last state changes</p>
              </div>
              <div className="space-y-2">
                {recentHistory.length === 0 ? (
                  <EmptyState text="No recent position changes" />
                ) : (
                  recentHistory.map((entry, index) => (
                    <div key={`${entry.timestamp}-${entry.market_slug}-${index}`} className="rounded-2xl border border-white/5 bg-background/70 px-3 py-3">
                      <div className="flex items-center justify-between">
                        <p className="font-medium">
                          {entry.status.toUpperCase()} {entry.outcome.toUpperCase()}
                        </p>
                        <span className="text-xs text-muted-foreground">{formatDateTime(entry.timestamp)}</span>
                      </div>
                      <div className="mt-1 flex items-center justify-between text-xs text-muted-foreground">
                        <span>{formatCompactNumber(entry.size)} shares</span>
                        <span>{formatSignedCurrency(entry.unrealized_pnl ?? undefined)}</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          </section>
        </div>

        <div className="mt-2 flex items-center justify-center gap-2">
          {pageIds.map((page, index) => (
            <button
              key={page}
              className={cn(
                "h-2.5 rounded-full transition-all",
                activePage === index ? "w-8 bg-accent" : "w-2.5 bg-white/14",
              )}
              onClick={() => setActivePage(index)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function Segmented({
  label,
  options,
  value,
  onChange,
  compact,
}: {
  label: string;
  options: Array<{ value: string; label: string }>;
  value: string;
  onChange: (value: string) => void;
  compact?: boolean;
}) {
  return (
    <div className={compact ? "mt-2" : "mt-3"}>
      <p className="mb-2 text-[11px] uppercase tracking-[0.24em] text-muted-foreground">{label}</p>
      <div className="grid grid-cols-2 gap-2 rounded-2xl bg-background/70 p-1">
        {options.map((option) => (
          <button
            key={option.value}
            className={cn(
              compact ? "rounded-2xl px-3 py-2 text-sm font-medium transition" : "rounded-2xl px-3 py-2.5 text-sm font-medium transition",
              option.value === value ? "bg-accent text-accent-foreground" : "text-muted-foreground",
            )}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  disabled,
  placeholder,
  compact,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  compact?: boolean;
}) {
  return (
    <label className={cn(compact ? "rounded-2xl border border-white/8 bg-background/70 px-3 py-2.5" : "rounded-2xl border border-white/8 bg-background/70 px-3 py-3", disabled && "opacity-50")}>
      <p className="mb-2 text-[11px] uppercase tracking-[0.24em] text-muted-foreground">{label}</p>
      <input
        className={cn("w-full bg-transparent font-semibold outline-none", compact ? "text-base" : "text-lg")}
        disabled={disabled}
        inputMode="decimal"
        placeholder={placeholder}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

function EmptyState({ text }: { text: string }) {
  return <div className="rounded-2xl border border-dashed border-white/8 px-3 py-5 text-center text-sm text-muted-foreground">{text}</div>;
}

export default App;
