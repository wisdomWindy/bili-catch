import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthSnapshot, AuthStateEvent, AuthStatus } from "./contracts";
import type { AuthEventSource } from "./events";
import type { AuthService } from "./service";
import { useAuthStore } from "./store";

const snapshot = (revision: number, status: AuthStatus = "anonymous"): AuthSnapshot => ({
  revision,
  status,
  qrContent: status === "waiting_scan" ? "https://passport.bilibili.com/fixture" : null,
  expiresAt: status === "waiting_scan" ? "2026-09-11T00:03:00Z" : null,
  account: status === "authenticated"
    ? { mid: "9001", name: "Fixture account", avatarUrl: null }
    : null,
  error: null,
});

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((onResolve, onReject) => {
    resolve = onResolve;
    reject = onReject;
  });
  return { promise, resolve, reject };
};

const serviceWith = (overrides: Partial<AuthService> = {}): AuthService => ({
  getSnapshot: vi.fn(async () => snapshot(1)),
  start: vi.fn(async () => snapshot(2, "waiting_scan")),
  cancel: vi.fn(async () => snapshot(3, "cancelled")),
  logout: vi.fn(async () => snapshot(4)),
  ...overrides,
});

describe("authentication store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("subscribes before loading and replays only newer buffered events", async () => {
    let onState!: (event: AuthStateEvent) => void;
    const events: AuthEventSource = {
      subscribe: vi.fn(async (handler) => {
        onState = handler;
        return vi.fn();
      }),
    };
    const service = serviceWith({
      getSnapshot: vi.fn(async () => {
        onState({ snapshot: snapshot(3, "waiting_scan") });
        return snapshot(2);
      }),
    });
    const store = useAuthStore();

    await store.initialize(service, events);
    onState({ snapshot: snapshot(2, "cancelled") });
    onState({ snapshot: snapshot(4, "authenticated") });

    expect(events.subscribe).toHaveBeenCalledBefore(service.getSnapshot as ReturnType<typeof vi.fn>);
    expect(store.snapshot).toEqual(snapshot(4, "authenticated"));
    expect(store.status).toBe("ready");
  });

  it("keeps command snapshots usable when event subscription fails", async () => {
    const events: AuthEventSource = {
      subscribe: vi.fn(async () => { throw "listener unavailable"; }),
    };
    const store = useAuthStore();

    await store.initialize(serviceWith(), events);

    expect(store.snapshot.revision).toBe(1);
    expect(store.status).toBe("ready");
    expect(store.eventError).toMatchObject({ code: "E_INTERNAL" });
  });

  it("keeps action pending flags independent and clears them after success or failure", async () => {
    const started = deferred<AuthSnapshot>();
    const service = serviceWith({
      start: vi.fn(() => started.promise),
      cancel: vi.fn(async () => { throw { code: "E002", message: "timed out" }; }),
    });
    const events: AuthEventSource = { subscribe: vi.fn(async () => vi.fn()) };
    const store = useAuthStore();
    await store.initialize(service, events);

    const first = store.start();
    void store.start();
    expect(store.pending.start).toBe(true);
    expect(service.start).toHaveBeenCalledOnce();
    started.resolve(snapshot(2, "waiting_scan"));
    await first;
    expect(store.pending.start).toBe(false);
    expect(store.snapshot.status).toBe("waiting_scan");

    await store.cancel();
    expect(store.pending.cancel).toBe(false);
    expect(store.error).toMatchObject({ code: "E002", message: "timed out" });
  });

  it("ignores a snapshot response that resolves after disposal", async () => {
    const loaded = deferred<AuthSnapshot>();
    const unsubscribe = vi.fn();
    const service = serviceWith({ getSnapshot: vi.fn(() => loaded.promise) });
    const events: AuthEventSource = { subscribe: vi.fn(async () => unsubscribe) };
    const store = useAuthStore();

    const initializing = store.initialize(service, events);
    await vi.waitFor(() => expect(service.getSnapshot).toHaveBeenCalledOnce());
    store.dispose();
    loaded.resolve(snapshot(5, "authenticated"));
    await initializing;

    expect(store.snapshot.revision).toBe(0);
    expect(unsubscribe).toHaveBeenCalledOnce();
  });

  it("releases a stale listener when a newer initialization wins", async () => {
    const firstSubscription = deferred<() => void>();
    const firstUnsubscribe = vi.fn();
    const secondUnsubscribe = vi.fn();
    let subscriptionCount = 0;
    const events: AuthEventSource = {
      subscribe: vi.fn(async () => {
        subscriptionCount += 1;
        return subscriptionCount === 1
          ? firstSubscription.promise
          : secondUnsubscribe;
      }),
    };
    const store = useAuthStore();

    const first = store.initialize(serviceWith(), events);
    const second = store.initialize(serviceWith(), events);
    await second;
    firstSubscription.resolve(firstUnsubscribe);
    await first;

    expect(firstUnsubscribe).toHaveBeenCalledOnce();
    store.dispose();
    expect(secondUnsubscribe).toHaveBeenCalledOnce();
  });
});
