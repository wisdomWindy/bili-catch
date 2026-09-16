import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { useDownloadCenterStore } from "../features/download-center/store";
import type { SettingsValues } from "../features/settings/contracts";
import { useAppStore } from "../stores/app";
import { createSettingsEffectSink } from "./settings-effects";

const values: SettingsValues = {
  downloadDirectory: "D:/Media",
  temporaryDirectory: "D:/Temp",
  maxConcurrentTasks: 3,
  connectionsPerTask: 8,
  defaultVideoQuality: "64",
  defaultAudioFormat: "flac",
  theme: "dark",
  locale: "en-US",
  notifyOnComplete: true,
  completionSound: true,
  closeBehavior: "minimizeToTray",
  autoCheckUpdates: true,
};

describe("settings effect sink", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("separates optimistic appearance from confirmed download defaults", () => {
    const app = useAppStore();
    const download = useDownloadCenterStore();
    const sink = createSettingsEffectSink(app, download);

    sink.applyAppearance({ theme: "dark", locale: "en-US" });
    expect(app.themePreference).toBe("dark");
    expect(app.locale).toBe("en-US");
    expect(download.outputDir).toBe("Downloads");

    sink.applyCommitted(values);
    expect(download.outputDir).toBe("D:/Media");
    expect(download.defaults).toMatchObject({
      defaultVideoQuality: "64",
      defaultAudioFormat: "flac",
    });
  });
});
