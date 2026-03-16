import { KeyRound, LockKeyhole } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/Button";

interface TradeLoginProps {
  loading: boolean;
  error?: string | null;
  onSubmit: (token: string) => Promise<void> | void;
}

export function TradeLogin({ loading, error, onSubmit }: TradeLoginProps) {
  const [token, setToken] = useState("");

  return (
    <div className="flex min-h-screen items-center justify-center px-5 pb-8 pt-safe">
      <div className="w-full max-w-sm rounded-[2rem] border border-white/6 bg-card/95 p-6 shadow-panel backdrop-blur">
        <div className="mb-6 flex items-center gap-3">
          <div className="flex h-11 w-11 items-center justify-center rounded-2xl bg-bitcoin/16 text-bitcoin">
            <KeyRound className="h-5 w-5" />
          </div>
          <div>
            <p className="text-xs uppercase tracking-[0.28em] text-muted-foreground">PM Trade</p>
            <h1 className="text-xl font-semibold text-foreground">Gateway Access</h1>
          </div>
        </div>

        <p className="mb-5 text-sm leading-6 text-muted-foreground">
          Enter your private gateway key. This logs you into your assigned Polymarket account context.
        </p>

        <label className="mb-2 block text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
          Gateway key
        </label>
        <div className="mb-4 flex items-center gap-3 rounded-2xl border border-white/8 bg-background/80 px-4 py-3">
          <LockKeyhole className="h-4 w-4 text-muted-foreground" />
          <input
            autoCapitalize="none"
            autoComplete="off"
            autoCorrect="off"
            className="w-full bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/60"
            placeholder="Paste your user token"
            type="password"
            value={token}
            onChange={(event) => setToken(event.target.value)}
          />
        </div>

        {error ? <p className="mb-4 text-sm text-danger">{error}</p> : null}

        <Button
          className="w-full"
          disabled={loading || token.trim().length === 0}
          onClick={() => onSubmit(token.trim())}
        >
          {loading ? "Connecting..." : "Enter trade app"}
        </Button>
      </div>
    </div>
  );
}
