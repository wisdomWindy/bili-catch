<script setup lang="ts">
import type { CloseBehavior, FieldSaveState, SettingKey, SettingsPatch, SettingsValues } from "../contracts";
import { CLOSE_BEHAVIOR_OPTIONS } from "../options";
import SettingRow from "./SettingRow.vue";
import SettingSection from "./SettingSection.vue";

defineProps<{ values: SettingsValues; fieldStates: Record<SettingKey, FieldSaveState> }>();
const emit = defineEmits<{ commit: [patch: SettingsPatch] }>();
function selected(event: Event): CloseBehavior { return (event.target as HTMLSelectElement).value as CloseBehavior; }
function checked(event: Event): boolean { return (event.target as HTMLInputElement).checked; }
</script>

<template>
  <SettingSection id="system-settings" :title="$t('settings.sections.system')">
    <SettingRow id="setting-notify" field="notifyOnComplete" :label="$t('settings.fields.notifyOnComplete')" :status="fieldStates.notifyOnComplete"><input id="setting-notify" class="switch-control" type="checkbox" role="switch" :checked="values.notifyOnComplete" aria-describedby="setting-notify-status" @change="emit('commit', { field: 'notifyOnComplete', value: checked($event) })" /></SettingRow>
    <SettingRow id="setting-completion-sound" field="completionSound" :label="$t('settings.fields.completionSound')" :status="fieldStates.completionSound"><input id="setting-completion-sound" class="switch-control" type="checkbox" role="switch" :checked="values.completionSound" :disabled="!values.notifyOnComplete" aria-describedby="setting-completion-sound-status" @change="emit('commit', { field: 'completionSound', value: checked($event) })" /></SettingRow>
    <SettingRow id="setting-close-behavior" field="closeBehavior" :label="$t('settings.fields.closeBehavior')" :status="fieldStates.closeBehavior">
      <select id="setting-close-behavior" :value="values.closeBehavior" aria-describedby="setting-close-behavior-status" @change="emit('commit', { field: 'closeBehavior', value: selected($event) })"><option v-for="option in CLOSE_BEHAVIOR_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option></select>
    </SettingRow>
    <SettingRow id="setting-auto-updates" field="autoCheckUpdates" :label="$t('settings.fields.autoCheckUpdates')" :status="fieldStates.autoCheckUpdates"><input id="setting-auto-updates" class="switch-control" type="checkbox" role="switch" :checked="values.autoCheckUpdates" aria-describedby="setting-auto-updates-status" @change="emit('commit', { field: 'autoCheckUpdates', value: checked($event) })" /></SettingRow>
  </SettingSection>
</template>
