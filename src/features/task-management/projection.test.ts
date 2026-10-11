import { describe, expect, it } from "vitest";
import type { DownloadTask } from "./contracts";
import type { TranscribeTask } from "../transcription/contracts";
import { matchesCombinedFilter, projectDownloadTask, projectTranscriptionTask } from "./projection";

const download = { id: "d1", fileName: "video.mp4", status: "completed", progressPercent: 100, speedBytesPerSecond: "0", bytesDownloaded: "10", updatedAt: "2026-10-10T00:00:00Z" } as DownloadTask;
const transcription = { id: "t1", sourceInfo: { fileName: "audio.mp3" }, status: "transcribing", progress: { progress: 0.35, processedSeconds: 12, speed: 1.5 }, updatedAt: "2026-10-10T00:01:00Z" } as TranscribeTask;

describe("combined task projection", () => {
  it("keeps task domains explicitly discriminated", () => {
    const downloadItem = projectDownloadTask(download);
    const transcriptionItem = projectTranscriptionTask(transcription);

    expect(downloadItem).toMatchObject({ id: "download:d1", kind: "download", statusGroup: "completed" });
    expect(transcriptionItem).toMatchObject({ id: "transcription:t1", kind: "transcription", statusGroup: "active", progress: 35, primaryMetric: "1.5x" });
    expect(matchesCombinedFilter(downloadItem, "download")).toBe(true);
    expect(matchesCombinedFilter(downloadItem, "transcription")).toBe(false);
    expect(matchesCombinedFilter(transcriptionItem, "active")).toBe(true);
  });
});
