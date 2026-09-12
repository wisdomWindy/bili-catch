export const AUDIO_FORMATS = ["mp3", "m4a", "flac"] as const;
export type AudioFormat = (typeof AUDIO_FORMATS)[number];

export const VIDEO_QUALITY_IDS = ["16", "32", "64", "80", "112", "120", "125", "127"] as const;
export type VideoQualityId = (typeof VIDEO_QUALITY_IDS)[number];
