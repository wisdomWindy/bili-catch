<script setup lang="ts">
import { Search } from "@lucide/vue";

defineProps<{
  modelValue: string;
  busy: boolean;
  canParse: boolean;
  invalid: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  submit: [value?: string];
}>();

function acceptTransferredText(value: string) {
  emit("update:modelValue", value);
  emit("submit", value);
}

function onPaste(event: ClipboardEvent) {
  const value = event.clipboardData?.getData("text/plain");
  if (value === undefined) return;
  event.preventDefault();
  acceptTransferredText(value);
}

function onDrop(event: DragEvent) {
  const value = event.dataTransfer?.getData("text/plain");
  if (!value) return;
  event.preventDefault();
  acceptTransferredText(value);
}
</script>

<template>
  <section class="input-panel" aria-labelledby="download-input-title" @dragover.prevent @drop="onDrop">
    <div>
      <h2 id="download-input-title">{{ $t("download.inputTitle") }}</h2>
      <p id="download-input-hint">{{ $t("download.inputHint") }}</p>
    </div>
    <form class="parse-form" @submit.prevent="emit('submit')">
      <label class="sr-only" for="video-input">{{ $t("download.inputLabel") }}</label>
      <input
        id="video-input"
        :value="modelValue"
        :disabled="busy"
        :aria-invalid="invalid"
        aria-describedby="download-input-hint"
        :placeholder="$t('download.inputPlaceholder')"
        autocomplete="off"
        spellcheck="false"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        @paste="onPaste"
      />
      <button class="primary-button parse-button" type="submit" :disabled="!canParse">
        <Search :size="17" aria-hidden="true" />
        {{ busy ? $t("download.parsing") : $t("download.parse") }}
      </button>
    </form>
  </section>
</template>
