<script setup lang="ts">
import { CircleCheck, CirclePause, CircleX, Clock3, Download, LoaderCircle, TriangleAlert } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { TaskStatus } from "../contracts";

const props = defineProps<{ status: TaskStatus }>();
const { t } = useI18n();
const icon = computed(() => ({
  queued: Clock3, downloading: Download, paused: CirclePause, processing: LoaderCircle,
  completed: CircleCheck, failed: TriangleAlert, cancelled: CircleX,
})[props.status]);
</script>

<template>
  <span class="task-status" :class="`task-status--${status}`">
    <component :is="icon" :size="15" :class="{ spin: status === 'processing' }" aria-hidden="true" />
    {{ t(`tasks.statuses.${status}`) }}
  </span>
</template>
