import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

type TimeTuple = [number, number, number, number, number, number, number?, number?, number?];

function isTimeTuple(value: unknown): value is TimeTuple {
  return Array.isArray(value) && value.length >= 6 && value.every((entry) => typeof entry === "number");
}

export function normalizeApiDates<T>(value: T): T {
  if (isTimeTuple(value)) {
    const [year, ordinal, hour, minute, second, nanos = 0] = value;
    const date = new Date(Date.UTC(year, 0, 1, hour, minute, second, Math.floor(nanos / 1_000_000)));
    date.setUTCDate(ordinal);
    return date.toISOString() as T;
  }
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeApiDates(entry)) as T;
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, entry]) => [key, normalizeApiDates(entry)]),
    ) as T;
  }
  return value;
}

export function formatCurrency(value?: number | null, digits = 2) {
  if (value == null || Number.isNaN(value)) return "--";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
}

export function formatCompactNumber(value?: number | null, digits = 2) {
  if (value == null || Number.isNaN(value)) return "--";
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: digits,
  }).format(value);
}

export function formatSharePrice(value?: number | null) {
  if (value == null || Number.isNaN(value)) return "--";
  return `${Math.round(value * 100)}c`;
}

export function formatPercent(value?: number | null) {
  if (value == null || Number.isNaN(value)) return "--";
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

export function formatSignedCurrency(value?: number | null) {
  if (value == null || Number.isNaN(value)) return "--";
  const formatted = formatCurrency(Math.abs(value));
  return `${value >= 0 ? "+" : "-"}${formatted}`;
}

export function formatDateTime(value?: string | null) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

export function formatCountdown(seconds?: number | null) {
  if (seconds == null || Number.isNaN(seconds)) return "--";
  const total = Math.max(0, Math.floor(seconds));
  const mins = Math.floor(total / 60);
  const secs = total % 60;
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function estimateWindowCountdown(windowEnd?: string | null) {
  if (!windowEnd) return null;
  const end = new Date(windowEnd).getTime();
  if (!Number.isFinite(end)) return null;
  return Math.max(0, Math.floor((end - Date.now()) / 1000));
}

export function median(values: number[]) {
  if (!values.length) return null;
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[middle];
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

export function parseBookLevels(raw: unknown): Array<[number, number]> {
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry) => {
      if (Array.isArray(entry) && entry.length >= 2) {
        const price = Number(entry[0]);
        const size = Number(entry[1]);
        return Number.isFinite(price) && Number.isFinite(size) ? ([price, size] as [number, number]) : null;
      }
      if (entry && typeof entry === "object") {
        const record = entry as Record<string, unknown>;
        const price = Number(record.price ?? record[0]);
        const size = Number(record.size ?? record.amount ?? record.quantity ?? record[1]);
        return Number.isFinite(price) && Number.isFinite(size) ? ([price, size] as [number, number]) : null;
      }
      return null;
    })
    .filter((entry): entry is [number, number] => entry !== null);
}
