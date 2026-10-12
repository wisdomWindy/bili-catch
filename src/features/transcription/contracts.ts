import type { AppError } from "../../contracts/app-error";

export const ASR_MODEL_IDS = [
  "tiny",
  "base",
  "small",
  "medium",
  "large-v3",
  "fire-red-asr2",
] as const;
export type AsrModelId = (typeof ASR_MODEL_IDS)[number];

export const TRANSCRIPTION_LANGUAGES = ["auto", "zh", "zh-TW", "en", "ja", "ko"] as const;
export type TranscriptionLanguage = (typeof TRANSCRIPTION_LANGUAGES)[number];

export const TRANSCRIPT_FORMATS = ["txt", "srt", "vtt", "json", "markdown"] as const;
export type TranscriptFormat = (typeof TRANSCRIPT_FORMATS)[number];

export type GpuMode = "auto" | "enabled" | "disabled";
export const TRANSCRIBE_TASK_STATUSES = [
  "pending",
  "preprocessing",
  "loadingModel",
  "transcribing",
  "processing",
  "saving",
  "completed",
  "failed",
  "cancelled",
] as const;
export type TranscribeTaskStatus = (typeof TRANSCRIBE_TASK_STATUSES)[number];
export type TranscribeTaskAction = "cancel" | "retry" | "delete";
export type MediaKind = "audio" | "video";

export type TranscriptionSource =
  | { kind: "localFile"; path: string }
  | {
      kind: "downloadTask";
      taskId: string;
      outputPath: string;
      bvid?: string;
      cid?: number;
    };

export interface MediaSourceInfo {
  fileName: string;
  path: string;
  durationSeconds: number;
  sizeBytes: number;
  mediaKind: MediaKind;
  format: string;
}

export interface TranscriptionOptions {
  modelId: AsrModelId;
  language: TranscriptionLanguage;
  outputFormat: TranscriptFormat;
  timestamps: boolean;
  punctuation: boolean;
  outputDirectory: string;
  gpuMode: GpuMode;
}

export interface CreateTranscribeTaskRequest {
  source: TranscriptionSource;
  options: TranscriptionOptions;
  force: boolean;
}

export interface TranscriptionProgress {
  progress: number;
  processedSeconds: number;
  totalSeconds?: number;
  speed?: number;
  etaSeconds?: number;
  message: string;
}

export interface TranscribeTask {
  id: string;
  revision: number;
  source: TranscriptionSource;
  sourceInfo: MediaSourceInfo;
  options: TranscriptionOptions;
  status: TranscribeTaskStatus;
  progress: TranscriptionProgress;
  resultId?: string;
  outputPath?: string;
  error?: AppError;
  cacheReused: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface TranscribeTaskListSnapshot {
  sequence: number;
  tasks: TranscribeTask[];
}

export interface TranscriptSegment {
  index: number;
  startMs: number;
  endMs: number;
  text: string;
}

export interface TranscriptResult {
  id: string;
  taskId: string;
  revision: number;
  sourceFingerprint: string;
  modelId: AsrModelId;
  requestedLanguage: TranscriptionLanguage;
  detectedLanguage: string;
  segments: TranscriptSegment[];
  plainText: string;
  createdAt: string;
  updatedAt: string;
}

export type ModelDownloadState =
  | "idle"
  | "queued"
  | "downloading"
  | "verifying"
  | "completed"
  | "failed"
  | "cancelled";

export interface AsrModel {
  id: AsrModelId;
  displayName: string;
  fileName: string;
  sizeBytes: number;
  installed: boolean;
  valid: boolean;
  isDefault: boolean;
  inUse: boolean;
  path?: string;
  recommendedFor: string;
  capabilities: string[];
  downloadState: ModelDownloadState;
  downloadedBytes: number;
  error?: AppError;
  revision: number;
}

export interface AsrModelListSnapshot {
  sequence: number;
  models: AsrModel[];
}

export interface TranscriptionProgressEvent {
  sequence: number;
  task: TranscribeTask;
}

export interface TranscriptionRemovedEvent {
  sequence: number;
  taskId: string;
}

export interface ModelDownloadProgressEvent {
  sequence: number;
  model: AsrModel;
}

export interface CreateTranscribeTaskResult {
  task: TranscribeTask;
  reused: boolean;
}

export interface ControlTranscribeTaskRequest {
  taskId: string;
  action: TranscribeTaskAction;
}

export interface SaveTranscriptEditsRequest {
  taskId: string;
  expectedRevision: number;
  segments: string[];
}

export interface ExportTranscriptRequest {
  taskId: string;
  resultRevision: number;
  format: TranscriptFormat;
  timestamps: boolean;
  outputDirectory: string;
}

export interface ExportTranscriptResult {
  outputPath: string;
}
