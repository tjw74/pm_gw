const baseUrl = import.meta.env.VITE_PM_GW_BASE_URL?.replace(/\/$/, "") ?? "";

export const apiUrl = (path: string) => `${baseUrl}${path}`;

export const wsUrl = (path: string) => {
  const url = new URL(apiUrl(path), window.location.origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
};

export async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(apiUrl(path), {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `Request failed: ${response.status}`);
  }
  return response.json();
}
