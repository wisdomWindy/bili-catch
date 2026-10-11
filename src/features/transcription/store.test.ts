import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TranscriptionEventSource } from "./events";
import type { AsrModel, TranscribeTask } from "./contracts";
import type { TranscriptionService } from "./service";
import { useTranscriptionStore } from "./store";

const task = (revision = 0): TranscribeTask => ({
  id: "task-1", revision,
  source: { kind: "localFile", path: "D:/voice.wav" },
  sourceInfo: { fileName: "voice.wav", path: "D:/voice.wav", durationSeconds: 10, sizeBytes: 10, mediaKind: "audio", format: "wav" },
  options: { modelId: "base", language: "auto", outputFormat: "srt", timestamps: true, punctuation: true, outputDirectory: "D:/out", gpuMode: "auto" },
  status: "pending", progress: { progress: 0, processedSeconds: 0, message: "" }, cacheReused: false,
  createdAt: "2026-10-10T00:00:00Z", updatedAt: "2026-10-10T00:00:00Z",
});

const model = (revision: number, downloadState: AsrModel["downloadState"]): AsrModel => ({
  id: "base",
  displayName: "Base",
  fileName: "ggml-base.bin",
  sizeBytes: 100,
  downloadedBytes: downloadState === "completed" ? 100 : 50,
  installed: downloadState === "completed",
  downloadState,
  valid: downloadState === "completed",
  isDefault: true,
  inUse: false,
  recommendedFor: "test",
  capabilities: ["test"],
  revision,
});

describe("transcription store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("reuses one initialization while the same runtime is hydrating and ready", async () => {
    let resolveTasks!: (value: { sequence: number; tasks: TranscribeTask[] }) => void;
    const listTasks = vi.fn(() => new Promise<{ sequence: number; tasks: TranscribeTask[] }>((resolve) => {
      resolveTasks = resolve;
    }));
    const listModels = vi.fn(async () => ({ sequence: 0, models: [] }));
    const subscribe = vi.fn(async () => vi.fn());
    const service = { listTasks, listModels } as unknown as TranscriptionService;
    const events: TranscriptionEventSource = { subscribe };
    const store = useTranscriptionStore();

    const first = store.initialize(service, events);
    await Promise.resolve();
    const second = store.initialize(service, events);

    expect(subscribe).toHaveBeenCalledOnce();
    expect(listTasks).toHaveBeenCalledOnce();
    expect(listModels).toHaveBeenCalledOnce();
    resolveTasks({ sequence: 0, tasks: [] });
    await Promise.all([first, second]);
    await store.initialize(service, events);
    expect(subscribe).toHaveBeenCalledOnce();
    expect(listTasks).toHaveBeenCalledOnce();
    expect(listModels).toHaveBeenCalledOnce();
  });

  it("releases a subscription that resolves after the store is disposed", async () => {
    let resolveSubscribe!: (unsubscribe: () => void) => void;
    const unsubscribe = vi.fn();
    const events: TranscriptionEventSource = {
      subscribe: vi.fn(() => new Promise<() => void>((resolve) => { resolveSubscribe = resolve; })),
    };
    const listTasks = vi.fn(async () => ({ sequence: 0, tasks: [] }));
    const listModels = vi.fn(async () => ({ sequence: 0, models: [] }));
    const service = { listTasks, listModels } as unknown as TranscriptionService;
    const store = useTranscriptionStore();

    const initializing = store.initialize(service, events);
    store.dispose();
    resolveSubscribe(unsubscribe);
    await initializing;

    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(listTasks).not.toHaveBeenCalled();
    expect(listModels).not.toHaveBeenCalled();
  });

  it("buffers events during hydration and rejects stale revisions after removal", async () => {
    let progress!: (event: { sequence: number; task: TranscribeTask }) => void;
    let removed!: (event: { sequence: number; taskId: string }) => void;
    const unsubscribe = vi.fn();
    const events: TranscriptionEventSource = {
      subscribe: vi.fn(async (onProgress, onRemoved) => {
        progress = onProgress; removed = onRemoved; return unsubscribe;
      }),
    };
    let resolveTasks!: (value: { sequence: number; tasks: TranscribeTask[] }) => void;
    const service = {
      listTasks: vi.fn(() => new Promise((resolve) => { resolveTasks = resolve; })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [] })),
    } as unknown as TranscriptionService;
    const store = useTranscriptionStore();
    const initializing = store.initialize(service, events);
    await Promise.resolve();
    progress({ sequence: 3, task: { ...task(2), status: "transcribing" } });
    resolveTasks({ sequence: 1, tasks: [task()] });
    await initializing;
    expect(store.byId["task-1"].status).toBe("transcribing");
    removed({ sequence: 4, taskId: "task-1" });
    progress({ sequence: 3, task: task(99) });
    expect(store.byId["task-1"]).toBeUndefined();
    store.dispose();
    expect(unsubscribe).toHaveBeenCalledOnce();
  });

  it("does not let a late model command response overwrite a newer event", async () => {
    let modelChanged!: (event: { sequence: number; model: AsrModel }) => void;
    let resolveDownload!: (value: AsrModel) => void;
    const events: TranscriptionEventSource = {
      subscribe: vi.fn(async (_onProgress, _onRemoved, onModel) => {
        modelChanged = onModel;
        return vi.fn();
      }),
    };
    const service = {
      listTasks: vi.fn(async () => ({ sequence: 0, tasks: [] })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [model(0, "idle")] })),
      downloadModel: vi.fn(() => new Promise<AsrModel>((resolve) => { resolveDownload = resolve; })),
    } as unknown as TranscriptionService;
    const store = useTranscriptionStore();
    await store.initialize(service, events);

    const pending = store.downloadModel("base");
    expect(store.modelsById.base?.downloadState).toBe("queued");
    modelChanged({ sequence: 2, model: model(2, "completed") });
    resolveDownload(model(1, "downloading"));
    await pending;

    expect(store.modelsById.base?.downloadState).toBe("completed");
    expect(store.modelsById.base?.revision).toBe(2);
  });

  it("shows a failed state when a model download request fails", async () => {
    const failure = { code: "E012", message: "download failed" } as const;
    const events: TranscriptionEventSource = {
      subscribe: vi.fn(async () => vi.fn()),
    };
    const service = {
      listTasks: vi.fn(async () => ({ sequence: 0, tasks: [] })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [model(0, "idle")] })),
      downloadModel: vi.fn(async () => Promise.reject(failure)),
    } as unknown as TranscriptionService;
    const store = useTranscriptionStore();
    await store.initialize(service, events);

    await store.downloadModel("base");

    expect(store.modelsById.base?.downloadState).toBe("failed");
    expect(store.modelsById.base?.error?.code).toBe("E012");
  });
});
