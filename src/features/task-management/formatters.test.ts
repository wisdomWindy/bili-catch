import { describe, expect, it } from "vitest";
import { compareTasks, formatBytes, formatEta, formatSpeed } from "./formatters";
import type { DownloadTask } from "./contracts";

describe("task formatters", () => {
  it("formats byte strings without losing integer precision", () => {
    expect(formatBytes("9007199254740993")).toBe("8 PiB");
    expect(formatBytes("1536")).toBe("1.5 KiB");
    expect(formatSpeed("1048576")).toBe("1 MiB/s");
    expect(formatBytes(null)).toBe("—");
  });

  it("formats ETA in stable minute and hour forms", () => {
    expect(formatEta(65)).toBe("01:05");
    expect(formatEta(3661)).toBe("1:01:01");
    expect(formatEta(null)).toBe("—");
  });

  it("sorts active tasks first and then by creation time descending", () => {
    const make = (id: string, status: DownloadTask["status"], createdAt: string) =>
      ({ id, status, createdAt }) as DownloadTask;
    const tasks = [
      make("old-active", "queued", "2026-09-09T00:00:00Z"),
      make("new-terminal", "completed", "2026-09-10T02:00:00Z"),
      make("new-active", "downloading", "2026-09-10T01:00:00Z"),
    ];

    expect(tasks.sort(compareTasks).map((task) => task.id)).toEqual([
      "new-active",
      "old-active",
      "new-terminal",
    ]);
  });
});
