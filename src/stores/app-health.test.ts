import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppIpcService } from "../services/ipc/app";
import { useAppStore } from "./app";

describe("application health initialization", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loads app metadata and marks the backend healthy", async () => {
    const service: AppIpcService = {
      getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.0" }),
      checkBackendHealth: vi.fn().mockResolvedValue({ status: "ok", timestamp: "2026-09-10T00:00:00Z" }),
    };
    const store = useAppStore();

    const pending = store.initialize(service);
    expect(store.backendStatus).toBe("pending");
    await pending;

    expect(store.backendStatus).toBe("healthy");
    expect(store.appInfo?.name).toBe("BiliCatch");
    expect(store.backendError).toBeNull();
  });

  it("keeps a normalized failure and supports retry", async () => {
    const service: AppIpcService = {
      getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.0" }),
      checkBackendHealth: vi
        .fn()
        .mockRejectedValueOnce({ code: "E007", message: "Unavailable" })
        .mockResolvedValueOnce({ status: "ok", timestamp: "2026-09-10T00:00:00Z" }),
    };
    const store = useAppStore();

    await store.initialize(service);
    expect(store.backendStatus).toBe("failed");
    expect(store.backendError?.code).toBe("E007");

    await store.initialize(service);
    expect(store.backendStatus).toBe("healthy");
    expect(service.checkBackendHealth).toHaveBeenCalledTimes(2);
  });
});
