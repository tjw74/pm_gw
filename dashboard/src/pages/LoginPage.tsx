import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useDashboardStore } from "@/store/useDashboardStore";

export function LoginPage() {
  const login = useDashboardStore((state) => state.login);
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string>();
  const navigate = useNavigate();

  return (
    <div className="mx-auto max-w-md pt-16">
      <Card>
        <div className="text-sm uppercase tracking-[0.18em] text-muted-foreground">Operator access</div>
        <h1 className="mt-2 text-3xl font-semibold">Unlock admin mode</h1>
        <form
          className="mt-6 space-y-4"
          onSubmit={async (event) => {
            event.preventDefault();
            try {
              setError(undefined);
              await login(username, password);
              navigate("/admin");
            } catch (err) {
              setError(err instanceof Error ? err.message : "Login failed");
            }
          }}
        >
          <Input value={username} onChange={(event) => setUsername(event.target.value)} placeholder="Username" />
          <Input value={password} onChange={(event) => setPassword(event.target.value)} placeholder="Password" type="password" />
          {error ? <div className="text-sm text-danger">{error}</div> : null}
          <Button className="w-full" type="submit">Login</Button>
        </form>
      </Card>
    </div>
  );
}
