import type { TaskAction, TaskStatus } from "./contracts";

export function actionsForStatus(
  status: TaskStatus,
  canCancelProcessing = false,
): TaskAction[] {
  switch (status) {
    case "queued":
      return ["cancel"];
    case "downloading":
      return ["pause", "cancel"];
    case "paused":
      return ["resume", "cancel"];
    case "processing":
      return canCancelProcessing ? ["cancel"] : [];
    case "failed":
      return ["retry", "delete"];
    case "completed":
    case "cancelled":
      return ["delete"];
  }
}
