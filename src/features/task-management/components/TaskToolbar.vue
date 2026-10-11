<script setup lang="ts">
import { Pause, Trash2 } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { CombinedTaskFilter } from "../projection";

defineProps<{ filter: CombinedTaskFilter; counts: Record<CombinedTaskFilter, number>; canPause: boolean; canClear: boolean; busy: boolean }>();
const emit = defineEmits<{ filter: [value: CombinedTaskFilter]; pauseAll: []; clearCompleted: [] }>();
const { t } = useI18n();
const filters: CombinedTaskFilter[] = ["all", "download", "transcription", "active", "completed", "failed"];
</script>

<template>
  <div class="task-toolbar">
    <div class="task-tabs" role="tablist" :aria-label="t('tasks.filters.label')">
      <button v-for="item in filters" :key="item" type="button" role="tab" :aria-selected="filter === item" @click="emit('filter', item)">
        {{ t(`tasks.filters.${item}`) }} <span>{{ counts[item] }}</span>
      </button>
    </div>
    <div class="task-batch-actions">
      <button type="button" :disabled="!canPause || busy" @click="emit('pauseAll')"><Pause :size="17" />{{ t('tasks.actions.pauseAll') }}</button>
      <button type="button" :disabled="!canClear || busy" @click="emit('clearCompleted')"><Trash2 :size="17" />{{ t('tasks.actions.clearCompleted') }}</button>
    </div>
  </div>
</template>
