import { describe, expect, it } from "vitest";
import type {
  CreateDownloadTasksRequest,
  DownloadTask,
  TaskAction,
  TaskProgressEvent,
} from "./contracts";

describe("task management contracts", () => {
  it("preserves the task snapshot wire shape without numeric byte coercion", () => {
    const task: DownloadTask = {
      id: "batch-1:0",
      revision: 3,
      createdAt: "2026-09-10T00:00:00Z",
      updatedAt: "2026-09-10T00:01:00Z",
      fileName: "P1.mp4",
      outputDir: "Downloads",
      outputPath: null,
      bvid: "BV1xx411c7BF",
      cid: 1001,
      page: 1,
      partTitle: "P1",
      mode: "video-audio",
      qualityId: "80",
      codec: "avc",
      audioFormat: "m4a",
      audioBitrateId: null,
      status: "downloading",
      controlRequest: "none",
      progressPercent: 50,
      bytesDownloaded: "9007199254740993",
      totalBytes: "18014398509481986",
      speedBytesPerSecond: "1048576",
      etaSeconds: 30,
      automaticRetryCount: 1,
      nextRetryAt: null,
      error: null,
    };

    const event: TaskProgressEvent = { sequence: 7, task };

    expect(event.task.bytesDownloaded).toBe("9007199254740993");
    expect(event.task.status).toBe("downloading");
    expect(event.sequence).toBe(7);
  });

  it("keeps command requests nested and action values stable", () => {
    const request: CreateDownloadTasksRequest = {
      requestId: "handoff_20260910",
      drafts: [],
    };
    const actions: TaskAction[] = ["pause", "resume", "cancel", "retry", "delete"];

    expect({ request }).toEqual({
      request: { requestId: "handoff_20260910", drafts: [] },
    });
    expect(actions).toEqual(["pause", "resume", "cancel", "retry", "delete"]);
  });
});
