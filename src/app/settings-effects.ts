import type { useDownloadCenterStore } from "../features/download-center/store";
import type { AppearanceSettings, SettingsValues } from "../features/settings/contracts";
import type { SettingsEffectSink } from "../features/settings/effects";
import type { useAppStore } from "../stores/app";

type AppStore = ReturnType<typeof useAppStore>;
type DownloadCenterStore = ReturnType<typeof useDownloadCenterStore>;

export function createSettingsEffectSink(
  appStore: AppStore,
  downloadStore: DownloadCenterStore,
): SettingsEffectSink {
  return {
    applyAppearance(appearance: AppearanceSettings) {
      appStore.setTheme(appearance.theme);
      appStore.setLocale(appearance.locale);
    },
    applyCommitted(values: SettingsValues) {
      downloadStore.configureDefaults({
        downloadDirectory: values.downloadDirectory,
        defaultVideoQuality: values.defaultVideoQuality,
        defaultAudioFormat: values.defaultAudioFormat,
      });
    },
  };
}
