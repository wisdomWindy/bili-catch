import type { TaskEventSource } from "../../services/ipc/events";
import type { DownloadTask, TaskStatus } from "./contracts";
import type { TaskService } from "./service";

const statuses: TaskStatus[] = ["downloading", "queued", "paused", "processing", "completed", "failed", "cancelled"];
const makeTask = (status: TaskStatus, index: number): DownloadTask => ({
  id: `demo:${index}`, revision: 1, createdAt: `2026-09-10T0${8 - index}:00:00Z`, updatedAt: "2026-09-10T08:10:00Z",
  fileName: index === 0 ? "Vue 3 工程实战：从组件设计到可维护架构.mp4" : `BiliCatch 测试分 P ${index + 1}.mp4`,
  outputDir: "D:/Downloads/BiliCatch", outputPath: status === "completed" ? "D:/Downloads/BiliCatch/demo.mp4" : null,
  bvid: "BV1xx411c7BF", cid: 1000 + index, page: index + 1, partTitle: `P${index + 1}`,
  mode: "video-audio", qualityId: "80", codec: "avc", audioFormat: null, audioBitrateId: null,
  status, controlRequest: "none", progressPercent: [62, 0, 38, 100, 100, 24, 12][index] ?? 0,
  bytesDownloaded: ["650117120", "0", "398458880", "1048576000", "1572864000", "251658240", "125829120"][index] ?? "0",
  totalBytes: "1572864000", speedBytesPerSecond: status === "downloading" ? "8388608" : "0", etaSeconds: status === "downloading" ? 112 : null,
  automaticRetryCount: status === "failed" ? 3 : 0, nextRetryAt: null,
  error: status === "failed" ? { code: "E001", message: "网络连接中断", details: "3 automatic retries exhausted" } : null,
});

export function createDemoTaskRuntime(mode: "normal" | "empty" | "error" = "normal"): { service: TaskService; events: TaskEventSource } {
  let sequence = 20;
  let tasks = mode === "empty" ? [] : statuses.map(makeTask);
  const service: TaskService = {
    list: async () => {
      if (mode === "error") throw { code: "E007", message: "无法读取本地任务" };
      return { sequence, capacity: 100, tasks: [...tasks] };
    },
    create: async (request) => ({ sequence, capacity: 100, tasks: tasks.filter((task) => task.id.startsWith(request.requestId)), reused: false }),
    control: async ({ taskId, action }) => {
      const item = tasks.find((task) => task.id === taskId);
      if (!item) throw { code: "E_INTERNAL", message: "Task not found" };
      if (action === "delete") { tasks = tasks.filter((task) => task.id !== taskId); sequence += 1; return item; }
      const transitions: Partial<Record<typeof action, TaskStatus>> = { cancel: "cancelled", retry: "queued", pause: "paused", resume: "downloading" };
      Object.assign(item, { status: transitions[action] ?? item.status, revision: item.revision + 1 }); sequence += 1; return { ...item };
    },
    pauseAll: async () => ({ sequence, affectedTaskIds: [], failures: [] }),
    clearCompleted: async () => { const ids = tasks.filter((task) => task.status === "completed").map((task) => task.id); tasks = tasks.filter((task) => task.status !== "completed"); sequence += ids.length; return { sequence, removedTaskIds: ids }; },
    open: async () => undefined,
  };
  return { service, events: { subscribe: async () => () => undefined } };
}
