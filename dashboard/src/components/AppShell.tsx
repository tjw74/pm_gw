import { useEffect, useLayoutEffect } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import { Activity, AlertTriangle, CandlestickChart, Gauge, Radio, Shield } from "lucide-react";
import { useDashboardStore } from "@/store/useDashboardStore";
import { Button } from "@/components/ui/button";

const nav = [
  { to: "/", label: "Overview", icon: Gauge },
  { to: "/feeds", label: "Feeds", icon: Radio },
  { to: "/market", label: "Market", icon: CandlestickChart },
  { to: "/streaming", label: "Streaming", icon: Activity },
  { to: "/alerts", label: "Alerts", icon: AlertTriangle },
  { to: "/admin", label: "Admin", icon: Shield },
];

export function AppShell() {
  const token = useDashboardStore((state) => state.token);
  const logout = useDashboardStore((state) => state.logout);
  const generatedAt = useDashboardStore((state) => state.publicSnapshot?.meta.generated_at);
  const location = useLocation();
  const scrollKey = `pm_gw_scroll:${location.pathname}`;

  useEffect(() => {
    const restore = sessionStorage.getItem(scrollKey);
    if (restore) {
      window.scrollTo({ top: Number(restore), behavior: "auto" });
    }
    const onScroll = () => {
      sessionStorage.setItem(scrollKey, String(window.scrollY));
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [scrollKey]);

  useLayoutEffect(() => {
    const restore = sessionStorage.getItem(scrollKey);
    if (!restore) return;
    requestAnimationFrame(() => {
      window.scrollTo({ top: Number(restore), behavior: "auto" });
    });
  }, [generatedAt, scrollKey]);

  return (
    <div className="mx-auto flex min-h-screen max-w-[1760px] gap-4 px-4 py-4">
      <aside className="hidden w-60 shrink-0 lg:block">
        <div className="surface sticky top-4 p-4">
          <Link to="/" className="flex items-center gap-2.5">
            <div className="rounded-2xl border border-bitcoin/30 bg-bitcoin/10 p-2.5 text-bitcoin">pm</div>
            <div>
              <div className="text-[11px] uppercase tracking-[0.22em] text-muted-foreground">Mission Control</div>
              <div className="mt-0.5 text-base font-semibold">pm_gw</div>
            </div>
          </Link>
          <nav className="mt-6 space-y-1.5">
            {nav.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) =>
                  `flex items-center gap-2.5 rounded-2xl px-3 py-2.5 text-sm transition ${
                    isActive ? "bg-white/8 text-foreground" : "text-muted-foreground hover:bg-white/5 hover:text-foreground"
                  }`
                }
              >
                <item.icon className="h-4 w-4" />
                {item.label}
              </NavLink>
            ))}
          </nav>
          <div className="mt-6 rounded-2xl border border-white/5 bg-black/20 p-3.5">
            <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Access</div>
            <div className="mt-2.5 text-sm text-foreground">{token ? "Admin mode unlocked" : "Public read-only mode"}</div>
            {token ? (
              <Button variant="ghost" className="mt-3 w-full" onClick={logout}>Logout</Button>
            ) : (
              <Link to="/login" className="mt-3 inline-flex h-10 w-full items-center justify-center rounded-2xl bg-accent px-4 text-sm font-medium text-accent-foreground transition hover:bg-accent/90">
                Admin login
              </Link>
            )}
          </div>
        </div>
      </aside>
      <main className="min-w-0 flex-1">
        <Outlet />
      </main>
    </div>
  );
}
