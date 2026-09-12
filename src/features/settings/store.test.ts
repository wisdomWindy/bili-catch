import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SettingsSnapshot } from "./contracts";
import type { SettingsEffectSink } from "./effects";
import type { SettingsService } from "./service";
import { useSettingsStore } from "./store";

function makeSnapshot(
  revision = 0,
  values: Partial<SettingsSnapshot["values"]> = {},
): SettingsSnapshot {
  return {
    schemaVersion: 1,
    revision,
    values: {
      downloadDirectory: "D:/Downloads/BiliCatch",
      temporaryDirectory: "D:/Temp",
      maxConcurrentTasks: 3,
      connectionsPerTask: 8,
      defaultVideoQuality: "80",
      defaultAudioFormat: "mp3",
      theme: "system",
      locale: "zh-CN",
      notifyOnComplete: true,
      closeBehavior: "minimizeToTray",
      autoCheckUpdates: true,
      ...values,
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((onResolve, onReject) => {
    resolve = onResolve;
    reject = onReject;
  });
  return { promise, resolve, reject };
}

function effects(): SettingsEffectSink {
  return {
    applyAppearance: vi.fn(),
    applyCommitted: vi.fn(),
  };
}

describe("settings store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useRealTimers();
  });

  it("hydrates once and applies only a confirmed snapshot", async () => {
    const pending = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockReturnValue(pending.promise),
      update: vi.fn(),
    };
    const sink = effects();
    const store = useSettingsStore();

    const first = store.initialize(service, sink);
    const second = store.initialize(service, sink);
    expect(store.status).toBe("loading");
    expect(service.getSnapshot).toHaveBeenCalledOnce();
    expect(sink.applyCommitted).not.toHaveBeenCalled();
    pending.resolve(makeSnapshot());
    await Promise.all([first, second]);

    expect(store.status).toBe("ready");
    expect(store.values?.theme).toBe("system");
    expect(sink.applyCommitted).toHaveBeenCalledOnce();
    await store.initialize(service, sink);
    expect(service.getSnapshot).toHaveBeenCalledOnce();
  });

  it("uses safe appearance on load error and retry ignores an older lifecycle", async () => {
    const old = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn()
        .mockReturnValueOnce(old.promise)
        .mockResolvedValueOnce(makeSnapshot(4, { theme: "dark", locale: "en-US" })),
      update: vi.fn(),
    };
    const sink = effects();
    const store = useSettingsStore();

    const first = store.initialize(service, sink);
    await store.retryInitialize(service, sink);
    old.reject({ code: "E_INTERNAL", message: "old failure" });
    await first;

    expect(store.status).toBe("ready");
    expect(store.snapshot?.revision).toBe(4);
    expect(store.values?.theme).toBe("dark");
    expect(sink.applyAppearance).not.toHaveBeenCalledWith({
      theme: "system",
      locale: "zh-CN",
    });
  });

  it("reports current load failures and allows retry", async () => {
    const service: SettingsService = {
      getSnapshot: vi.fn()
        .mockRejectedValueOnce({ code: "E_INTERNAL", message: "failed" })
        .mockResolvedValueOnce(makeSnapshot()),
      update: vi.fn(),
    };
    const sink = effects();
    const store = useSettingsStore();

    await store.initialize(service, sink);
    expect(store.status).toBe("load_error");
    expect(store.loadError?.code).toBe("E_INTERNAL");
    expect(sink.applyAppearance).toHaveBeenLastCalledWith({
      theme: "system",
      locale: "zh-CN",
    });
    await store.retryInitialize(service, sink);
    expect(store.status).toBe("ready");
  });

  it("coalesces A-B-C edits and persists the final field intent", async () => {
    const first = deferred<SettingsSnapshot>();
    const last = deferred<SettingsSnapshot>();
    const update = vi.fn()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(last.promise);
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update,
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);

    const a = store.commit({ field: "maxConcurrentTasks", value: 4 }, service, sink);
    const b = store.commit({ field: "maxConcurrentTasks", value: 5 }, service, sink);
    const c = store.commit({ field: "maxConcurrentTasks", value: 6 }, service, sink);
    expect(update).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenLastCalledWith({ field: "maxConcurrentTasks", value: 4 });
    first.resolve(makeSnapshot(1, { maxConcurrentTasks: 4 }));
    await Promise.resolve();
    expect(update).toHaveBeenCalledTimes(2);
    expect(update).toHaveBeenLastCalledWith({ field: "maxConcurrentTasks", value: 6 });
    last.resolve(makeSnapshot(2, { maxConcurrentTasks: 6 }));
    await Promise.all([a, b, c]);

    expect(store.values?.maxConcurrentTasks).toBe(6);
    expect(store.snapshot?.values.maxConcurrentTasks).toBe(6);
    expect(store.fieldStates.maxConcurrentTasks.status).toBe("saved");
  });

  it("previews a range value without invoking persistence", async () => {
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn(),
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);

    store.preview({ field: "connectionsPerTask", value: 16 }, sink);

    expect(store.values?.connectionsPerTask).toBe(16);
    expect(store.fieldStates.connectionsPerTask.status).toBe("preview");
    expect(service.update).not.toHaveBeenCalled();
  });

  it("keeps different fields concurrent and ignores an older full snapshot", async () => {
    const theme = deferred<SettingsSnapshot>();
    const locale = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn((patch) => patch.field === "theme" ? theme.promise : locale.promise),
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);

    const themeSave = store.commit({ field: "theme", value: "dark" }, service, sink);
    const localeSave = store.commit({ field: "locale", value: "en-US" }, service, sink);
    expect(service.update).toHaveBeenCalledTimes(2);
    locale.resolve(makeSnapshot(2, { theme: "dark", locale: "en-US" }));
    await localeSave;
    theme.resolve(makeSnapshot(1, { theme: "dark" }));
    await themeSave;

    expect(store.snapshot?.revision).toBe(2);
    expect(store.values?.theme).toBe("dark");
    expect(store.values?.locale).toBe("en-US");
  });

  it("rolls back only the current failed intent and its appearance", async () => {
    const failure = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn().mockReturnValue(failure.promise),
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);

    const saving = store.commit({ field: "theme", value: "dark" }, service, sink);
    expect(store.values?.theme).toBe("dark");
    expect(sink.applyAppearance).toHaveBeenLastCalledWith({ theme: "dark", locale: "zh-CN" });
    failure.reject({ code: "E_INTERNAL", message: "disk failed" });
    await saving;

    expect(store.values?.theme).toBe("system");
    expect(store.fieldStates.theme.status).toBe("error");
    expect(sink.applyAppearance).toHaveBeenLastCalledWith({ theme: "system", locale: "zh-CN" });
  });

  it("does not let an old failure roll back a newer desired value", async () => {
    const old = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn()
        .mockReturnValueOnce(old.promise)
        .mockResolvedValueOnce(makeSnapshot(1, { theme: "light" })),
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);

    const dark = store.commit({ field: "theme", value: "dark" }, service, sink);
    const light = store.commit({ field: "theme", value: "light" }, service, sink);
    old.reject({ code: "E_INTERNAL", message: "old failure" });
    await Promise.all([dark, light]);

    expect(store.values?.theme).toBe("light");
    expect(store.snapshot?.values.theme).toBe("light");
    expect(store.fieldStates.theme.status).toBe("saved");
  });

  it("a saved timer cannot clear a newer saving generation", async () => {
    vi.useFakeTimers();
    const second = deferred<SettingsSnapshot>();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn()
        .mockResolvedValueOnce(makeSnapshot(1, { maxConcurrentTasks: 4 }))
        .mockReturnValueOnce(second.promise),
    };
    const sink = effects();
    const store = useSettingsStore();
    await store.initialize(service, sink);
    await store.commit({ field: "maxConcurrentTasks", value: 4 }, service, sink);
    expect(store.fieldStates.maxConcurrentTasks.status).toBe("saved");

    const saving = store.commit({ field: "maxConcurrentTasks", value: 5 }, service, sink);
    await vi.advanceTimersByTimeAsync(2_000);
    expect(store.fieldStates.maxConcurrentTasks.status).toBe("saving");
    second.resolve(makeSnapshot(2, { maxConcurrentTasks: 5 }));
    await saving;
  });

  it("clears saved status timers when the root store is disposed", async () => {
    vi.useFakeTimers();
    const service: SettingsService = {
      getSnapshot: vi.fn().mockResolvedValue(makeSnapshot()),
      update: vi.fn().mockResolvedValue(makeSnapshot(1, { maxConcurrentTasks: 4 })),
    };
    const store = useSettingsStore();
    await store.initialize(service, effects());
    await store.commit({ field: "maxConcurrentTasks", value: 4 }, service, effects());
    expect(vi.getTimerCount()).toBe(1);

    store.dispose();

    expect(vi.getTimerCount()).toBe(0);
  });
});
