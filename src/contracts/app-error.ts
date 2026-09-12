export const APP_ERROR_CODES = [
  "E001",
  "E002",
  "E003",
  "E004",
  "E005",
  "E006",
  "E007",
  "E008",
  "E009",
  "E010",
  "E_INTERNAL",
] as const;

export type AppErrorCode = (typeof APP_ERROR_CODES)[number];

export interface AppError {
  code: AppErrorCode;
  message: string;
  details?: string;
}

const appErrorCodes = new Set<string>(APP_ERROR_CODES);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isAppErrorCode(value: unknown): value is AppErrorCode {
  return typeof value === "string" && appErrorCodes.has(value);
}

export function normalizeIpcError(value: unknown): AppError {
  if (isRecord(value) && isAppErrorCode(value.code) && typeof value.message === "string") {
    const error: AppError = { code: value.code, message: value.message };
    if (typeof value.details === "string") error.details = value.details;
    return error;
  }

  return {
    code: "E_INTERNAL",
    message: typeof value === "string" && value.trim() ? value : "Unexpected application error",
  };
}
