import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import type { TaskEventSource } from "../services/ipc/events";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import type { DownloadTask } from "../features/task-management/contracts";
import type { TaskService } from "../features/task-management/service";
import TasksPage from "./TasksPage.vue";

const task: DownloadTask = {
  id: "task-1", revision: 1, createdAt: "2026-09-10T00:00:00Z", updatedAt: "2026-09-10T00:00:00Z",
  fileName: "P1.mp4", outputDir: "Downloads", outputPath: null, bvid: "BV1", cid: 1, page: 1,
  partTitle: "P1", mode: "video-only", qualityId: "80", codec: "avc", audioFormat: null,
  audioBitrateId: null, status: "queued", controlRequest: "none", progressPercent: 0,
  bytesDownloaded: "0", totalBytes: null, speedBytesPerSecond: "0", etaSeconds: null,
  automaticRetryCount: 0, nextRetryAt: null, error: null,
};

async function render(list: TaskService["list"], subscribe: TaskEventSource["subscribe"] = async () => () => undefined) {
  const router = createAppRouter(createMemoryHistory()); await router.push("/tasks"); await router.isReady();
  const service = { list, create: vi.fn(), control: vi.fn(), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn() } as TaskService;
  return mount(TasksPage, { global: { plugins: [createPinia(), router, createAppI18n()], provide: { [taskServiceKey as symbol]: service, [taskEventSourceKey as symbol]: { subscribe } } } });
}

describe("TasksPage states", () => {
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
