export type ParseInputResult =
  | { ok: true; value: string }
  | { ok: false; reason: "empty" | "invalid" };

const bvidPattern = /^BV[0-9A-Za-z]{10}$/;
const aidPattern = /^av[1-9][0-9]*$/i;

function isAllowedHost(hostname: string): boolean {
  return hostname === "b23.tv" || hostname === "bilibili.com" || hostname.endsWith(".bilibili.com");
}

function isSupportedUrl(value: string): boolean {
  try {
    const url = new URL(value);
    if (url.protocol !== "https:" || !isAllowedHost(url.hostname.toLowerCase())) return false;
    if (url.hostname.toLowerCase() === "b23.tv") return url.pathname.length > 1;
    const segments = url.pathname.split("/").filter(Boolean);
    const videoId = segments[segments.length - 1] ?? "";
    if (!bvidPattern.test(videoId) && !aidPattern.test(videoId)) return false;
    const page = url.searchParams.get("p");
    return page === null || /^[1-9][0-9]*$/.test(page);
  } catch {
    return false;
  }
}

function normalizePastedUrl(value: string): string {
  return value.replace(/\\([?&=_#])/g, "$1");
}

export function normalizeParseInput(input: string): ParseInputResult {
  const value = normalizePastedUrl(input.trim());
  if (!value) return { ok: false, reason: "empty" };
  if ([...value].some((character) => /\s/.test(character) || character < " ")) {
    return { ok: false, reason: "invalid" };
  }
  if (bvidPattern.test(value) || aidPattern.test(value) || isSupportedUrl(value)) {
    return { ok: true, value };
  }
  return { ok: false, reason: "invalid" };
}

export function parseInputCacheKey(input: string): string | null {
  const normalized = normalizeParseInput(input);
  if (!normalized.ok) return null;
  const value = normalized.value;
  if (bvidPattern.test(value)) return `bvid:${value}:p1`;
  if (aidPattern.test(value)) return `aid:${value.slice(2)}:p1`;

  const url = new URL(value);
  if (url.hostname.toLowerCase() === "b23.tv") return `short:${url.toString()}`;
  const segments = url.pathname.split("/").filter(Boolean);
  const videoId = segments[segments.length - 1] ?? "";
  const page = url.searchParams.get("p") ?? "1";
  if (bvidPattern.test(videoId)) return `bvid:${videoId}:p${page}`;
  return `aid:${videoId.slice(2)}:p${page}`;
}
