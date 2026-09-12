import type { AudioFormat } from "./contracts";

export type AudioProfileId = "128" | "192" | "320" | "source" | "lossless";

export interface AudioProfileOption {
  id: AudioProfileId;
  label: string;
}

const PROFILES: Readonly<Record<AudioFormat, readonly AudioProfileOption[]>> = {
  mp3: [
    { id: "128", label: "128K" },
    { id: "192", label: "192K" },
    { id: "320", label: "320K" },
  ],
  m4a: [{ id: "source", label: "Original" }],
  flac: [{ id: "lossless", label: "Lossless" }],
};

export function audioProfileOptions(format: AudioFormat): readonly AudioProfileOption[] {
  return PROFILES[format];
}

export function defaultAudioProfile(format: AudioFormat): AudioProfileId {
  return format === "mp3" ? "320" : format === "m4a" ? "source" : "lossless";
}

export function isAudioProfileValid(
  format: AudioFormat | null,
  profileId: string | null,
): profileId is AudioProfileId {
  return format !== null && profileId !== null && PROFILES[format].some(({ id }) => id === profileId);
}
