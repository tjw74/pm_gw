import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatDuration(seconds?: number | null) {
  if (seconds == null) return "--";
  const s = Math.max(0, Math.floor(seconds));
  const mins = Math.floor(s / 60);
  const rem = s % 60;
  const hours = Math.floor(mins / 60);
  if (hours > 0) return `${hours}h ${mins % 60}m`;
  return `${mins}m ${rem}s`;
}

export function formatMs(value?: number | null) {
  if (value == null) return "--";
  return `${Math.round(value)} ms`;
}

export function relativeAgeMs(value?: number | null) {
  if (value == null) return "--";
  if (value < 1_000) return `${value}ms`;
  if (value < 60_000) return `${(value / 1000).toFixed(1)}s`;
  return `${(value / 60000).toFixed(1)}m`;
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

export function formatDateTime(value?: string | null) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toISOString().replace("T", " ").replace(".000Z", " UTC");
}

export function parseSlugUnixTimestamp(slug?: string | null) {
  if (!slug) return null;
  const match = slug.match(/-(\d{10})$/);
  if (!match) return null;
  const timestamp = Number(match[1]) * 1000;
  if (!Number.isFinite(timestamp)) return null;
  return new Date(timestamp);
}

export function formatSlugWindowTime(slug?: string | null) {
  const date = parseSlugUnixTimestamp(slug);
  if (!date || Number.isNaN(date.getTime())) return null;
  return date.toISOString().replace("T", " ").replace(".000Z", " UTC");
}
