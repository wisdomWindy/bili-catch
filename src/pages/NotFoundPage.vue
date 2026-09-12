<script setup lang="ts">
import { FileQuestion } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import EmptyState from "../components/shared/EmptyState.vue";

const router = useRouter();
const { t } = useI18n();
</script>

<template>
  <EmptyState :icon="FileQuestion" :title="t('errors.notFound')" :message="t('errors.notFoundMessage')">
    <button class="primary-button" data-testid="not-found-home" type="button" @click="router.push({ name: 'download' })">
      {{ t("actions.goDownload") }}
    </button>
  </EmptyState>
</template>
