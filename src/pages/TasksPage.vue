<script setup lang="ts">
import { ListTodo } from "@lucide/vue";
import { computed, inject, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import EmptyState from "../components/shared/EmptyState.vue";
import TaskConfirmDialog from "../features/task-management/components/TaskConfirmDialog.vue";
import TaskRow from "../features/task-management/components/TaskRow.vue";
import TaskToolbar from "../features/task-management/components/TaskToolbar.vue";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import type { DownloadTask, TaskAction } from "../features/task-management/contracts";
import { useTaskManagementStore } from "../features/task-management/store";
import { useTaskDraftsStore } from "../stores/task-drafts";

const { t } = useI18n();
const router = useRouter();
const service = inject(taskServiceKey, null);
const eventSource = inject(taskEventSourceKey, null);
const store = useTaskManagementStore();
const drafts = useTaskDraftsStore();
const confirmation = ref<{ title: string; message: string; run: () => void } | null>(null);
const canPause = computed(() => store.allTasks.some((task) => task.status === "downloading"));
const visibleError = computed(() => store.eventError ?? store.error);
const safeErrorText = computed(() => t(`errors.${visibleError.value?.code ?? "E_INTERNAL"}`));

onMounted(() => {
  if (service && eventSource) void store.initialize(service, eventSource, drafts);
});
onUnmounted(() => store.dispose());

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
    message: t("tasks.confirm.clearMessage", { count: store.counts.completed }),
    run: () => { void store.clearCompleted(); },
  };
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
      <span v-if="store.isNearCapacity" class="capacity-indicator" :class="{ 'capacity-indicator--full': store.isFull }">
        {{ t(store.isFull ? "tasks.capacity.full" : "tasks.capacity.near", { count: store.allTasks.length }) }}
      </span>
    </header>
    <p class="task-live-region" aria-live="polite" aria-atomic="true">
      {{ t("tasks.liveSummary", store.counts) }}
    </p>
    <TaskToolbar :filter="store.filter" :counts="store.counts" :can-pause="canPause" :can-clear="store.counts.completed > 0"
      :busy="Boolean(store.pending.batchPause || store.pending.batchClear)" @filter="store.filter = $event"
      @pause-all="store.pauseAll()" @clear-completed="requestClear" />
    <div v-if="store.eventError || (store.error && store.allTasks.length)" class="task-banner" role="status">
      {{ safeErrorText }}
    </div>
    <div v-if="store.status === 'subscribing' || store.status === 'loading'" class="task-skeleton" aria-label="Loading tasks">
      <span v-for="index in 5" :key="index" />
    </div>
    <EmptyState v-else-if="store.status === 'failed' && !store.allTasks.length" :icon="ListTodo" :title="t('tasks.loadFailed')" :message="safeErrorText" />
    <EmptyState v-else-if="!store.visibleTasks.length" :icon="ListTodo" :title="store.allTasks.length ? t('tasks.filteredEmpty') : t('pages.tasks.title')" :message="store.allTasks.length ? t('tasks.filteredEmptyHint') : t('pages.tasks.empty')">
      <button v-if="!store.allTasks.length" class="primary-button" type="button" @click="router.push('/download')">{{ t("actions.goDownload") }}</button>
    </EmptyState>
    <div v-else class="task-list" role="list">
      <div class="task-list__header" aria-hidden="true">
        <span>{{ t("tasks.columns.file") }}</span><span>{{ t("tasks.columns.status") }}</span><span>{{ t("tasks.columns.progress") }}</span>
        <span>{{ t("tasks.columns.speed") }}</span><span>{{ t("tasks.columns.remaining") }}</span><span>{{ t("tasks.columns.actions") }}</span>
      </div>
      <TaskRow v-for="task in store.visibleTasks" :key="task.id" role="listitem" :task="task" :pending="Boolean(store.pending[task.id])"
        @action="requestAction(task, $event)" @open="store.open(task.id, $event)" />
    </div>
    <TaskConfirmDialog :open="Boolean(confirmation)" :title="confirmation?.title ?? ''" :message="confirmation?.message ?? ''"
      @cancel="confirmation = null" @confirm="confirm" />
  </section>
</template>
