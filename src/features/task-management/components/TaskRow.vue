<script setup lang="ts">
import { FilePlay, FolderOpen, MoreHorizontal, Pause, Play, RotateCcw, Trash2, X } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import AppIconButton from "../../../components/shared/AppIconButton.vue";
import { actionsForStatus } from "../action-policy";
import type { DownloadTask, OpenTarget, TaskAction } from "../contracts";
import { formatBytes, formatEta, formatSpeed } from "../formatters";
import TaskProgress from "./TaskProgress.vue";
import TaskStatus from "./TaskStatus.vue";

const props = defineProps<{ task: DownloadTask; pending: boolean }>();
const emit = defineEmits<{ action: [action: TaskAction]; open: [target: OpenTarget] }>();
const { t } = useI18n();
const actions = computed(() => actionsForStatus(props.task.status));
const openMenu = ref<HTMLDetailsElement>();
const size = computed(() => props.task.totalBytes
  ? `${formatBytes(props.task.bytesDownloaded)} / ${formatBytes(props.task.totalBytes)}`
  : formatBytes(props.task.bytesDownloaded));
const audioProfile = computed(() => {
  if (props.task.mode !== "audio-only" || !props.task.audioFormat) return null;
  if (props.task.audioFormat === "mp3") return `${props.task.audioFormat.toUpperCase()} ${props.task.audioBitrateId ?? ""}K`.trim();
  if (props.task.audioFormat === "m4a") return `${props.task.audioFormat.toUpperCase()} ${t("download.originalBitrate")}`;
  return `${props.task.audioFormat.toUpperCase()} ${t("download.lossless")}`;
});
const media = computed(() => [
  `P${props.task.page}`,
  t(`download.modes.${props.task.mode}`),
  props.task.codec?.toUpperCase() ?? audioProfile.value,
].filter(Boolean).join(" · "));
const safeError = computed(() => props.task.error ? t(`errors.${props.task.error.code}`) : "");
const actionIcon = { pause: Pause, resume: Play, cancel: X, retry: RotateCcw, delete: Trash2 };
const actionLabel = computed<Record<TaskAction, string>>(() => ({
  pause: t("tasks.actions.pause"), resume: t("tasks.actions.resume"),
  cancel: t("tasks.actions.cancel"), retry: t("tasks.actions.retry"), delete: t("tasks.actions.delete"),
}));
function open(target: OpenTarget) {
  emit("open", target);
  if (openMenu.value) openMenu.value.open = false;
}
</script>

<template>
  <article class="task-row" :aria-busy="pending">
    <div class="task-row__file">
      <strong :title="task.fileName" tabindex="0">{{ task.fileName }}</strong>
      <span>{{ media }}</span>
      <span v-if="task.error && (task.status === 'failed' || task.status === 'queued')" class="task-row__error">
        {{ safeError }}
      </span>
    </div>
    <div class="task-row__status"><TaskStatus :status="task.status" /></div>
    <div class="task-row__progress"><TaskProgress :value="task.progressPercent" :processing="task.status === 'processing'" /></div>
    <div class="task-row__metric" data-label="Speed">
      {{ task.status === 'downloading' && task.speedBytesPerSecond !== '0' ? formatSpeed(task.speedBytesPerSecond) : '—' }}
    </div>
    <div class="task-row__metric task-row__size" data-label="ETA / Size">
      <span>{{ task.status === 'downloading' ? formatEta(task.etaSeconds) : '—' }}</span>
      <small>{{ size }}</small>
    </div>
    <div class="task-row__actions">
      <template v-if="task.status === 'completed'">
        <details ref="openMenu" class="task-open-menu">
          <summary :aria-label="t('tasks.actions.openOptions')" :title="t('tasks.actions.openOptions')">
            <MoreHorizontal :size="19" :stroke-width="1.8" aria-hidden="true" />
          </summary>
          <div class="task-open-menu__panel">
            <button type="button" :aria-label="t('tasks.actions.openFile')" :disabled="pending || !task.outputPath" @click="open('file')">
              <FilePlay :size="17" aria-hidden="true" />{{ t('tasks.actions.openFile') }}
            </button>
            <button type="button" :aria-label="t('tasks.actions.openDirectory')" :disabled="pending || !task.outputPath" @click="open('directory')">
              <FolderOpen :size="17" aria-hidden="true" />{{ t('tasks.actions.openDirectory') }}
            </button>
          </div>
        </details>
      </template>
      <AppIconButton
        v-for="action in actions"
        :key="action"
        :icon="actionIcon[action]"
        :label="actionLabel[action]"
        :disabled="pending"
        @click="emit('action', action)"
      />
    </div>
  </article>
</template>
