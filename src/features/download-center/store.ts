import { defineStore } from "pinia";
import type { AppError } from "../../contracts/app-error";
import { normalizeIpcError } from "../../contracts/app-error";
import { normalizeParseInput } from "./input";
import { defaultAudioProfile, isAudioProfileValid } from "./audio-options";
import type { AudioProfileId } from "./audio-options";
import type {
  AudioFormat,
  DownloadDefaults,
  DownloadMode,
  DownloadTaskDraft,
  MediaOption,
  ParseVideoResult,
  VideoCodec,
} from "./contracts";
import type { ParseVideoService } from "./service";
import { isVideoVariantValid, selectVideoVariant } from "./video-options";

export type ParseStatus = "idle" | "parsing" | "success" | "failed" | "enqueueing";

interface DownloadCenterState {
  input: string;
  normalizedInput: string | null;
  status: ParseStatus;
  result: ParseVideoResult | null;
  error: AppError | null;
  selectedCids: number[];
  mode: DownloadMode;
  qualityId: string | null;
  codec: VideoCodec | null;
  audioFormat: AudioFormat | null;
  audioBitrateId: string | null;
  authenticated: boolean;
  lastMp3Profile: AudioProfileId;
  audioFallbackNotice: boolean;
  videoFallbackNotice: boolean;
  outputDir: string;
  defaults: DownloadDefaults;
  requestToken: number;
}

function optionIsAvailable(options: MediaOption[], id: string | null): boolean {
  return id !== null && options.some((option) => option.id === id && !option.requiresLogin);
}

function audioFormatAvailable(
  result: ParseVideoResult,
  format: AudioFormat,
  authenticated: boolean,
): boolean {
  if (format === "flac") {
    return authenticated &&
      result.audioFormats.includes("flac") &&
      (result.audioCapability.losslessAvailable || result.audioCapability.hiResAvailable);
  }
  return result.audioFormats.includes(format);
}

function fallbackAudioFormat(result: ParseVideoResult, authenticated: boolean): AudioFormat | null {
  return (["m4a", "mp3", "flac"] as const)
    .find((format) => audioFormatAvailable(result, format, authenticated)) ?? null;
}

export const useDownloadCenterStore = defineStore("download-center", {
  state: (): DownloadCenterState => ({
    input: "",
    normalizedInput: null,
    status: "idle",
    result: null,
    error: null,
    selectedCids: [],
    mode: "video-audio",
    qualityId: null,
    codec: null,
    audioFormat: null,
    audioBitrateId: null,
    authenticated: false,
    lastMp3Profile: "320",
    audioFallbackNotice: false,
    videoFallbackNotice: false,
    outputDir: "Downloads",
    defaults: {
      downloadDirectory: "Downloads",
      defaultVideoQuality: "80",
      defaultAudioFormat: "m4a",
    },
    requestToken: 0,
  }),
  getters: {
    canParse(state): boolean {
      return normalizeParseInput(state.input).ok && state.status !== "parsing" && state.status !== "enqueueing";
    },
    selectedCount(state): number {
      return state.selectedCids.length;
    },
    canEnqueue(state): boolean {
      if (!state.result || state.selectedCids.length === 0 || !state.outputDir.trim()) return false;
      const videoReady =
        optionIsAvailable(state.result.qualities, state.qualityId) &&
        isVideoVariantValid(state.result.videoVariants, state.qualityId, state.codec);
      const audioReady =
        state.audioFormat !== null &&
        audioFormatAvailable(state.result, state.audioFormat, state.authenticated) &&
        isAudioProfileValid(state.audioFormat, state.audioBitrateId);
      if (state.mode !== "audio-only") return videoReady;
      return audioReady;
    },
  },
  actions: {
    setInput(value: string) {
      this.input = value;
      if (this.status === "failed") {
        this.status = "idle";
        this.error = null;
      }
    },
    configureDefaults(defaults: DownloadDefaults) {
      const outputFollowsDefault = this.outputDir === this.defaults.downloadDirectory;
      this.defaults = { ...defaults };
      if (outputFollowsDefault) this.outputDir = defaults.downloadDirectory;
    },
    applyResult(result: ParseVideoResult) {
      this.result = result;
      const preferred = result.parts.find((part) => part.page === result.requestedPage) ?? result.parts[0];
      this.selectedCids = preferred ? [preferred.cid] : [];
      this.mode = "video-audio";
      const videoVariant = selectVideoVariant(
        result.qualities,
        result.videoVariants,
        this.defaults.defaultVideoQuality,
        null,
      );
      this.qualityId = videoVariant?.qualityId ?? null;
      this.codec = videoVariant?.codec ?? null;
      this.audioFormat = null;
      this.audioBitrateId = null;
      this.lastMp3Profile = "320";
      this.audioFallbackNotice = false;
      this.videoFallbackNotice = false;
      this.status = "success";
      this.error = null;
    },
    async parse(service: ParseVideoService, input?: string) {
      const submittedInput = input ?? this.input;
      this.input = submittedInput;
      const normalized = normalizeParseInput(submittedInput);
      if (!normalized.ok) {
        if (normalized.reason === "invalid") {
          this.status = "failed";
          this.error = { code: "E003", message: "Invalid Bilibili video input" };
        }
        return;
      }
      const token = ++this.requestToken;
      this.normalizedInput = normalized.value;
      this.status = "parsing";
      this.error = null;
      try {
        const result = await service.parseVideo(normalized.value);
        if (token !== this.requestToken) return;
        this.applyResult(result);
      } catch (error: unknown) {
        if (token !== this.requestToken) return;
        this.status = "failed";
        this.result = null;
        this.error = normalizeIpcError(error);
      }
    },
    retry(service: ParseVideoService) {
      if (this.normalizedInput) return this.parse(service, this.normalizedInput);
      return Promise.resolve();
    },
    togglePart(cid: number) {
      this.selectedCids = this.selectedCids.includes(cid)
        ? this.selectedCids.filter((value) => value !== cid)
        : [...this.selectedCids, cid];
    },
    selectAll() {
      this.selectedCids = this.result?.parts.map((part) => part.cid) ?? [];
    },
    invertSelection() {
      this.selectedCids = this.result?.parts
        .filter((part) => !this.selectedCids.includes(part.cid))
        .map((part) => part.cid) ?? [];
    },
    setMode(mode: DownloadMode, authenticated?: boolean) {
      const isAuthenticated = authenticated ?? this.authenticated;
      this.mode = mode;
      this.authenticated = isAuthenticated;
      if (mode !== "audio-only") {
        this.audioFormat = null;
        this.audioBitrateId = null;
        if (this.result) {
          const videoVariant = selectVideoVariant(
            this.result.qualities,
            this.result.videoVariants,
            this.qualityId ?? this.defaults.defaultVideoQuality,
            this.codec,
          );
          this.qualityId = videoVariant?.qualityId ?? null;
          this.codec = videoVariant?.codec ?? null;
        }
      } else {
        this.qualityId = null;
        this.codec = null;
        if (this.result) {
          const preferred = this.audioFormat ?? this.defaults.defaultAudioFormat;
          this.setAudioFormat(preferred, isAuthenticated);
        }
      }
    },
    setVideoQuality(qualityId: string | null) {
      if (!this.result || qualityId === null) {
        this.qualityId = null;
        this.codec = null;
        return;
      }
      const quality = this.result.qualities.find((item) => item.id === qualityId);
      const videoVariant = quality && !quality.requiresLogin
        ? selectVideoVariant([quality], this.result.videoVariants, qualityId, this.codec)
        : null;
      this.qualityId = videoVariant?.qualityId ?? null;
      this.codec = videoVariant?.codec ?? null;
    },
    setVideoCodec(codec: VideoCodec | null) {
      this.codec = isVideoVariantValid(this.result?.videoVariants ?? [], this.qualityId, codec)
        ? codec
        : null;
    },
    setAudioFormat(format: AudioFormat | null, authenticated?: boolean) {
      const isAuthenticated = authenticated ?? this.authenticated;
      this.authenticated = isAuthenticated;
      if (!this.result || format === null) {
        this.audioFormat = null;
        this.audioBitrateId = null;
        return;
      }
      if (!audioFormatAvailable(this.result, format, isAuthenticated)) {
        const fallback = fallbackAudioFormat(this.result, isAuthenticated);
        if (format === "flac" && fallback !== "flac") this.audioFallbackNotice = true;
        this.audioFormat = fallback;
      } else {
        this.audioFormat = format;
      }
      if (this.audioFormat === "mp3") this.audioBitrateId = this.lastMp3Profile;
      else if (this.audioFormat) this.audioBitrateId = defaultAudioProfile(this.audioFormat);
      else this.audioBitrateId = null;
    },
    setAudioProfile(profileId: string | null) {
      if (this.audioFormat !== "mp3" || !isAudioProfileValid("mp3", profileId)) return;
      this.lastMp3Profile = profileId;
      this.audioBitrateId = profileId;
    },
    reconcileAudioAvailability(authenticated: boolean) {
      this.authenticated = authenticated;
      if (!this.result || this.mode !== "audio-only" || !this.audioFormat) return;
      if (!audioFormatAvailable(this.result, this.audioFormat, authenticated)) {
        this.setAudioFormat(this.audioFormat, authenticated);
      }
    },
    async refreshForAuthChange(service: ParseVideoService, authenticated: boolean) {
      const input = this.normalizedInput;
      const previousMode = this.mode;
      const previousQuality = this.qualityId;
      const previousCodec = this.codec;
      const previousAudioFormat = this.audioFormat;
      const previousAudioBitrate = this.audioBitrateId;
      this.authenticated = authenticated;
      this.videoFallbackNotice = false;
      if (!input) {
        this.reconcileAudioAvailability(authenticated);
        return;
      }
      await this.parse(service, input);
      if (!this.result || this.status !== "success") return;
      this.setMode(previousMode, authenticated);
      if (previousMode !== "audio-only") {
        if (isVideoVariantValid(this.result.videoVariants, previousQuality, previousCodec)) {
          this.qualityId = previousQuality;
          this.codec = previousCodec;
        } else if (previousQuality !== null || previousCodec !== null) {
          this.videoFallbackNotice = true;
        }
      } else if (previousAudioFormat) {
        this.setAudioFormat(previousAudioFormat, authenticated);
        if (this.audioFormat === previousAudioFormat && previousAudioBitrate) {
          this.setAudioProfile(previousAudioBitrate);
        }
      }
    },
    consumeAudioFallbackNotice(): boolean {
      const visible = this.audioFallbackNotice;
      this.audioFallbackNotice = false;
      return visible;
    },
    consumeVideoFallbackNotice(): boolean {
      const visible = this.videoFallbackNotice;
      this.videoFallbackNotice = false;
      return visible;
    },
    buildDrafts(): DownloadTaskDraft[] {
      if (!this.canEnqueue || !this.result) return [];
      const result = this.result;
      return result.parts
        .filter((part) => this.selectedCids.includes(part.cid))
        .map((part) => ({
          canonicalUrl: result.canonicalUrl,
          bvid: result.bvid,
          cid: part.cid,
          page: part.page,
          partTitle: part.title,
          videoTitle: result.title,
          partCount: result.parts.length,
          mode: this.mode,
          outputDir: this.outputDir.trim(),
          qualityId: this.mode === "audio-only" ? null : this.qualityId,
          codec: this.mode === "audio-only" ? null : this.codec,
          audioFormat: this.mode === "audio-only" ? this.audioFormat : null,
          audioBitrateId: this.mode === "audio-only" ? this.audioBitrateId : null,
        }));
    },
    clear() {
      this.requestToken += 1;
      this.input = "";
      this.normalizedInput = null;
      this.status = "idle";
      this.result = null;
      this.error = null;
      this.selectedCids = [];
      this.mode = "video-audio";
      this.qualityId = null;
      this.codec = null;
      this.audioFormat = null;
      this.audioBitrateId = null;
      this.lastMp3Profile = "320";
      this.audioFallbackNotice = false;
      this.videoFallbackNotice = false;
    },
  },
});
