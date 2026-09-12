<script setup lang="ts">
import type { FieldSaveState } from "../contracts";
import SettingRow from "./SettingRow.vue";

defineProps<{ id: string; field: "maxConcurrentTasks" | "connectionsPerTask"; label: string; description?: string; value: number; min: number; max: number; status: FieldSaveState }>();
const emit = defineEmits<{ preview: [value: number]; commit: [value: number] }>();
function numberValue(event: Event): number { return Number((event.target as HTMLInputElement).value); }
</script>

<template>
  <SettingRow :id="id" :field="field" :label="label" :description="description" :status="status">
    <div class="range-control">
      <input :id="id" type="range" :value="value" :min="min" :max="max" step="1" :aria-describedby="description ? `${id}-description ${id}-status` : `${id}-status`" :aria-valuetext="$t('settings.rangeValue', { value })" @input="emit('preview', numberValue($event))" @change="emit('commit', numberValue($event))" />
      <output :for="id">{{ value }}</output>
    </div>
  </SettingRow>
</template>
