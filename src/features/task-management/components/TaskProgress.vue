<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{ value: number; processing?: boolean }>();
const { t } = useI18n();
</script>

<template>
  <div class="task-progress">
    <div
      class="task-progress__track"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-valuenow="processing ? undefined : value"
      :aria-valuetext="processing ? t('tasks.statuses.processing') : undefined"
    >
      <span :style="{ width: `${value}%` }" :class="{ 'task-progress__fill--busy': processing }" />
    </div>
    <span class="task-progress__value">{{ value }}%</span>
  </div>
</template>
