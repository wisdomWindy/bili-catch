import type { AppLocale, ThemePreference } from "../../contracts/app";
import type { AudioFormat, VideoQualityId } from "../../contracts/media";
import type { AppError } from "../../contracts/app-error";

export type CloseBehavior = "minimizeToTray" | "exit";

export interface SettingsValues {
  downloadDirectory: string;
  temporaryDirectory: string;
  maxConcurrentTasks: number;
  connectionsPerTask: number;
  defaultVideoQuality: VideoQualityId;
  defaultAudioFormat: AudioFormat;
  theme: ThemePreference;
  locale: AppLocale;
  notifyOnComplete: boolean;
  completionSound: boolean;
  closeBehavior: CloseBehavior;
  autoCheckUpdates: boolean;
}

export interface SettingsSnapshot {
  schemaVersion: 1;
  revision: number;
  values: SettingsValues;
}

export const SETTING_KEYS = [
  "downloadDirectory",
  "temporaryDirectory",
  "maxConcurrentTasks",
  "connectionsPerTask",
  "defaultVideoQuality",
  "defaultAudioFormat",
  "theme",
  "locale",
  "notifyOnComplete",
  "completionSound",
  "closeBehavior",
  "autoCheckUpdates",
] as const satisfies readonly (keyof SettingsValues)[];

export type SettingKey = (typeof SETTING_KEYS)[number];

export type SettingsPatch = {
  [Key in SettingKey]: { field: Key; value: SettingsValues[Key] };
}[SettingKey];

export type FieldSaveStatus = "idle" | "preview" | "saving" | "saved" | "error";

export interface FieldSaveState {
  status: FieldSaveStatus;
  error: AppError | null;
}

export type SettingsLoadStatus = "idle" | "loading" | "ready" | "load_error";

export type AppearanceSettings = Pick<SettingsValues, "theme" | "locale">;
