<script setup lang="ts">
import { CheckCircle2, Download, LoaderCircle, Trash2 } from "@lucide/vue";
import { NModal, NProgress } from "naive-ui";
import type { AsrModel, AsrModelId } from "../contracts";

defineProps<{ show: boolean; models: AsrModel[] }>();
defineEmits<{
  close: [];
  download: [id: AsrModelId];
  cancel: [id: AsrModelId];
  delete: [id: AsrModelId];
}>();

const activeDownloadStates = new Set<AsrModel["downloadState"]>(["queued", "downloading", "verifying"]);
</script>

<template>
  <NModal :show="show" preset="card" class="model-modal" :title="$t('transcribe.models.title')"
    @close="$emit('close')" @mask-click="$emit('close')" @esc="$emit('close')">
    <p class="model-download-hint">{{ $t("transcribe.models.onDemandHint") }}</p>
    <div class="model-list">
      <article v-for="model in models" :key="model.id" class="model-row">
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
  </NModal>
</template>
