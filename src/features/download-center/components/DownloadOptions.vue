<script setup lang="ts">
import { computed } from "vue";
import { audioProfileOptions } from "../audio-options";
import { availableCodecsForQuality } from "../video-options";
import type { AudioCapability, AudioFormat, DownloadMode, MediaOption, VideoCodec, VideoVariant } from "../contracts";

const props = defineProps<{
  mode: DownloadMode;
  qualities: MediaOption[];
  codecs: VideoCodec[];
  videoVariants: VideoVariant[];
  audioFormats: AudioFormat[];
  audioCapability: AudioCapability;
  authenticated: boolean;
  qualityId: string | null;
  codec: VideoCodec | null;
  audioFormat: AudioFormat | null;
  audioBitrateId: string | null;
}>();

const formatOrder: readonly AudioFormat[] = ["mp3", "m4a", "flac"];
const profileOptions = computed(() => props.audioFormat ? audioProfileOptions(props.audioFormat) : []);
const profileDisabled = computed(() => props.audioFormat !== "mp3" || profileOptions.value.length === 0);
const availableVideoCodecs = computed(() =>
  availableCodecsForQuality(props.videoVariants, props.qualityId),
);

function formatDisabled(format: AudioFormat): boolean {
  if (format === "flac") {
    return !props.authenticated ||
      !props.audioFormats.includes("flac") ||
      !(props.audioCapability.losslessAvailable || props.audioCapability.hiResAvailable);
  }
  return !props.audioFormats.includes(format);
}

const emit = defineEmits<{
  mode: [value: DownloadMode];
  quality: [value: string | null];
  codec: [value: VideoCodec | null];
  audioFormat: [value: AudioFormat | null];
  audioBitrate: [value: string | null];
}>();

function selected(event: Event): string | null {
  return (event.target as HTMLSelectElement).value || null;
}
</script>

<template>
  <section class="options-section" aria-labelledby="options-title">
    <div class="section-heading"><h2 id="options-title">{{ $t("download.options") }}</h2></div>
    <fieldset class="mode-selector">
      <legend>{{ $t("download.mode") }}</legend>
      <label v-for="value in (['video-audio', 'video-only', 'audio-only'] as DownloadMode[])" :key="value">
        <input type="radio" name="download-mode" :value="value" :checked="mode === value" @change="emit('mode', value)" />
        <span>{{ $t(`download.modes.${value}`) }}</span>
      </label>
    </fieldset>

    <div class="option-grid">
      <label v-if="mode !== 'audio-only'">
        <span>{{ $t("download.quality") }}</span>
        <select :value="qualityId ?? ''" :disabled="qualities.length === 0" @change="emit('quality', selected($event))">
          <option value="" disabled>{{ $t("download.unavailable") }}</option>
          <option v-for="option in qualities" :key="option.id" :value="option.id" :disabled="option.requiresLogin">
            {{ option.label }}{{ option.requiresLogin ? ` · ${$t('download.loginRequired')}` : '' }}
          </option>
        </select>
      </label>
      <label v-if="mode !== 'audio-only'">
        <span>{{ $t("download.codec") }}</span>
        <select data-testid="video-codec" :value="codec ?? ''" :disabled="availableVideoCodecs.length === 0" @change="emit('codec', selected($event) as VideoCodec | null)">
          <option value="" disabled>{{ $t("download.unavailable") }}</option>
          <option v-for="value in availableVideoCodecs" :key="value" :value="value">{{ value.toUpperCase() }}</option>
        </select>
      </label>
      <label v-if="mode === 'audio-only'">
        <span>{{ $t("download.audioFormat") }}</span>
        <select data-testid="audio-format" :value="audioFormat ?? ''" @change="emit('audioFormat', selected($event) as AudioFormat | null)">
          <option value="" disabled>{{ $t("download.unavailable") }}</option>
          <option v-for="value in formatOrder" :key="value" :value="value" :disabled="formatDisabled(value)">
            {{ value.toUpperCase() }}<template v-if="value === 'flac' && formatDisabled(value)"> · {{ $t(authenticated ? 'download.sourceUnsupported' : 'download.loginRequired') }}</template>
          </option>
        </select>
      </label>
      <label v-if="mode === 'audio-only'">
        <span>{{ $t("download.audioProfile") }}</span>
        <select data-testid="audio-profile" :aria-label="$t('download.audioProfile')" :value="audioBitrateId ?? ''" :disabled="profileDisabled" @change="emit('audioBitrate', selected($event))">
          <option v-if="profileOptions.length === 0" value="" disabled>{{ $t("download.unavailable") }}</option>
          <option v-for="option in profileOptions" :key="option.id" :value="option.id">
            {{ option.id === 'source' ? $t('download.originalBitrate') : option.id === 'lossless' ? $t('download.lossless') : option.label }}
          </option>
        </select>
      </label>
    </div>
  </section>
</template>
