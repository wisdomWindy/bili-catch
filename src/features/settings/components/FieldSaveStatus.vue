<script setup lang="ts">
import { Check, CircleAlert, LoaderCircle } from "@lucide/vue";
import type { FieldSaveState } from "../contracts";
defineProps<{ id: string; state: FieldSaveState }>();
</script>

<template>
  <span :id="id" class="field-save-status" :class="`field-save-status--${state.status}`" aria-live="polite">
    <template v-if="state.status === 'saving'"><LoaderCircle :size="15" class="spin" aria-hidden="true" /><span>{{ $t("settings.status.saving") }}</span></template>
    <template v-else-if="state.status === 'saved'"><Check :size="15" aria-hidden="true" /><span>{{ $t("settings.status.saved") }}</span></template>
    <template v-else-if="state.status === 'error'"><CircleAlert :size="15" aria-hidden="true" /><span>{{ $t("settings.status.saveFailed") }}</span></template>
  </span>
</template>
