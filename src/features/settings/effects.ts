import type { AppearanceSettings, SettingsValues } from "./contracts";

export interface SettingsEffectSink {
  applyAppearance(appearance: AppearanceSettings): void;
  applyCommitted(values: SettingsValues): void;
}
