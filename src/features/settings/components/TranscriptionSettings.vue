<script setup lang="ts">
import type {
  AsrModelId,
  GpuMode,
  TranscriptFormat,
  TranscriptionLanguage,
} from "../../transcription/contracts";
import type { FieldSaveState, SettingKey, SettingsPatch, SettingsValues } from "../contracts";
import {
  ASR_MODEL_OPTIONS,
  GPU_MODE_OPTIONS,
  TRANSCRIPT_FORMAT_OPTIONS,
  TRANSCRIPTION_LANGUAGE_OPTIONS,
} from "../options";
import PathSetting from "./PathSetting.vue";
import RangeSetting from "./RangeSetting.vue";
import SettingRow from "./SettingRow.vue";
import SettingSection from "./SettingSection.vue";

defineProps<{
  values: SettingsValues;
  fieldStates: Record<SettingKey, FieldSaveState>;
  picking: boolean;
}>();

const emit = defineEmits<{
  preview: [patch: SettingsPatch];
  commit: [patch: SettingsPatch];
  pick: [field: "asrModelDirectory", button: HTMLButtonElement];
}>();

function selected(event: Event): string { return (event.target as HTMLSelectElement).value; }
function checked(event: Event): boolean { return (event.target as HTMLInputElement).checked; }
function textValue(event: Event): string { return (event.target as HTMLInputElement).value.trim(); }
</script>

<template>
  <SettingSection id="transcription-settings" :title="$t('settings.sections.transcription')">
    <SettingRow id="setting-asr-model" field="defaultAsrModel" :label="$t('settings.fields.defaultAsrModel')" :status="fieldStates.defaultAsrModel">
      <select id="setting-asr-model" :value="values.defaultAsrModel" aria-describedby="setting-asr-model-status" @change="emit('commit', { field: 'defaultAsrModel', value: selected($event) as AsrModelId })">
        <option v-for="option in ASR_MODEL_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
    <SettingRow id="setting-transcription-language" field="defaultTranscriptionLanguage" :label="$t('settings.fields.defaultTranscriptionLanguage')" :status="fieldStates.defaultTranscriptionLanguage">
      <select id="setting-transcription-language" :value="values.defaultTranscriptionLanguage" aria-describedby="setting-transcription-language-status" @change="emit('commit', { field: 'defaultTranscriptionLanguage', value: selected($event) as TranscriptionLanguage })">
        <option v-for="option in TRANSCRIPTION_LANGUAGE_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
    <SettingRow id="setting-transcript-format" field="defaultTranscriptFormat" :label="$t('settings.fields.defaultTranscriptFormat')" :status="fieldStates.defaultTranscriptFormat">
      <select id="setting-transcript-format" :value="values.defaultTranscriptFormat" aria-describedby="setting-transcript-format-status" @change="emit('commit', { field: 'defaultTranscriptFormat', value: selected($event) as TranscriptFormat })">
        <option v-for="option in TRANSCRIPT_FORMAT_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
    <SettingRow id="setting-transcription-timestamps" field="transcriptionTimestamps" :label="$t('settings.fields.transcriptionTimestamps')" :status="fieldStates.transcriptionTimestamps"><input id="setting-transcription-timestamps" class="switch-control" type="checkbox" role="switch" :checked="values.transcriptionTimestamps" aria-describedby="setting-transcription-timestamps-status" @change="emit('commit', { field: 'transcriptionTimestamps', value: checked($event) })" /></SettingRow>
    <SettingRow id="setting-transcription-punctuation" field="transcriptionPunctuation" :label="$t('settings.fields.transcriptionPunctuation')" :status="fieldStates.transcriptionPunctuation"><input id="setting-transcription-punctuation" class="switch-control" type="checkbox" role="switch" :checked="values.transcriptionPunctuation" aria-describedby="setting-transcription-punctuation-status" @change="emit('commit', { field: 'transcriptionPunctuation', value: checked($event) })" /></SettingRow>
    <SettingRow id="setting-prefer-platform-subtitles" field="preferPlatformSubtitles" :label="$t('settings.fields.preferPlatformSubtitles')" :status="fieldStates.preferPlatformSubtitles"><input id="setting-prefer-platform-subtitles" class="switch-control" type="checkbox" role="switch" :checked="values.preferPlatformSubtitles" aria-describedby="setting-prefer-platform-subtitles-status" @change="emit('commit', { field: 'preferPlatformSubtitles', value: checked($event) })" /></SettingRow>
    <SettingRow id="setting-auto-transcribe-downloads" field="autoTranscribeDownloads" :label="$t('settings.fields.autoTranscribeDownloads')" :status="fieldStates.autoTranscribeDownloads"><input id="setting-auto-transcribe-downloads" class="switch-control" type="checkbox" role="switch" :checked="values.autoTranscribeDownloads" aria-describedby="setting-auto-transcribe-downloads-status" @change="emit('commit', { field: 'autoTranscribeDownloads', value: checked($event) })" /></SettingRow>
    <RangeSetting id="setting-max-transcriptions" field="maxConcurrentTranscriptions" :label="$t('settings.fields.maxConcurrentTranscriptions')" :value="values.maxConcurrentTranscriptions" :min="1" :max="3" :status="fieldStates.maxConcurrentTranscriptions" @preview="emit('preview', { field: 'maxConcurrentTranscriptions', value: $event })" @commit="emit('commit', { field: 'maxConcurrentTranscriptions', value: $event })" />
    <PathSetting id="setting-asr-model-directory" field="asrModelDirectory" :label="$t('settings.fields.asrModelDirectory')" :select-label="$t('settings.actions.selectAsrModel')" :value="values.asrModelDirectory" :status="fieldStates.asrModelDirectory" :picking="picking" @select="emit('pick', 'asrModelDirectory', $event)" />
    <SettingRow id="setting-asr-gpu-mode" field="asrGpuMode" :label="$t('settings.fields.asrGpuMode')" :status="fieldStates.asrGpuMode">
      <select id="setting-asr-gpu-mode" :value="values.asrGpuMode" aria-describedby="setting-asr-gpu-mode-status" @change="emit('commit', { field: 'asrGpuMode', value: selected($event) as GpuMode })">
        <option v-for="option in GPU_MODE_OPTIONS" :key="option.value" :value="option.value">{{ $t(option.labelKey) }}</option>
      </select>
    </SettingRow>
    <SettingRow id="setting-asr-model-source" field="asrModelSource" :label="$t('settings.fields.asrModelSource')" :description="$t('settings.fields.asrModelSourceDescription')" :status="fieldStates.asrModelSource">
      <input id="setting-asr-model-source" type="url" :value="values.asrModelSource" :placeholder="$t('settings.fields.asrModelSourcePlaceholder')" aria-describedby="setting-asr-model-source-description setting-asr-model-source-status" @change="emit('commit', { field: 'asrModelSource', value: textValue($event) })" />
    </SettingRow>
  </SettingSection>
</template>
