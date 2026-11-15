<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Check, Copy, Download, ExternalLink, RefreshCw, Save } from "@lucide/vue";
import type { TranscriptFormat, TranscriptResult } from "../contracts";

const props = defineProps<{ result: TranscriptResult; saving: boolean }>();
const emit = defineEmits<{ save: [texts: string[]]; copy: []; open: []; export: [format: TranscriptFormat]; retranscribe: [] }>();
const drafts = ref<string[]>([]);
const exportFormat = ref<TranscriptFormat>("srt");
watch(() => props.result, (value) => { drafts.value = value.segments.map((segment) => segment.text); }, { immediate: true });
const dirty = computed(() => drafts.value.some((text, index) => text !== props.result.segments[index]?.text));

function formatTime(milliseconds: number) {
  const date = new Date(milliseconds);
  return date.toISOString().slice(11, 23);
}

function segmentClass(segment: { confidence?: number; alternates?: unknown[] }) {
  if (segment.alternates && segment.alternates.length > 0) return "segment-row is-conflict";
  if (segment.confidence !== undefined && segment.confidence < 0.55) return "segment-row is-low";
  return "segment-row";
}
</script>

<template>
  <section class="transcribe-section transcript-editor">
    <header><Check :size="19" aria-hidden="true" /><h2>{{ $t("transcribe.result.title") }}</h2><span>{{ result.detectedLanguage }}</span><span v-if="result.pipeline?.reviewRecommended" class="pipeline-hint">{{ $t("transcribe.result.reviewRecommended") }}</span></header>
    <div class="segment-list">
      <label v-for="(segment, index) in result.segments" :key="segment.index" :class="segmentClass(segment)" :title="segment.alternates?.length ? $t('transcribe.result.conflict') : (segment.confidence ?? 1) < 0.55 ? $t('transcribe.result.lowConfidence') : undefined">
        <span>{{ formatTime(segment.startMs) }}</span>
        <textarea v-model="drafts[index]" rows="2" />
      </label>
    </div>
    <footer class="result-actions">
      <button class="secondary-button" type="button" @click="emit('copy')"><Copy :size="16" />{{ $t("transcribe.result.copy") }}</button>
      <button class="secondary-button" type="button" @click="emit('open')"><ExternalLink :size="16" />{{ $t("transcribe.result.open") }}</button>
      <button class="secondary-button" type="button" @click="emit('retranscribe')"><RefreshCw :size="16" />{{ $t("tasks.actions.retranscribe") }}</button>
      <label class="result-export-format"><span class="sr-only">{{ $t("transcribe.result.exportFormat") }}</span><select v-model="exportFormat" :aria-label="$t('transcribe.result.exportFormat')"><option value="txt">TXT</option><option value="srt">SRT</option><option value="vtt">VTT</option><option value="json">JSON</option><option value="markdown">Markdown</option></select></label>
      <button class="secondary-button" type="button" @click="emit('export', exportFormat)"><Download :size="16" />{{ $t("transcribe.result.export") }}</button>
      <button class="primary-button" type="button" :disabled="!dirty || saving" @click="emit('save', drafts)">
        <Save :size="16" />{{ saving ? $t("transcribe.result.saving") : $t("transcribe.result.save") }}
      </button>
    </footer>
  </section>
</template>

<style scoped>
.pipeline-hint {
  margin-left: auto;
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--warning-color, #e6a23c) 18%, transparent);
  color: var(--warning-color, #e6a23c);
}

.segment-row.is-low textarea {
  border-left: 3px solid var(--warning-color, #e6a23c);
}

.segment-row.is-conflict textarea {
  border-left: 3px solid var(--danger-color, #f56c6c);
}
</style>
