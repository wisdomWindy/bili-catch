import type { AudioFormat, VideoQualityId } from "../../contracts/media";
import type { AsrModelId, TranscriptFormat, TranscriptionLanguage } from "../transcription/contracts";

export type { AudioFormat } from "../../contracts/media";

export type DownloadMode = "video-audio" | "video-only" | "audio-only";
export type VideoCodec = "avc" | "hevc" | "av1";
export type VideoSource = "ugc" | "bangumi";
export type ContainerKind = "video" | "season";
export type Availability = "playable" | "unavailable";
export type UnavailableReason =
  | "login-required"
  | "vip-required"
  | "paid"
  | "region-locked"
  | "download-disabled"
  | "preview"
  | "unknown";

export interface VideoVariant {
  qualityId: string;
  codec: VideoCodec;
}

export interface ParseContainer {
  kind: ContainerKind;
  id: string;
  title: string;
  unitCount: number;
}

export interface VideoPart {
  cid: number;
  /** bvid of the archive this unit belongs to; differs per collection member. */
  bvid: string;
  /** Bangumi episode id; `null` for UGC units. */
  epId: number | null;
  /** 1-based part index inside its container. */
  page: number;
  /** Number of downloadable parts inside the container. */
  partCount: number;
  /** 1-based container order inside the parse result. */
  groupIndex: number;
  title: string;
  durationSeconds: number;
  availability: Availability;
  unavailableReason: UnavailableReason | null;
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
  bvid: string | null;
  aid: number | null;
  source: VideoSource;
  container: ParseContainer | null;
  title: string;
  ownerName: string;
  coverUrl: string;
  durationSeconds: number;
  requestedPage: number | null;
  /** Unit selected first; `null` falls back to `requestedPage`. */
  requestedCid: number | null;
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
  autoTranscribeDownloads: boolean;
  defaultAsrModel: AsrModelId;
  defaultTranscriptionLanguage: TranscriptionLanguage;
  defaultTranscriptFormat: TranscriptFormat;
  transcriptionTimestamps: boolean;
  transcriptionPunctuation: boolean;
}

export interface TranscriptionPreset {
  modelId: AsrModelId;
  language: TranscriptionLanguage;
  format: TranscriptFormat;
  timestamps: boolean;
  punctuation: boolean;
}

export interface DownloadTaskDraft {
  canonicalUrl: string;
  bvid: string;
  cid: number;
  page: number;
  partTitle: string;
  videoTitle: string;
  partCount: number;
  source: VideoSource;
  containerKind: ContainerKind;
  epId: number | null;
  groupIndex: number;
  mode: DownloadMode;
  outputDir: string;
  qualityId: string | null;
  codec: VideoCodec | null;
  audioFormat: AudioFormat | null;
  audioBitrateId: string | null;
  autoTranscribe: boolean;
  transcriptionPreset: TranscriptionPreset | null;
}
