import type { AppError } from "../../contracts/app-error";
import type {
  AudioFormat,
  DownloadMode,
  DownloadTaskDraft,
  VideoCodec,
} from "../download-center/contracts";

export type TaskStatus =
  | "queued"
  | "downloading"
  | "paused"
  | "processing"
  | "completed"
  | "failed"
  | "cancelled";

export type TaskControlRequest =
  | "none"
  | "pauseRequested"
  | "resumeRequested"
  | "cancelRequested";

export type TaskAction = "pause" | "resume" | "cancel" | "retry" | "delete";
export type OpenTarget = "file" | "directory";

export interface DownloadTask {
  id: string;
  revision: number;
  createdAt: string;
  updatedAt: string;
  fileName: string;
  outputDir: string;
  outputPath: string | null;
  bvid: string;
  cid: number;
  page: number;
  partTitle: string;
  mode: DownloadMode;
  qualityId: string | null;
  codec: VideoCodec | null;
  audioFormat: AudioFormat | null;
  audioBitrateId: string | null;
  status: TaskStatus;
  controlRequest: TaskControlRequest;
  progressPercent: number;
  bytesDownloaded: string;
  totalBytes: string | null;
  speedBytesPerSecond: string;
  etaSeconds: number | null;
  automaticRetryCount: number;
  nextRetryAt: string | null;
  error: AppError | null;
}

export interface CreateDownloadTasksRequest {
  requestId: string;
  drafts: DownloadTaskDraft[];
}

export interface ControlDownloadTaskRequest {
  taskId: string;
  action: TaskAction;
}

export interface OpenDownloadTaskRequest {
  taskId: string;
  target: OpenTarget;
}

export interface TaskListSnapshot {
  sequence: number;
  capacity: 100;
  tasks: DownloadTask[];
}

export interface CreateTasksResult extends TaskListSnapshot {
  reused: boolean;
}

export interface BatchTaskFailure {
  taskId: string;
  error: AppError;
}

export interface BatchTaskResult {
  sequence: number;
  affectedTaskIds: string[];
  failures: BatchTaskFailure[];
}

export interface ClearTasksResult {
  sequence: number;
  removedTaskIds: string[];
}

export interface TaskProgressEvent {
  sequence: number;
  task: DownloadTask;
}

export interface TaskRemovedEvent {
  sequence: number;
  taskId: string;
}
