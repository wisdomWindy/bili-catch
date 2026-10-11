import { AUDIO_FORMATS, VIDEO_QUALITY_IDS, type AudioFormat, type VideoQualityId } from "../../contracts/media";
import type { AppLocale, ThemePreference } from "../../contracts/app";
import type { CloseBehavior } from "./contracts";
import {
  ASR_MODEL_IDS,
  TRANSCRIPT_FORMATS,
  TRANSCRIPTION_LANGUAGES,
  type AsrModelId,
  type GpuMode,
  type TranscriptFormat,
  type TranscriptionLanguage,
} from "../transcription/contracts";

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

export const ASR_MODEL_OPTIONS: readonly SettingOption<AsrModelId>[] = ASR_MODEL_IDS.map((value) => ({
  value,
  labelKey: `settings.options.asrModel.${value.replace("-", "")}`,
}));

export const TRANSCRIPTION_LANGUAGE_OPTIONS: readonly SettingOption<TranscriptionLanguage>[] = TRANSCRIPTION_LANGUAGES.map((value) => ({
  value,
  labelKey: `settings.options.transcriptionLanguage.${value.replace("-", "")}`,
}));

export const TRANSCRIPT_FORMAT_OPTIONS: readonly SettingOption<TranscriptFormat>[] = TRANSCRIPT_FORMATS.map((value) => ({
  value,
  labelKey: `settings.options.transcriptFormat.${value}`,
}));

export const GPU_MODE_OPTIONS: readonly SettingOption<GpuMode>[] = [
  { value: "auto", labelKey: "settings.options.gpuMode.auto" },
  { value: "enabled", labelKey: "settings.options.gpuMode.enabled" },
  { value: "disabled", labelKey: "settings.options.gpuMode.disabled" },
];
