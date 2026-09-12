import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DownloadTask, TaskProgressEvent, TaskRemovedEvent } from "./contracts";
import type { TaskEventSource } from "../../services/ipc/events";
import { useTaskManagementStore } from "./store";

const task = (id: string, status: DownloadTask["status"], revision = 1): DownloadTask => ({
  id, revision, createdAt: "2026-09-10T00:00:00Z", updatedAt: "2026-09-10T00:00:00Z",
  fileName: `${id}.mp4`, outputDir: "Downloads", outputPath: null, bvid: "BV1", cid: 1,
  page: 1, partTitle: id, mode: "video-only", qualityId: "80", codec: "avc",
  audioFormat: null, audioBitrateId: null, status, controlRequest: "none", progressPercent: 0,
  bytesDownloaded: "0", totalBytes: null, speedBytesPerSecond: "0", etaSeconds: null,
  automaticRetryCount: 0, nextRetryAt: null, error: null,
});

describe("task management store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("replays newer buffered events after the snapshot and ignores older events", async () => {
    let progress!: (event: TaskProgressEvent) => void;
    let removed!: (event: TaskRemovedEvent) => void;
    const events: TaskEventSource = { subscribe: vi.fn(async (onProgress, onRemoved) => {
      progress = onProgress; removed = onRemoved; return vi.fn();
    }) };
    const service = {
      list: vi.fn(async () => {
        progress({ sequence: 12, task: task("new", "downloading", 2) });
        return { sequence: 10, capacity: 100 as const, tasks: [task("old", "queued")] };
      }),
      create: vi.fn(), control: vi.fn(), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn(),
    };
    const store = useTaskManagementStore();
    await store.initialize(service, events);
    removed({ sequence: 11, taskId: "new" });

    expect(store.sequence).toBe(12);
    expect(store.visibleTasks.map((item) => item.id)).toEqual(["new", "old"]);
    expect(store.status).toBe("ready");
  });

  it("derives four filters without mutating the authoritative collection", async () => {
    const events: TaskEventSource = { subscribe: vi.fn(async () => vi.fn()) };
    const all = [task("a", "queued"), task("b", "completed"), task("c", "failed"), task("d", "cancelled")];
    const service = { list: vi.fn(async () => ({ sequence: 1, capacity: 100 as const, tasks: all })), create: vi.fn(), control: vi.fn(), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn() };
    const store = useTaskManagementStore();
    await store.initialize(service, events);

    store.filter = "active"; expect(store.visibleTasks.map((item) => item.id)).toEqual(["a"]);
    store.filter = "completed"; expect(store.visibleTasks.map((item) => item.id)).toEqual(["b"]);
    store.filter = "failed"; expect(store.visibleTasks.map((item) => item.id)).toEqual(["c"]);
    expect(store.counts.all).toBe(4);
  });

  it("clears row pending and keeps the authoritative task when a command fails", async () => {
    const events: TaskEventSource = { subscribe: vi.fn(async () => vi.fn()) };
    const service = { list: vi.fn(async () => ({ sequence: 1, capacity: 100 as const, tasks: [task("a", "queued")] })), create: vi.fn(), control: vi.fn(async () => { throw { code: "E008", message: "save failed" }; }), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn() };
    const store = useTaskManagementStore(); await store.initialize(service, events);
    await store.control("a", "cancel");

    expect(store.pending.a).toBeUndefined();
    expect(store.byId.a.status).toBe("queued");
    expect(store.error?.message).toBe("save failed");
  });

  it("keeps a newer command result when a delayed progress event arrives", async () => {
    let progress!: (event: TaskProgressEvent) => void;
    const events: TaskEventSource = { subscribe: vi.fn(async (onProgress) => {
      progress = onProgress; return vi.fn();
    }) };
    const service = {
      list: vi.fn(async () => ({ sequence: 1, capacity: 100 as const, tasks: [task("a", "downloading", 1)] })),
      create: vi.fn(),
      control: vi.fn(async () => task("a", "paused", 3)),
      pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn(),
    };
    const store = useTaskManagementStore(); await store.initialize(service, events);
    await store.control("a", "pause");
    progress({ sequence: 2, task: task("a", "downloading", 2) });

    expect(store.sequence).toBe(2);
    expect(store.byId.a.status).toBe("paused");
    expect(store.byId.a.revision).toBe(3);
  });

  it("does not resurrect a deleted task while its removal event is in flight", async () => {
    let progress!: (event: TaskProgressEvent) => void;
    let removed!: (event: TaskRemovedEvent) => void;
    const events: TaskEventSource = { subscribe: vi.fn(async (onProgress, onRemoved) => {
      progress = onProgress; removed = onRemoved; return vi.fn();
    }) };
    const service = {
      list: vi.fn(async () => ({ sequence: 1, capacity: 100 as const, tasks: [task("a", "completed", 2)] })),
      create: vi.fn(),
      control: vi.fn(async () => task("a", "completed", 2)),
      pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn(),
    };
    const store = useTaskManagementStore(); await store.initialize(service, events);
    await store.control("a", "delete");
    progress({ sequence: 2, task: task("a", "completed", 2) });

    expect(store.byId.a).toBeUndefined();
    expect(store.sequence).toBe(2);

    removed({ sequence: 3, taskId: "a" });
    expect(store.byId.a).toBeUndefined();
  });
});
