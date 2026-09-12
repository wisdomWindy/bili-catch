<script setup lang="ts">
import type { FieldSaveState, SettingKey } from "../contracts";
import FieldSaveStatus from "./FieldSaveStatus.vue";
defineProps<{ id: string; field: SettingKey; label: string; description?: string; status: FieldSaveState }>();
</script>

<template>
  <div class="setting-row" :data-setting-field="field">
    <div class="setting-row__label"><label :for="id">{{ label }}</label><p v-if="description" :id="`${id}-description`">{{ description }}</p></div>
    <div class="setting-row__control"><slot /></div>
    <FieldSaveStatus :id="`${id}-status`" :state="status" />
  </div>
</template>
