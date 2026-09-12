import type { AppError } from "../../contracts/app-error";

export type AuthStatus =
  | "restoring"
  | "anonymous"
  | "requesting"
  | "waiting_scan"
  | "waiting_confirm"
  | "authenticated"
  | "expired"
  | "cancelled"
  | "error";

export interface AuthAccount {
  mid: string | null;
  name: string;
  avatarUrl: string | null;
}

export interface AuthSnapshot {
  revision: number;
  status: AuthStatus;
  qrContent: string | null;
  expiresAt: string | null;
  account: AuthAccount | null;
  error: AppError | null;
}

export interface AuthStateEvent {
  snapshot: AuthSnapshot;
}
