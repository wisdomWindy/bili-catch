<script setup lang="ts">
import { FolderOpen, ListPlus, Trash2 } from "@lucide/vue";

defineProps<{ outputDir: string; canEnqueue: boolean; busy: boolean }>();
const emit = defineEmits<{ clear: []; enqueue: [] }>();
</script>

<template>
  <section class="action-bar" :aria-label="$t('download.actions')">
    <label class="output-field">
      <span>{{ $t("download.outputDir") }}</span>
      <span class="output-control">
        <input :value="outputDir" readonly />
        <span :title="$t('download.outputDirPending')">
          <button type="button" disabled :aria-label="$t('download.changeOutputDir')">
            <FolderOpen :size="17" aria-hidden="true" />
          </button>
        </span>
      </span>
    </label>
    <div class="action-buttons">
      <button class="secondary-button" type="button" :disabled="busy" @click="emit('clear')">
        <Trash2 :size="17" aria-hidden="true" />{{ $t("download.clear") }}
      </button>
      <button class="primary-button" type="button" :disabled="!canEnqueue || busy" @click="emit('enqueue')">
        <ListPlus :size="17" aria-hidden="true" />{{ $t("download.enqueue") }}
      </button>
    </div>
  </section>
</template>
