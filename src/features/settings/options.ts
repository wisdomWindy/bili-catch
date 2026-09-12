import { AUDIO_FORMATS, VIDEO_QUALITY_IDS, type AudioFormat, type VideoQualityId } from "../../contracts/media";
import type { AppLocale, ThemePreference } from "../../contracts/app";
import type { CloseBehavior } from "./contracts";

export interface SettingOption<Value extends string> {
  value: Value;
  labelKey: string;
}

const qualityLabelKeys = ["p360", "p480", "p720", "p1080", "p1080Plus", "p4k", "hdr", "p8k"] as const;

export const VIDEO_QUALITY_OPTIONS: readonly SettingOption<VideoQualityId>[] = VIDEO_QUALITY_IDS.map((value, index) => ({
  value,
  labelKey: `settings.options.quality.${qualityLabelKeys[index]}`,
}));

export const AUDIO_FORMAT_OPTIONS: readonly SettingOption<AudioFormat>[] = AUDIO_FORMATS.map((value) => ({
  value,
  labelKey: `settings.options.audio.${value}`,
}));

export const THEME_OPTIONS: readonly SettingOption<ThemePreference>[] = [
  { value: "light", labelKey: "theme.light" },
  { value: "dark", labelKey: "theme.dark" },
  { value: "system", labelKey: "theme.system" },
];

export const LOCALE_OPTIONS: readonly SettingOption<AppLocale>[] = [
  { value: "zh-CN", labelKey: "locale.zhCN" },
  { value: "en-US", labelKey: "locale.enUS" },
];

export const CLOSE_BEHAVIOR_OPTIONS: readonly SettingOption<CloseBehavior>[] = [
  { value: "minimizeToTray", labelKey: "settings.options.close.minimizeToTray" },
  { value: "exit", labelKey: "settings.options.close.exit" },
];
