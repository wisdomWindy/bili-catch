<script setup lang="ts">
import type { AudioFormat, VideoQualityId } from "../../../contracts/media";
import type { FieldSaveState, SettingKey, SettingsPatch, SettingsValues } from "../contracts";
import { AUDIO_FORMAT_OPTIONS, VIDEO_QUALITY_OPTIONS } from "../options";
import PathSetting from "./PathSetting.vue";
import RangeSetting from "./RangeSetting.vue";
import SettingRow from "./SettingRow.vue";
import SettingSection from "./SettingSection.vue";

defineProps<{ values: SettingsValues; fieldStates: Record<SettingKey, FieldSaveState>; picking: Record<"downloadDirectory" | "temporaryDirectory", boolean> }>();
const emit = defineEmits<{ preview: [patch: SettingsPatch]; commit: [patch: SettingsPatch]; pick: [field: "downloadDirectory" | "temporaryDirectory", button: HTMLButtonElement] }>();
function selected(event: Event): string { return (event.target as HTMLSelectElement).value; }
</script>

<template>
  <SettingSection id="download-settings" :title="$t('settings.sections.download')">
    <PathSetting id="setting-download-directory" field="downloadDirectory" :label="$t('settings.fields.downloadDirectory')" :select-label="$t('settings.actions.selectDownload')" :value="values.downloadDirectory" :status="fieldStates.downloadDirectory" :picking="picking.downloadDirectory" @select="emit('pick', 'downloadDirectory', $event)" />
    <PathSetting id="setting-temporary-directory" field="temporaryDirectory" :label="$t('settings.fields.temporaryDirectory')" :select-label="$t('settings.actions.selectTemporary')" :value="values.temporaryDirectory" :status="fieldStates.temporaryDirectory" :picking="picking.temporaryDirectory" @select="emit('pick', 'temporaryDirectory', $event)" />
    <RangeSetting id="setting-max-concurrent" field="maxConcurrentTasks" :label="$t('settings.fields.maxConcurrentTasks')" :value="values.maxConcurrentTasks" :min="1" :max="10" :status="fieldStates.maxConcurrentTasks" @preview="emit('preview', { field: 'maxConcurrentTasks', value: $event })" @commit="emit('commit', { field: 'maxConcurrentTasks', value: $event })" />
    <RangeSetting id="setting-connections" field="connectionsPerTask" :label="$t('settings.fields.connectionsPerTask')" :value="values.connectionsPerTask" :min="1" :max="32" :status="fieldStates.connectionsPerTask" @preview="emit('preview', { field: 'connectionsPerTask', value: $event })" @commit="emit('commit', { field: 'connectionsPerTask', value: $event })" />
    <SettingRow id="setting-video-quality" field="defaultVideoQuality" :label="$t('settings.fields.defaultVideoQuality')" :status="fieldStates.defaultVideoQuality">
      <select id="setting-video-quality" :value="values.defaultVideoQuality" aria-describedby="setting-video-quality-status" @change="emit('commit', { field: 'defaultVideoQuality', value: selected($event) as VideoQualityId })">
        <option v-for="option in VIDEO_QUALITY_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
    <SettingRow id="setting-audio-format" field="defaultAudioFormat" :label="$t('settings.fields.defaultAudioFormat')" :status="fieldStates.defaultAudioFormat">
      <select id="setting-audio-format" :value="values.defaultAudioFormat" aria-describedby="setting-audio-format-status" @change="emit('commit', { field: 'defaultAudioFormat', value: selected($event) as AudioFormat })">
        <option v-for="option in AUDIO_FORMAT_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
  </SettingSection>
</template>
