import { describe, expect, it, vi } from "vitest";
import type { IpcTransport } from "../../contracts/ipc";
import { createTaskService } from "./service";

describe("task service transport contract", () => {
  it("uses the six exact command names and nested request arguments", async () => {
    const invoke = vi.fn().mockResolvedValue({ sequence: 0, capacity: 100, tasks: [] });
    const service = createTaskService({ invoke } as IpcTransport);

    await service.list();
    await service.create({ requestId: "batch_1", drafts: [] });
    await service.control({ taskId: "task-1", action: "pause" });
    await service.pauseAll();
    await service.clearCompleted();
    await service.open({ taskId: "task-1", target: "file" });

    expect(invoke.mock.calls).toEqual([
      ["list_download_tasks"],
      ["create_download_tasks", { request: { requestId: "batch_1", drafts: [] } }],
      ["control_download_task", { request: { taskId: "task-1", action: "pause" } }],
      ["pause_all_download_tasks"],
      ["clear_completed_tasks"],
      ["open_download_task", { request: { taskId: "task-1", target: "file" } }],
    ]);
  });

  it("normalizes unknown IPC failures", async () => {
    const service = createTaskService({ invoke: vi.fn().mockRejectedValue("offline") });
    await expect(service.list()).rejects.toMatchObject({ code: "E_INTERNAL", message: "offline" });
  });
});
