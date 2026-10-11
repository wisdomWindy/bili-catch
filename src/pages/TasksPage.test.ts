import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import type { TaskEventSource } from "../services/ipc/events";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import type { DownloadTask } from "../features/task-management/contracts";
import type { TaskService } from "../features/task-management/service";
import { useTaskManagementStore } from "../features/task-management/store";
import type { TranscriptionEventSource } from "../features/transcription/events";
import { transcriptionEventSourceKey, transcriptionServiceKey } from "../features/transcription/injection";
import type { TranscriptionService } from "../features/transcription/service";
import { useTranscriptionStore } from "../features/transcription/store";
import TasksPage from "./TasksPage.vue";

const task: DownloadTask = {
  id: "task-1", revision: 1, createdAt: "2026-09-10T00:00:00Z", updatedAt: "2026-09-10T00:00:00Z",
  fileName: "P1.mp4", outputDir: "Downloads", outputPath: null, bvid: "BV1", cid: 1, page: 1,
  partTitle: "P1", mode: "video-only", qualityId: "80", codec: "avc", audioFormat: null,
  audioBitrateId: null, status: "queued", controlRequest: "none", progressPercent: 0,
  autoTranscribe: false, transcriptionPreset: null,
  bytesDownloaded: "0", totalBytes: null, speedBytesPerSecond: "0", etaSeconds: null,
  automaticRetryCount: 0, nextRetryAt: null, error: null,
};

async function render(list: TaskService["list"], subscribe: TaskEventSource["subscribe"] = async () => () => undefined) {
  const router = createAppRouter(createMemoryHistory()); await router.push("/tasks"); await router.isReady();
  const service = { list, create: vi.fn(), control: vi.fn(), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn() } as TaskService;
  const wrapper = mount(TasksPage, { global: { plugins: [createPinia(), router, createAppI18n()], provide: { [taskServiceKey as symbol]: service, [taskEventSourceKey as symbol]: { subscribe } } } });
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => window.setTimeout(resolve, 0)));
  return wrapper;
}

async function renderWithInitializationSpies() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const taskInitialization = vi.spyOn(useTaskManagementStore(pinia), "initialize").mockResolvedValue(undefined);
  const transcriptionInitialization = vi.spyOn(useTranscriptionStore(pinia), "initialize").mockResolvedValue(undefined);
  const router = createAppRouter(createMemoryHistory());
  await router.push("/tasks");
  await router.isReady();
  const taskService = { list: vi.fn() } as unknown as TaskService;
  const taskEvents = { subscribe: vi.fn() } as TaskEventSource;
  const transcriptionService = { listTasks: vi.fn(), listModels: vi.fn() } as unknown as TranscriptionService;
  const transcriptionEvents = { subscribe: vi.fn() } as TranscriptionEventSource;
  const wrapper = mount(TasksPage, {
    global: {
      plugins: [pinia, router, createAppI18n()],
      provide: {
        [taskServiceKey as symbol]: taskService,
        [taskEventSourceKey as symbol]: taskEvents,
        [transcriptionServiceKey as symbol]: transcriptionService,
        [transcriptionEventSourceKey as symbol]: transcriptionEvents,
      },
    },
  });
  return { taskInitialization, transcriptionInitialization, wrapper };
}

afterEach(() => vi.useRealTimers());

async function runScheduledInitialization() {
  vi.advanceTimersToNextFrame();
  await vi.runOnlyPendingTimersAsync();
}

describe("TasksPage states", () => {
  it("does not project cached tasks before the first page frame", async () => {
    vi.useFakeTimers();
    const pinia = createPinia();
    setActivePinia(pinia);
    const taskStore = useTaskManagementStore(pinia);
    taskStore.byId[task.id] = task;
    vi.spyOn(taskStore, "initialize").mockResolvedValue(undefined);
    vi.spyOn(useTranscriptionStore(pinia), "initialize").mockResolvedValue(undefined);
    const router = createAppRouter(createMemoryHistory());
    await router.push("/tasks");
    await router.isReady();

    const wrapper = mount(TasksPage, {
      global: {
        plugins: [pinia, router, createAppI18n()],
        provide: {
          [taskServiceKey as symbol]: { list: vi.fn() } as unknown as TaskService,
          [taskEventSourceKey as symbol]: { subscribe: vi.fn() } as TaskEventSource,
          [transcriptionServiceKey as symbol]: { listTasks: vi.fn(), listModels: vi.fn() } as unknown as TranscriptionService,
          [transcriptionEventSourceKey as symbol]: { subscribe: vi.fn() } as TranscriptionEventSource,
        },
      },
    });

    expect(wrapper.find("[aria-label='Loading tasks']").exists()).toBe(true);
    expect(wrapper.text()).not.toContain(task.fileName);

    await runScheduledInitialization();
    await flushPromises();
    expect(wrapper.text()).toContain(task.fileName);
    wrapper.unmount();
  });

  it("renders before starting task initialization", async () => {
    vi.useFakeTimers();
    const { taskInitialization, transcriptionInitialization, wrapper } = await renderWithInitializationSpies();

    expect(wrapper.get("[data-testid='page-heading']").text()).toBe("任务列表");
    expect(taskInitialization).not.toHaveBeenCalled();
    expect(transcriptionInitialization).not.toHaveBeenCalled();

    await runScheduledInitialization();
    expect(taskInitialization).toHaveBeenCalledOnce();
    expect(transcriptionInitialization).toHaveBeenCalledOnce();
    wrapper.unmount();
  });

  it("cancels pending task initialization when leaving the page", async () => {
    vi.useFakeTimers();
    const { taskInitialization, transcriptionInitialization, wrapper } = await renderWithInitializationSpies();

    wrapper.unmount();
    await runScheduledInitialization();

    expect(taskInitialization).not.toHaveBeenCalled();
    expect(transcriptionInitialization).not.toHaveBeenCalled();
  });

  it("shows a stable skeleton while the initial snapshot is pending", async () => {
    const wrapper = await render(() => new Promise(() => undefined));
    await flushPromises();
    expect(wrapper.get("[aria-label='Loading tasks']").findAll("span")).toHaveLength(5);
    wrapper.unmount();
  });

  it("shows an actionable empty state for an empty snapshot", async () => {
    const wrapper = await render(async () => ({ sequence: 0, capacity: 100, tasks: [] })); await flushPromises();
    expect(wrapper.text()).toContain("当前没有下载任务");
    expect(wrapper.findAll("button").some((button) => button.text().includes("返回下载中心"))).toBe(true);
  });

  it("keeps snapshot rows visible when event subscription fails", async () => {
    const wrapper = await render(async () => ({ sequence: 1, capacity: 100, tasks: [task] }), async () => { throw "event offline"; }); await flushPromises();
    expect(wrapper.text()).toContain("P1.mp4");
    expect(wrapper.get("[role='status']").text()).not.toContain("event offline");
  });

  it("shows the load error instead of an empty success state", async () => {
    const wrapper = await render(async () => { throw { code: "E007", message: "read failed" }; }); await flushPromises();
    expect(wrapper.text()).toContain("任务读取失败");
    expect(wrapper.text()).not.toContain("read failed");
  });
});
