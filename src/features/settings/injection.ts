import { inject, type InjectionKey } from "vue";
import type { DirectoryPickerPort } from "./dialog";
import type { SettingsEffectSink } from "./effects";
import type { ReleaseActionsPort } from "./release";
import type { SettingsService } from "./service";

export const settingsServiceKey: InjectionKey<SettingsService> = Symbol("settings-service");
export const directoryPickerKey: InjectionKey<DirectoryPickerPort> = Symbol("directory-picker");
export const releaseActionsKey: InjectionKey<ReleaseActionsPort> = Symbol("release-actions");
export const settingsEffectSinkKey: InjectionKey<SettingsEffectSink> = Symbol("settings-effects");

const unavailableSettings: SettingsService = {
  getSnapshot: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Settings service is unavailable",
      details: "SETTINGS_SERVICE_UNAVAILABLE",
    };
  },
  update: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Settings service is unavailable",
      details: "SETTINGS_SERVICE_UNAVAILABLE",
    };
  },
};

const unavailablePicker: DirectoryPickerPort = {
  selectDirectory: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Directory picker is unavailable",
      details: "DIRECTORY_PICKER_FAILED",
    };
  },
};

const unavailableRelease: ReleaseActionsPort = {
  checkForUpdates: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Release information is not configured",
      details: "UPDATE_NOT_CONFIGURED",
    };
  },
  installUpdate: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Release information is not configured",
      details: "UPDATE_NOT_CONFIGURED",
    };
  },
  openLicenses: async () => {
    throw {
      code: "E_INTERNAL",
      message: "Release information is not configured",
      details: "LICENSES_NOT_CONFIGURED",
    };
  },
};

const unavailableEffects: SettingsEffectSink = {
  applyAppearance: () => undefined,
  applyCommitted: () => undefined,
};

export function useSettingsService(): SettingsService {
  return inject(settingsServiceKey, unavailableSettings);
}

export function useDirectoryPicker(): DirectoryPickerPort {
  return inject(directoryPickerKey, unavailablePicker);
}

export function useReleaseActions(): ReleaseActionsPort {
  return inject(releaseActionsKey, unavailableRelease);
}

export function useSettingsEffectSink(): SettingsEffectSink {
  return inject(settingsEffectSinkKey, unavailableEffects);
}
