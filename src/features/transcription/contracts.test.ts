import { describe, expect, it } from "vitest";
import {
  ASR_MODEL_IDS,
  TRANSCRIBE_TASK_STATUSES,
  TRANSCRIPT_FORMATS,
  type CreateTranscribeTaskRequest,
  type TranscriptionSource,
} from "./contracts";

describe("transcription contracts", () => {
  it("exposes the approved model, format, and task status domains", () => {
    expect(ASR_MODEL_IDS).toEqual([
      "tiny",
      "base",
      "small",
      "medium",
      "large-v3",
      "fire-red-asr2",
    ]);
    expect(TRANSCRIPT_FORMATS).toEqual(["txt", "srt", "vtt", "json", "markdown"]);
    expect(TRANSCRIBE_TASK_STATUSES).toEqual([
      "pending",
      "preprocessing",
      "loadingModel",
      "transcribing",
      "processing",
      "saving",
      "completed",
      "failed",
      "cancelled",
    ]);
  });

  it("preserves the Rust tagged source and request field names", () => {
    const source: TranscriptionSource = {
      kind: "downloadTask",
      taskId: "download-7",
      outputPath: "D:/Media/video.mp4",
      bvid: "BV1example",
      cid: 42,
    };
    const request: CreateTranscribeTaskRequest = {
      source,
      options: {
        modelId: "base",
        language: "auto",
        outputFormat: "srt",
        timestamps: true,
        punctuation: true,
        outputDirectory: "D:/Media/Transcribe",
        gpuMode: "auto",
      },
      force: false,
    };

    expect(request).toEqual({
      source: {
        kind: "downloadTask",
        taskId: "download-7",
        outputPath: "D:/Media/video.mp4",
        bvid: "BV1example",
        cid: 42,
      },
      options: {
        modelId: "base",
        language: "auto",
        outputFormat: "srt",
        timestamps: true,
        punctuation: true,
        outputDirectory: "D:/Media/Transcribe",
        gpuMode: "auto",
      },
      force: false,
    });
  });
});
