import type { AudioFormat, VideoQualityId } from "../../contracts/media";

export type { AudioFormat } from "../../contracts/media";

export type DownloadMode = "video-audio" | "video-only" | "audio-only";
export type VideoCodec = "avc" | "hevc" | "av1";

export interface VideoVariant {
  qualityId: string;
  codec: VideoCodec;
}

export interface VideoPart {
  cid: number;
  page: number;
  title: string;
  durationSeconds: number;
}

export interface MediaOption {
  id: string;
  label: string;
  requiresLogin: boolean;
}

export interface AudioCapability {
  maxLossyKbps: number | null;
  losslessAvailable: boolean;
  hiResAvailable: boolean;
}

export interface ParseVideoResult {
  canonicalUrl: string;
  bvid: string;
  aid: number;
  title: string;
  ownerName: string;
  coverUrl: string;
  durationSeconds: number;
  requestedPage: number | null;
  parts: VideoPart[];
  qualities: MediaOption[];
  codecs: VideoCodec[];
  videoVariants: VideoVariant[];
  audioFormats: AudioFormat[];
  audioBitrates: MediaOption[];
  audioCapability: AudioCapability;
}

export interface DownloadDefaults {
  downloadDirectory: string;
  defaultVideoQuality: VideoQualityId;
  defaultAudioFormat: AudioFormat;
}

export interface DownloadTaskDraft {
  canonicalUrl: string;
  bvid: string;
  cid: number;
  page: number;
  partTitle: string;
  videoTitle: string;
  partCount: number;
  mode: DownloadMode;
  outputDir: string;
  qualityId: string | null;
  codec: VideoCodec | null;
  audioFormat: AudioFormat | null;
  audioBitrateId: string | null;
}
