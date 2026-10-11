import type { DownloadTask } from "./contracts";
import type { TranscribeTask } from "../transcription/contracts";

export type CombinedTaskFilter = "all" | "download" | "transcription" | "active" | "completed" | "failed";
export type TaskStatusGroup = "active" | "completed" | "failed";

interface TaskListItemBase {
  id: string;
  name: string;
  statusGroup: TaskStatusGroup;
  statusLabel: string;
  progress: number;
  primaryMetric: string;
  secondaryMetric: string;
  updatedAt: string;
}

export interface DownloadTaskListItem extends TaskListItemBase {
  kind: "download";
  task: DownloadTask;
}

export interface TranscriptionTaskListItem extends TaskListItemBase {
  kind: "transcription";
  task: TranscribeTask;
}

export type TaskListItem = DownloadTaskListItem | TranscriptionTaskListItem;

const terminalFailures = new Set(["failed", "cancelled"]);

export function projectDownloadTask(task: DownloadTask): DownloadTaskListItem {
  return {
    id: `download:${task.id}`,
    name: task.fileName,
    kind: "download",
    statusGroup: task.status === "completed" ? "completed" : terminalFailures.has(task.status) ? "failed" : "active",
    statusLabel: `tasks.statuses.${task.status}`,
    progress: task.progressPercent,
    primaryMetric: task.speedBytesPerSecond,
    secondaryMetric: task.bytesDownloaded,
    updatedAt: task.updatedAt,
    task,
  };
}

export function projectTranscriptionTask(task: TranscribeTask): TranscriptionTaskListItem {
  return {
    id: `transcription:${task.id}`,
    name: task.sourceInfo.fileName,
    kind: "transcription",
    statusGroup: task.status === "completed" ? "completed" : terminalFailures.has(task.status) ? "failed" : "active",
    statusLabel: `tasks.transcriptionStatuses.${task.status}`,
    progress: Math.round(task.progress.progress * 100),
    primaryMetric: task.progress.speed ? `${task.progress.speed.toFixed(1)}x` : "",
    secondaryMetric: `${Math.round(task.progress.processedSeconds)}s`,
    updatedAt: task.updatedAt,
    task,
  };
}

export function matchesCombinedFilter(item: TaskListItem, filter: CombinedTaskFilter): boolean {
  if (filter === "all") return true;
  if (filter === "download" || filter === "transcription") return item.kind === filter;
  return item.statusGroup === filter;
}
