import { describe, expect, it, vi } from "vitest";
import type { IpcTransport } from "../../contracts/ipc";
import type { SettingsSnapshot } from "./contracts";
import { createSettingsService } from "./service";

const snapshot: SettingsSnapshot = {
  schemaVersion: 2,
  revision: 0,
  values: {
    downloadDirectory: "D:/Downloads/BiliCatch",
    temporaryDirectory: "D:/Temp",
    maxConcurrentTasks: 3,
    connectionsPerTask: 8,
    defaultVideoQuality: "80",
    defaultAudioFormat: "mp3",
    theme: "system",
    locale: "zh-CN",
    notifyOnComplete: true,
    completionSound: true,
    closeBehavior: "minimizeToTray",
    autoCheckUpdates: true,
    defaultAsrModel: "base",
    defaultTranscriptionLanguage: "auto",
    defaultTranscriptFormat: "srt",
    transcriptionTimestamps: true,
    transcriptionPunctuation: true,
    preferPlatformSubtitles: true,
    autoTranscribeDownloads: false,
    maxConcurrentTranscriptions: 1,
    asrModelDirectory: "D:/AppData/BiliCatch/models/whisper",
    asrGpuMode: "auto",
    asrModelSource: "",
  },
};

describe("createSettingsService", () => {
  it("uses the exact settings command contracts", async () => {
    const invoke = vi.fn().mockResolvedValue(snapshot);
    const service = createSettingsService({ invoke } satisfies IpcTransport);

    await expect(service.getSnapshot()).resolves.toEqual(snapshot);
    await expect(service.update({ field: "theme", value: "dark" })).resolves.toEqual(snapshot);

    expect(invoke).toHaveBeenNthCalledWith(1, "get_settings_snapshot");
    expect(invoke).toHaveBeenNthCalledWith(2, "update_setting", {
      request: { patch: { field: "theme", value: "dark" } },
    });
  });

  it("normalizes load and update failures", async () => {
    const service = createSettingsService({
      invoke: vi.fn().mockRejectedValue("offline"),
    });

    await expect(service.getSnapshot()).rejects.toEqual({
      code: "E_INTERNAL",
      message: "offline",
    });
    await expect(service.update({ field: "locale", value: "en-US" })).rejects.toEqual({
      code: "E_INTERNAL",
      message: "offline",
    });
  });
});
