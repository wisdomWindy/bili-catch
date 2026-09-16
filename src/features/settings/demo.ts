import type { AppError } from "../../contracts/app-error";
import type { SettingsPatch, SettingsSnapshot } from "./contracts";
import type { DirectoryPickerPort } from "./dialog";
import type { ReleaseActionsPort } from "./release";
import type { SettingsService } from "./service";

export interface DemoSettingsRuntime {
  service: SettingsService;
  directoryPicker: DirectoryPickerPort;
  releaseActions: ReleaseActionsPort;
}

const initialSnapshot: SettingsSnapshot = {
  schemaVersion: 1,
  revision: 0,
  values: {
    downloadDirectory: "D:/BiliCatch Demo",
    temporaryDirectory: "D:/BiliCatch Demo/Temp",
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
  },
};

function demoError(message: string, details: string): AppError {
  return { code: "E_INTERNAL", message, details };
}

function validateDemoPatch(patch: SettingsPatch) {
  if (
    (patch.field === "maxConcurrentTasks" && !(patch.value >= 1 && patch.value <= 10))
    || (patch.field === "connectionsPerTask" && !(patch.value >= 1 && patch.value <= 32))
  ) {
    throw demoError("The setting value is invalid", "SETTINGS_INVALID_VALUE");
  }
  if (
    (patch.field === "downloadDirectory" || patch.field === "temporaryDirectory")
    && !/^(?:[A-Za-z]:[\\/]|\/)/.test(patch.value)
  ) {
    throw demoError("The selected directory is invalid", "SETTINGS_PATH_INVALID");
  }
}

export function createDemoSettingsRuntime(hash: string): DemoSettingsRuntime {
  let document: SettingsSnapshot = structuredClone(initialSnapshot);
  if (hash.includes("locale=en-US")) document.values.locale = "en-US";
  if (hash.includes("theme=dark")) document.values.theme = "dark";
  if (hash.includes("theme=light")) document.values.theme = "light";
  const loadStalls = hash.includes("settings=loading");
  const loadFails = hash.includes("settings=load-error");
  const saveFails = hash.includes("settings=save-error");

  return {
    service: {
      async getSnapshot() {
        if (loadStalls) {
          return new Promise<SettingsSnapshot>(() => undefined);
        }
        if (loadFails) {
          throw demoError("Settings storage is unavailable", "SETTINGS_STORE_UNAVAILABLE");
        }
        return structuredClone(document);
      },
      async update(patch) {
        if (saveFails) {
          throw demoError("Settings storage is unavailable", "SETTINGS_STORE_UNAVAILABLE");
        }
        validateDemoPatch(patch);
        document = {
          ...document,
          revision: document.revision + 1,
          values: { ...document.values, [patch.field]: patch.value },
        };
        return structuredClone(document);
      },
    },
    directoryPicker: {
      async selectDirectory() {
        return hash.includes("picker=cancel") ? null : "D:/BiliCatch Demo";
      },
    },
    releaseActions: {
      async checkForUpdates() {
        if (hash.includes("update=error")) {
          throw demoError("Unable to check for updates", "UPDATE_CHECK_FAILED");
        }
        if (hash.includes("update=available")) {
          return {
            status: "available",
            currentVersion: "0.1.0",
            latestVersion: "0.2.0",
          };
        }
        return { status: "latest", currentVersion: "0.1.0" };
      },
      async installUpdate() {
        if (!hash.includes("update=available")) {
          throw demoError("No update is available", "UPDATE_NOT_AVAILABLE");
        }
      },
      async openLicenses() {
        return undefined;
      },
    },
  };
}
