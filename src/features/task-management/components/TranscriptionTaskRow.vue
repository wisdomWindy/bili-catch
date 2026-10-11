<script setup lang="ts">
import { Eye, FolderOpen, RotateCcw, Trash2, X } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import AppIconButton from "../../../components/shared/AppIconButton.vue";
import type { TranscribeTask, TranscribeTaskAction } from "../../transcription/contracts";
import TaskProgress from "./TaskProgress.vue";

const props = defineProps<{ task: TranscribeTask; pending: boolean }>();
const emit = defineEmits<{
  action: [action: TranscribeTaskAction];
  view: [];
  open: [];
  retranscribe: [];
}>();
const { t } = useI18n();
const canCancel = computed(() => ["pending", "preprocessing", "loadingModel", "transcribing", "processing"].includes(props.task.status));
const canRetry = computed(() => ["failed", "cancelled"].includes(props.task.status));
const terminal = computed(() => ["completed", "failed", "cancelled"].includes(props.task.status));
const safeError = computed(() => props.task.error ? t(`errors.${props.task.error.code}`) : "");
</script>

<template>
  <article class="task-row" :aria-busy="pending">
    <div class="task-row__file">
      <strong :title="task.sourceInfo.path" tabindex="0">{{ task.sourceInfo.fileName }}</strong>
      <span>{{ t("tasks.kinds.transcription") }} · {{ task.options.modelId }} · {{ task.options.outputFormat.toUpperCase() }}</span>
      <span v-if="task.error" class="task-row__error">{{ safeError }}</span>
    </div>
    <div class="task-row__status"><span class="task-status" :data-status="task.status">{{ t(`tasks.transcriptionStatuses.${task.status}`) }}</span></div>
    <div class="task-row__progress"><TaskProgress :value="Math.round(task.progress.progress * 100)" :processing="['processing', 'saving'].includes(task.status)" /></div>
    <div class="task-row__metric">{{ task.progress.speed ? `${task.progress.speed.toFixed(1)}x` : "—" }}</div>
    <div class="task-row__metric task-row__size"><span>{{ Math.round(task.progress.processedSeconds) }}s</span><small>{{ task.progress.totalSeconds ? `/ ${Math.round(task.progress.totalSeconds)}s` : "" }}</small></div>
    <div class="task-row__actions">
      <AppIconButton v-if="task.status === 'completed'" :icon="Eye" :label="t('tasks.actions.viewResult')" :disabled="pending" @click="emit('view')" />
      <AppIconButton v-if="task.status === 'completed'" :icon="FolderOpen" :label="t('tasks.actions.openDirectory')" :disabled="pending || !task.outputPath" @click="emit('open')" />
      <AppIconButton v-if="task.status === 'completed'" :icon="RotateCcw" :label="t('tasks.actions.retranscribe')" :disabled="pending" @click="emit('retranscribe')" />
      <AppIconButton v-if="canCancel" :icon="X" :label="t('tasks.actions.cancel')" :disabled="pending" @click="emit('action', 'cancel')" />
      <AppIconButton v-if="canRetry" :icon="RotateCcw" :label="t('tasks.actions.retry')" :disabled="pending" @click="emit('action', 'retry')" />
      <AppIconButton v-if="terminal" :icon="Trash2" :label="t('tasks.actions.delete')" :disabled="pending" @click="emit('action', 'delete')" />
    </div>
  </article>
</template>
