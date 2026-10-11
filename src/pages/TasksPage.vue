<script setup lang="ts">
import { ListTodo } from "@lucide/vue";
import { computed, inject, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import EmptyState from "../components/shared/EmptyState.vue";
import TaskConfirmDialog from "../features/task-management/components/TaskConfirmDialog.vue";
import TaskRow from "../features/task-management/components/TaskRow.vue";
import TaskToolbar from "../features/task-management/components/TaskToolbar.vue";
import TranscriptionTaskRow from "../features/task-management/components/TranscriptionTaskRow.vue";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import type { DownloadTask, TaskAction } from "../features/task-management/contracts";
import {
  matchesCombinedFilter,
  projectDownloadTask,
  projectTranscriptionTask,
  type CombinedTaskFilter,
  type TaskListItem,
} from "../features/task-management/projection";
import { useTaskManagementStore } from "../features/task-management/store";
import { transcriptionEventSourceKey, transcriptionServiceKey } from "../features/transcription/injection";
import { useTranscriptionStore } from "../features/transcription/store";
import type { TranscribeTaskAction } from "../features/transcription/contracts";
import { useTaskDraftsStore } from "../stores/task-drafts";

const { t } = useI18n();
const router = useRouter();
const service = inject(taskServiceKey, null);
const eventSource = inject(taskEventSourceKey, null);
const store = useTaskManagementStore();
const transcriptionService = inject(transcriptionServiceKey, null);
const transcriptionEvents = inject(transcriptionEventSourceKey, null);
const transcriptionStore = useTranscriptionStore();
const drafts = useTaskDraftsStore();
const filter = ref<CombinedTaskFilter>("all");
const confirmation = ref<{ title: string; message: string; run: () => void } | null>(null);
const contentReady = ref(false);
const canPause = computed(() => contentReady.value && store.allTasks.some((task) => task.status === "downloading"));
const visibleError = computed(() => store.eventError ?? store.error);
const safeErrorText = computed(() => t(`errors.${visibleError.value?.code ?? "E_INTERNAL"}`));
const allItems = computed<TaskListItem[]>(() => contentReady.value ? [
  ...store.allTasks.map(projectDownloadTask),
  ...transcriptionStore.tasks.map(projectTranscriptionTask),
].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)) : []);
const visibleItems = computed(() => allItems.value.filter((item) => matchesCombinedFilter(item, filter.value)));
const counts = computed<Record<CombinedTaskFilter, number>>(() => ({
  all: allItems.value.length,
  download: allItems.value.filter((item) => item.kind === "download").length,
  transcription: allItems.value.filter((item) => item.kind === "transcription").length,
  active: allItems.value.filter((item) => item.statusGroup === "active").length,
  completed: allItems.value.filter((item) => item.statusGroup === "completed").length,
  failed: allItems.value.filter((item) => item.statusGroup === "failed").length,
}));
const canClear = computed(() => {
  if (filter.value === "active" || filter.value === "failed") return false;
  return visibleItems.value.some((item) => item.statusGroup === "completed");
});
const loading = computed(() => !contentReady.value || ["subscribing", "loading"].includes(store.status) || (transcriptionService && transcriptionStore.status === "loading"));
let presentationFrame: number | null = null;
let initializationTimer: number | null = null;

function initializeTaskStores() {
  initializationTimer = null;
  contentReady.value = true;
  if (service && eventSource) void store.initialize(service, eventSource, drafts);
  if (transcriptionService && transcriptionEvents) void transcriptionStore.initialize(transcriptionService, transcriptionEvents);
}

onMounted(() => {
  presentationFrame = window.requestAnimationFrame(() => {
    presentationFrame = null;
    initializationTimer = window.setTimeout(initializeTaskStores, 0);
  });
});
onUnmounted(() => {
  if (presentationFrame !== null) window.cancelAnimationFrame(presentationFrame);
  presentationFrame = null;
  if (initializationTimer !== null) window.clearTimeout(initializationTimer);
  initializationTimer = null;
});

function requestAction(task: DownloadTask, action: TaskAction) {
  if (action === "cancel" || (action === "delete" && task.status === "completed")) {
    confirmation.value = {
      title: t(`tasks.confirm.${action}Title`),
      message: t(`tasks.confirm.${action}Message`, { name: task.fileName }),
      run: () => { void store.control(task.id, action); },
    };
    return;
  }
  void store.control(task.id, action);
}

function requestClear() {
  confirmation.value = {
    title: t("tasks.confirm.clearTitle"),
    message: t("tasks.confirm.clearMessage", { count: visibleItems.value.filter((item) => item.statusGroup === "completed").length }),
    run: () => {
      if (filter.value !== "transcription") void store.clearCompleted();
      if (filter.value !== "download") void transcriptionStore.clearCompleted();
    },
  };
}

function requestTranscriptionAction(taskId: string, action: TranscribeTaskAction) {
  void transcriptionStore.control(taskId, action);
}

function openTranscription(taskId: string, retranscribe = false) {
  void router.push({ path: "/transcribe", query: retranscribe ? { taskId, action: "retranscribe" } : { taskId } });
}

function confirm() {
  const action = confirmation.value?.run;
  confirmation.value = null;
  action?.();
}
</script>

<template>
  <section class="tasks-page">
    <header class="tasks-heading">
      <div><h1 data-testid="page-heading">{{ t("pages.tasks.title") }}</h1><p>{{ t("tasks.subtitle") }}</p></div>
      <span v-if="contentReady && store.isNearCapacity" class="capacity-indicator" :class="{ 'capacity-indicator--full': store.isFull }">
        {{ t(store.isFull ? "tasks.capacity.full" : "tasks.capacity.near", { count: store.allTasks.length }) }}
      </span>
    </header>
    <p class="task-live-region" aria-live="polite" aria-atomic="true">
      {{ t("tasks.liveSummary", counts) }}
    </p>
    <TaskToolbar :filter="filter" :counts="counts" :can-pause="canPause" :can-clear="canClear"
      :busy="contentReady && Boolean(store.pending.batchPause || store.pending.batchClear || transcriptionStore.pending.clearCompleted)" @filter="filter = $event"
      @pause-all="store.pauseAll()" @clear-completed="requestClear" />
    <div v-if="contentReady && (store.eventError || (store.error && store.allTasks.length))" class="task-banner" role="status">
      {{ safeErrorText }}
    </div>
    <div v-if="loading" class="task-skeleton" aria-label="Loading tasks">
      <span v-for="index in 5" :key="index" />
    </div>
    <EmptyState v-else-if="store.status === 'failed' && !allItems.length" :icon="ListTodo" :title="t('tasks.loadFailed')" :message="safeErrorText" />
    <EmptyState v-else-if="!visibleItems.length" :icon="ListTodo" :title="allItems.length ? t('tasks.filteredEmpty') : t('pages.tasks.title')" :message="allItems.length ? t('tasks.filteredEmptyHint') : t('pages.tasks.empty')">
      <button v-if="!allItems.length" class="primary-button" type="button" @click="router.push('/download')">{{ t("actions.goDownload") }}</button>
    </EmptyState>
    <div v-else class="task-list" role="list">
      <div class="task-list__header" aria-hidden="true">
        <span>{{ t("tasks.columns.file") }}</span><span>{{ t("tasks.columns.status") }}</span><span>{{ t("tasks.columns.progress") }}</span>
        <span>{{ t("tasks.columns.speed") }}</span><span>{{ t("tasks.columns.remaining") }}</span><span>{{ t("tasks.columns.actions") }}</span>
      </div>
      <template v-for="item in visibleItems" :key="item.id">
        <TaskRow v-if="item.kind === 'download'" role="listitem" :task="item.task" :pending="Boolean(store.pending[item.task.id])"
          @action="requestAction(item.task, $event)" @open="store.open(item.task.id, $event)"
          @transcribe="router.push({ path: '/transcribe', query: { taskId: item.task.id } })" />
        <TranscriptionTaskRow v-else role="listitem" :task="item.task" :pending="Boolean(transcriptionStore.pending[item.task.id])"
          @action="requestTranscriptionAction(item.task.id, $event)" @view="openTranscription(item.task.id)"
          @open="transcriptionStore.openOutput(item.task.id)" @retranscribe="openTranscription(item.task.id, true)" />
      </template>
    </div>
    <TaskConfirmDialog :open="Boolean(confirmation)" :title="confirmation?.title ?? ''" :message="confirmation?.message ?? ''"
      @cancel="confirmation = null" @confirm="confirm" />
  </section>
</template>
