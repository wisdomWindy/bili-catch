import { describe, expect, it } from "vitest";
import { AUDIO_FORMATS, VIDEO_QUALITY_IDS } from "../../contracts/media";
import {
  SETTING_KEYS,
  type SettingsPatch,
  type SettingsSnapshot,
} from "./contracts";
import {
  AUDIO_FORMAT_OPTIONS,
  CLOSE_BEHAVIOR_OPTIONS,
  LOCALE_OPTIONS,
  THEME_OPTIONS,
  VIDEO_QUALITY_OPTIONS,
} from "./options";

describe("settings contracts", () => {
  it("exposes every persisted setting key exactly once", () => {
    expect(SETTING_KEYS).toEqual([
      "downloadDirectory",
      "temporaryDirectory",
      "maxConcurrentTasks",
      "connectionsPerTask",
      "defaultVideoQuality",
      "defaultAudioFormat",
      "theme",
      "locale",
      "notifyOnComplete",
      "closeBehavior",
      "autoCheckUpdates",
    ]);
  });

  it("keeps the approved media option values available to settings", () => {
    expect(VIDEO_QUALITY_IDS).toEqual(["16", "32", "64", "80", "112", "120", "125", "127"]);
    expect(AUDIO_FORMATS).toEqual(["mp3", "m4a", "flac"]);
    expect(VIDEO_QUALITY_OPTIONS.map(({ value }) => value)).toEqual(VIDEO_QUALITY_IDS);
    expect(AUDIO_FORMAT_OPTIONS.map(({ value }) => value)).toEqual(AUDIO_FORMATS);
    expect(THEME_OPTIONS.map(({ value }) => value)).toEqual(["light", "dark", "system"]);
    expect(LOCALE_OPTIONS.map(({ value }) => value)).toEqual(["zh-CN", "en-US"]);
    expect(CLOSE_BEHAVIOR_OPTIONS.map(({ value }) => value)).toEqual(["minimizeToTray", "exit"]);
  });

  it("preserves the backend field names in snapshots and typed patches", () => {
    const snapshot: SettingsSnapshot = {
      schemaVersion: 1,
      revision: 4,
      values: {
        downloadDirectory: "D:/Media/BiliCatch",
        temporaryDirectory: "D:/Temp",
        maxConcurrentTasks: 3,
        connectionsPerTask: 8,
        defaultVideoQuality: "80",
        defaultAudioFormat: "mp3",
        theme: "system",
        locale: "zh-CN",
        notifyOnComplete: true,
        closeBehavior: "minimizeToTray",
        autoCheckUpdates: true,
      },
    };
    const patch: SettingsPatch = { field: "theme", value: "dark" };

    expect(snapshot.values.defaultVideoQuality).toBe("80");
    expect(snapshot.values.connectionsPerTask).toBe(8);
    expect(patch).toEqual({ field: "theme", value: "dark" });
  });
});
