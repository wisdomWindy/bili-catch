import { describe, expect, it } from "vitest";
import { actionsForStatus } from "./action-policy";
import type { TaskStatus } from "./contracts";

describe("task action policy", () => {
  it.each<[TaskStatus, string[]]>([
    ["queued", ["cancel"]],
    ["downloading", ["pause", "cancel"]],
    ["paused", ["resume", "cancel"]],
    ["processing", []],
    ["failed", ["retry", "delete"]],
    ["completed", ["delete"]],
    ["cancelled", ["delete"]],
  ])("returns the approved actions for %s", (status, expected) => {
    expect(actionsForStatus(status)).toEqual(expected);
  });

  it("offers processing cancellation only when the executor can interrupt", () => {
    expect(actionsForStatus("processing", true)).toEqual(["cancel"]);
  });
});
