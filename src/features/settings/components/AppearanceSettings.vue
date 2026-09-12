<script setup lang="ts">
import type { AppLocale, ThemePreference } from "../../../contracts/app";
import type { FieldSaveState, SettingKey, SettingsPatch, SettingsValues } from "../contracts";
import { LOCALE_OPTIONS, THEME_OPTIONS } from "../options";
import SettingRow from "./SettingRow.vue";
import SettingSection from "./SettingSection.vue";

defineProps<{ values: SettingsValues; fieldStates: Record<SettingKey, FieldSaveState> }>();
const emit = defineEmits<{ commit: [patch: SettingsPatch] }>();
function selected(event: Event): string { return (event.target as HTMLSelectElement).value; }
</script>

<template>
  <SettingSection id="appearance-settings" :title="$t('settings.sections.appearance')">
    <SettingRow id="setting-theme" field="theme" :label="$t('settings.fields.theme')" :status="fieldStates.theme">
      <select id="setting-theme" :value="values.theme" aria-describedby="setting-theme-status" @change="emit('commit', { field: 'theme', value: selected($event) as ThemePreference })"><option v-for="option in THEME_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option></select>
    </SettingRow>
    <SettingRow id="setting-locale" field="locale" :label="$t('settings.fields.locale')" :status="fieldStates.locale">
      <select id="setting-locale" :value="values.locale" aria-describedby="setting-locale-status" @change="emit('commit', { field: 'locale', value: selected($event) as AppLocale })"><option v-for="option in LOCALE_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option></select>
    </SettingRow>
  </SettingSection>
</template>
