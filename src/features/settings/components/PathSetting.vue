<script setup lang="ts">
import { FolderOpen, LoaderCircle } from "@lucide/vue";
import type { FieldSaveState } from "../contracts";
import SettingRow from "./SettingRow.vue";

defineProps<{ id: string; field: "downloadDirectory" | "temporaryDirectory"; label: string; description?: string; value: string; status: FieldSaveState; picking: boolean; selectLabel: string }>();
const emit = defineEmits<{ select: [button: HTMLButtonElement] }>();
function select(event: MouseEvent) { emit("select", event.currentTarget as HTMLButtonElement); }
</script>

<template>
  <SettingRow :id="id" :field="field" :label="label" :description="description" :status="status">
    <div class="path-control">
      <input :id="id" type="text" :value="value" readonly :title="value" :aria-label="`${label}: ${value}`" :aria-describedby="description ? `${id}-description ${id}-status` : `${id}-status`" />
      <button type="button" class="icon-button" :disabled="picking" :aria-label="selectLabel" :title="selectLabel" @click="select">
        <LoaderCircle v-if="picking" :size="18" class="spin" aria-hidden="true" />
        <FolderOpen v-else :size="18" aria-hidden="true" />
      </button>
    </div>
  </SettingRow>
</template>
