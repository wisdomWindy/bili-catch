import type { DownloadTask, TaskStatus } from "./contracts";

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"] as const;
const ACTIVE = new Set<TaskStatus>(["queued", "downloading", "paused", "processing"]);

export function formatBytes(value: string | null): string {
  if (value === null || !/^\d+$/.test(value)) return "—";
  let bytes = BigInt(value);
  let unit = 0;
  let divisor = 1n;
  while (unit < UNITS.length - 1 && bytes >= divisor * 1024n) {
    divisor *= 1024n;
    unit += 1;
  }
  const tenths = (bytes * 10n + divisor / 2n) / divisor;
  const whole = tenths / 10n;
  const fraction = tenths % 10n;
  return `${whole}${fraction === 0n ? "" : `.${fraction}`} ${UNITS[unit]}`;
}

export function formatSpeed(value: string | null): string {
  const formatted = formatBytes(value);
  return formatted === "—" ? formatted : `${formatted}/s`;
}

export function formatEta(seconds: number | null): string {
  if (seconds === null || seconds < 0 || !Number.isFinite(seconds)) return "—";
  const total = Math.floor(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remaining = total % 60;
  const mm = String(minutes).padStart(2, "0");
  const ss = String(remaining).padStart(2, "0");
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
}

export function compareTasks(left: DownloadTask, right: DownloadTask): number {
  const activeDifference = Number(ACTIVE.has(right.status)) - Number(ACTIVE.has(left.status));
  if (activeDifference !== 0) return activeDifference;
  const dateDifference = Date.parse(right.createdAt) - Date.parse(left.createdAt);
  return dateDifference || left.id.localeCompare(right.id);
}
