<script setup lang="ts">
import { FileAudio, FolderOpen } from "@lucide/vue";
import type { MediaSourceInfo } from "../contracts";

defineProps<{
  path: string;
  info: MediaSourceInfo | null;
  inspecting: boolean;
}>();
defineEmits<{
  "update:path": [value: string];
  browse: [];
  inspect: [];
  drop: [event: DragEvent];
}>();
</script>

<template>
  <section class="transcribe-section source-panel" @dragover.prevent @drop.prevent="$emit('drop', $event)">
    <header><FileAudio :size="19" aria-hidden="true" /><h2>{{ $t("transcribe.source.title") }}</h2></header>
    <div class="source-input-row">
      <label class="sr-only" for="transcription-source">{{ $t("transcribe.source.path") }}</label>
      <input id="transcription-source" :value="path" :placeholder="$t('transcribe.source.placeholder')"
        @input="$emit('update:path', ($event.target as HTMLInputElement).value)" @keydown.enter="$emit('inspect')" />
      <button class="icon-button source-browse" type="button" :title="$t('transcribe.source.browse')" @click="$emit('browse')">
        <FolderOpen :size="18" aria-hidden="true" />
      </button>
      <button class="secondary-button" type="button" :disabled="!path.trim() || inspecting" @click="$emit('inspect')">
        {{ inspecting ? $t("transcribe.source.inspecting") : $t("transcribe.source.inspect") }}
      </button>
    </div>
    <div v-if="info" class="source-meta" role="status">
      <strong>{{ info.fileName }}</strong>
      <span>{{ info.format.toUpperCase() }}</span>
      <span>{{ $t("transcribe.source.minutes", { count: Math.ceil(info.durationSeconds / 60) }) }}</span>
      <span>{{ (info.sizeBytes / 1024 / 1024).toFixed(1) }} MB</span>
    </div>
    <p v-else class="source-drop-hint">{{ $t("transcribe.source.formats") }}</p>
  </section>
</template>
