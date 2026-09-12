import type { AuthSnapshot, AuthStateEvent, AuthStatus } from "./contracts";
import type { AuthEventSource } from "./events";
import type { AuthService } from "./service";

const anonymous = (revision: number): AuthSnapshot => ({
  revision,
  status: "anonymous",
  qrContent: null,
  expiresAt: null,
  account: null,
  error: null,
});

function demoSnapshot(status: AuthStatus): AuthSnapshot {
  const base = anonymous(1);
  if (status === "waiting_scan" || status === "waiting_confirm") {
    return {
      ...base,
      status,
      qrContent: "https://passport.bilibili.com/x/passport-login/web/qrcode/demo",
      expiresAt: new Date(Date.now() + 180_000).toISOString(),
    };
  }
  if (status === "authenticated") {
    return {
      ...base,
      status,
      account: { mid: "9001", name: "BiliCatch Demo", avatarUrl: null },
    };
  }
  if (status === "error") {
    return {
      ...base,
      status,
      error: { code: "E002", message: "Demo authentication request timed out" },
    };
  }
  return { ...base, status };
}

export function createDemoAuthRuntime(initialStatus: AuthStatus = "anonymous"): {
  service: AuthService;
  events: AuthEventSource;
} {
  let current = demoSnapshot(initialStatus);
  let listener: ((event: AuthStateEvent) => void) | null = null;
  const commit = (next: AuthSnapshot) => {
    current = next;
    listener?.({ snapshot: current });
    return current;
  };

  return {
    service: {
      getSnapshot: async () => current,
      start: async () => commit({
        ...anonymous(current.revision + 1),
        status: "waiting_scan",
        qrContent: "https://passport.bilibili.com/x/passport-login/web/qrcode/demo",
        expiresAt: new Date(Date.now() + 180_000).toISOString(),
      }),
      cancel: async () => ["requesting", "waiting_scan", "waiting_confirm"].includes(current.status)
        ? commit({ ...anonymous(current.revision + 1), status: "cancelled" })
        : current,
      logout: async () => current.status === "authenticated"
        ? commit(anonymous(current.revision + 1))
        : current,
    },
    events: {
      subscribe: async (onState) => {
        listener = onState;
        return () => {
          if (listener === onState) listener = null;
        };
      },
    },
  };
}
