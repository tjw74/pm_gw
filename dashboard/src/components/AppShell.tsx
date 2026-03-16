import { useEffect, useLayoutEffect, useState } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import { Activity, AlertTriangle, CandlestickChart, Gauge, PanelLeftClose, PanelLeftOpen, Radio, Shield } from "lucide-react";
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
  const [navCollapsed, setNavCollapsed] = useState(false);

  useEffect(() => {
    const saved = localStorage.getItem("pm_gw_nav_collapsed");
    if (saved != null) {
      setNavCollapsed(saved === "true");
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("pm_gw_nav_collapsed", String(navCollapsed));
  }, [navCollapsed]);

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
      <aside className={`hidden shrink-0 transition-[width] duration-200 lg:block ${navCollapsed ? "w-[72px]" : "w-60"}`}>
        <div className={`surface sticky top-4 transition-all duration-200 ${navCollapsed ? "p-2.5" : "p-4"}`}>
          <div className={`flex ${navCollapsed ? "flex-col items-center gap-2" : "items-start justify-between gap-3"}`}>
            <Link
              to="/"
              className={`flex min-w-0 items-center ${navCollapsed ? "justify-center" : "gap-2.5"}`}
              title="Overview"
            >
              <div className="rounded-2xl border border-bitcoin/30 bg-bitcoin/10 px-2.5 py-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-bitcoin">
                PMGW
              </div>
            </Link>
            <Button
              variant="ghost"
              size="sm"
              className="h-10 w-10 rounded-2xl border-0 px-0 shadow-none"
              onClick={() => setNavCollapsed((current) => !current)}
              aria-label={navCollapsed ? "Expand navigation" : "Collapse navigation"}
              title={navCollapsed ? "Expand navigation" : "Collapse navigation"}
            >
              {navCollapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
            </Button>
          </div>
          <nav className="mt-6 space-y-1.5">
            {nav.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                title={item.label}
                className={({ isActive }) =>
                  `flex rounded-2xl text-sm transition ${
                    navCollapsed ? "justify-center px-0 py-2.5" : "items-center gap-2.5 px-3 py-2.5"
                  } ${isActive ? "bg-white/8 text-foreground" : "text-muted-foreground hover:bg-white/5 hover:text-foreground"}`
                }
              >
                <item.icon className="h-4 w-4 shrink-0" />
                {navCollapsed ? null : item.label}
              </NavLink>
            ))}
          </nav>
          <div className={`mt-6 rounded-2xl border border-white/5 bg-black/20 ${navCollapsed ? "p-2" : "p-3.5"}`}>
            {navCollapsed ? (
              <div className="space-y-1.5">
                <Link
                  to={token ? "/admin" : "/login"}
                  title={token ? "Admin mode unlocked" : "Admin login"}
                  className="flex h-10 items-center justify-center rounded-2xl text-muted-foreground transition hover:bg-white/5 hover:text-foreground"
                >
                  <Shield className="h-4 w-4" />
                </Link>
                {token ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-10 w-full rounded-2xl px-0"
                    onClick={logout}
                    title="Logout"
                  >
                    <PanelLeftClose className="h-4 w-4 rotate-180" />
                  </Button>
                ) : null}
              </div>
            ) : (
              <>
                <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Access</div>
                <div className="mt-2.5 text-sm text-foreground">{token ? "Admin mode unlocked" : "Public read-only mode"}</div>
                {token ? (
                  <Button variant="ghost" className="mt-3 w-full" onClick={logout}>Logout</Button>
                ) : (
                  <Link to="/login" className="mt-3 inline-flex h-10 w-full items-center justify-center rounded-2xl bg-accent px-4 text-sm font-medium text-accent-foreground transition hover:bg-accent/90">
                    Admin login
                  </Link>
                )}
              </>
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
