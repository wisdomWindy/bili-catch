<script setup lang="ts">
import { CheckCircle2, ChevronDown, Download, LoaderCircle, Trash2 } from "@lucide/vue";
import { NModal, NProgress } from "naive-ui";
import { computed, ref } from "vue";
import type { AsrModel, AsrModelId } from "../contracts";

const props = defineProps<{ show: boolean; models: AsrModel[] }>();
defineEmits<{
  close: [];
  download: [id: AsrModelId];
  cancel: [id: AsrModelId];
  delete: [id: AsrModelId];
}>();

type ModelSeries = "whisper" | "fireRedAsr2";

const modelSeriesOrder: readonly ModelSeries[] = ["whisper", "fireRedAsr2"];
const modelSeriesById: Record<AsrModelId, ModelSeries> = {
  tiny: "whisper",
  base: "whisper",
  small: "whisper",
  medium: "whisper",
  "large-v3": "whisper",
  "fire-red-asr2": "fireRedAsr2",
};
const activeDownloadStates = new Set<AsrModel["downloadState"]>(["queued", "downloading", "verifying"]);
const collapsedSeries = ref<ReadonlySet<ModelSeries>>(new Set());
const groupedModels = computed(() => modelSeriesOrder
  .map((id) => ({ id, models: props.models.filter((model) => modelSeriesById[model.id] === id) }))
  .filter((group) => group.models.length > 0));

function isSeriesCollapsed(series: ModelSeries): boolean {
  return collapsedSeries.value.has(series);
}

function toggleSeries(series: ModelSeries): void {
  const next = new Set(collapsedSeries.value);
  if (next.has(series)) {
    next.delete(series);
  } else {
    next.add(series);
  }
  collapsedSeries.value = next;
}
</script>

<template>
  <NModal :show="show" preset="card" class="model-modal" :title="$t('transcribe.models.title')"
    @close="$emit('close')" @mask-click="$emit('close')" @esc="$emit('close')">
    <p class="model-download-hint">{{ $t("transcribe.models.onDemandHint") }}</p>
    <div class="model-list">
      <section v-for="group in groupedModels" :key="group.id" class="model-series"
        :aria-labelledby="`model-series-${group.id}`">
        <h3 :id="`model-series-${group.id}`" class="model-series-title">
          <button v-if="group.models.length > 1" class="model-series-toggle" type="button"
            :aria-expanded="!isSeriesCollapsed(group.id)"
            :aria-controls="`model-series-list-${group.id}`" @click="toggleSeries(group.id)">
            <span>{{ $t(`transcribe.models.series.${group.id}`) }}</span>
            <ChevronDown :size="16" aria-hidden="true" class="model-series-chevron"
              :class="{ 'is-collapsed': isSeriesCollapsed(group.id) }" />
          </button>
          <span v-else>{{ $t(`transcribe.models.series.${group.id}`) }}</span>
        </h3>
        <div v-if="group.models.length === 1 || !isSeriesCollapsed(group.id)"
          :id="`model-series-list-${group.id}`"
          class="model-series-list">
          <article v-for="model in group.models" :key="model.id" class="model-row">
            <div class="model-copy">
              <strong>{{ model.displayName }}<small v-if="model.isDefault" class="model-default">{{ $t("transcribe.models.default") }}</small></strong>
              <span>{{ model.recommendedFor }}</span>
              <small>{{ (model.sizeBytes / 1024 / 1024).toFixed(0) }} MB</small>
              <small v-if="model.error" class="model-error">{{ $t(`errors.${model.error.code}`) }}</small>
              <small v-else-if="model.downloadState !== 'idle'" class="model-state">{{ $t(`transcribe.models.states.${model.downloadState}`) }}</small>
            </div>
            <NProgress v-if="activeDownloadStates.has(model.downloadState)" type="line" :height="5" :show-indicator="false"
              :status="model.downloadState === 'verifying' ? 'success' : 'default'"
              :percentage="model.sizeBytes ? model.downloadedBytes / model.sizeBytes * 100 : 0" />
            <div class="model-actions">
              <CheckCircle2 v-if="model.valid" :size="18" class="model-valid" :aria-label="$t('transcribe.models.installed')" />
              <button v-if="activeDownloadStates.has(model.downloadState)" class="icon-button" type="button" :title="$t('transcribe.models.cancelDownload')" @click="$emit('cancel', model.id)">
                <LoaderCircle class="spin" :size="18" />
              </button>
              <button v-else-if="!model.valid" class="icon-button" type="button" :title="$t('transcribe.models.download')" @click="$emit('download', model.id)">
                <Download :size="18" />
              </button>
              <button v-else class="icon-button" type="button" :title="$t('transcribe.models.delete')" :disabled="model.inUse" @click="$emit('delete', model.id)">
                <Trash2 :size="18" />
              </button>
            </div>
          </article>
        </div>
      </section>
    </div>
  </NModal>
</template>
