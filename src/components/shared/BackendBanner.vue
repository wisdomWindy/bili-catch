<script setup lang="ts">
import { AlertTriangle, LoaderCircle, RotateCw } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../../stores/app";

defineEmits<{ retry: [] }>();

const store = useAppStore();
const { t } = useI18n();
</script>

<template>
  <div v-if="store.backendStatus === 'pending'" class="backend-banner backend-banner--pending" role="status">
    <LoaderCircle class="spin" :size="17" aria-hidden="true" />
    <span>{{ t("status.checking") }}</span>
  </div>
  <div v-else-if="store.backendStatus === 'failed'" class="backend-banner backend-banner--error" role="alert">
    <AlertTriangle :size="17" aria-hidden="true" />
    <span>{{ t(`errors.${store.backendError?.code ?? 'E_INTERNAL'}`) }}</span>
    <button type="button" data-testid="health-retry" @click="$emit('retry')">
      <RotateCw :size="15" aria-hidden="true" />
      {{ t("actions.retry") }}
    </button>
  </div>
</template>
