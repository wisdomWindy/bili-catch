import { describe, expect, it, vi } from "vitest";
import { createTaskEventSource } from "./events";

describe("task event source", () => {
  it("subscribes to both stable event names and releases both listeners", async () => {
    const first = vi.fn();
    const second = vi.fn();
    const listen = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second);
    const source = createTaskEventSource(listen);
    const unsubscribe = await source.subscribe(vi.fn(), vi.fn());

    expect(listen.mock.calls.map((call) => call[0])).toEqual([
      "download://progress",
      "download://removed",
    ]);
    unsubscribe();
    expect(first).toHaveBeenCalledOnce();
    expect(second).toHaveBeenCalledOnce();
  });
});
