import { useEffect } from "react";
import { Route, Routes } from "react-router-dom";
import { AppShell } from "@/components/AppShell";
import { useDashboardStore } from "@/store/useDashboardStore";
import { OverviewPage } from "@/pages/OverviewPage";
import { FeedsPage } from "@/pages/FeedsPage";
import { MarketPage } from "@/pages/MarketPage";
import { StreamingPage } from "@/pages/StreamingPage";
import { AlertsPage } from "@/pages/AlertsPage";
import { AdminPage } from "@/pages/AdminPage";
import { LoginPage } from "@/pages/LoginPage";

export default function App() {
  const init = useDashboardStore((state) => state.init);
  const loading = useDashboardStore((state) => state.loading);
  const error = useDashboardStore((state) => state.error);
  const snapshot = useDashboardStore((state) => state.publicSnapshot);

  useEffect(() => {
    void init();
  }, [init]);

  if (loading && !snapshot) {
    return <div className="flex min-h-screen items-center justify-center text-muted-foreground">Loading mission control…</div>;
  }

  if (error && !snapshot) {
    return <div className="flex min-h-screen items-center justify-center text-danger">{error}</div>;
  }

  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route path="/" element={<OverviewPage />} />
        <Route path="/feeds" element={<FeedsPage />} />
        <Route path="/market" element={<MarketPage />} />
        <Route path="/streaming" element={<StreamingPage />} />
        <Route path="/alerts" element={<AlertsPage />} />
        <Route path="/admin" element={<AdminPage />} />
        <Route path="/login" element={<LoginPage />} />
      </Route>
    </Routes>
  );
}
