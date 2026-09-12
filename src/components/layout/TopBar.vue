<script setup lang="ts">
import { ArrowLeft, LoaderCircle, Settings, UserRound } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import AppIconButton from "../shared/AppIconButton.vue";
import { useAuthStore } from "../../features/authentication/store";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const auth = useAuthStore();

const title = computed(() => {
  const key = route.meta.titleKey;
  return key ? t(key) : t("brand.name");
});
const canGoBack = computed(() => {
  void route.fullPath;
  return Boolean(router.options.history.state.back);
});
const accountLabel = computed(() => {
  if (auth.snapshot.status === "restoring") return t("auth.status.restoring");
  if (auth.snapshot.status !== "authenticated") return t("shell.anonymous");
  return auth.snapshot.account?.name.trim() || t("auth.accountFallback");
});
const restoringAuth = computed(() => auth.snapshot.status === "restoring");

function goBack() {
  if (canGoBack.value) router.back();
}

function openLogin() {
  const from = route.name === "login" ? undefined : route.fullPath;
  void router.push({ name: "login", query: from ? { from } : undefined });
}
</script>

<template>
  <header class="topbar">
    <AppIconButton
      data-testid="back-button"
      :label="t('shell.back')"
      :icon="ArrowLeft"
      :disabled="!canGoBack"
      @click="goBack"
    />
    <h1 class="topbar-title" data-testid="topbar-title" :aria-label="title" :title="title">{{ title }}</h1>
    <div class="topbar-actions">
      <button class="auth-button" data-testid="auth-button" type="button" :aria-label="accountLabel" :title="accountLabel" @click="openLogin">
        <LoaderCircle v-if="restoringAuth" class="spin" data-testid="auth-spinner" :size="18" :stroke-width="1.8" aria-hidden="true" />
        <UserRound v-else :size="18" :stroke-width="1.8" aria-hidden="true" />
        <span class="auth-button-label">{{ accountLabel }}</span>
      </button>
      <AppIconButton
        class="topbar-settings"
        data-testid="settings-button"
        :label="t('shell.openSettings')"
        :icon="Settings"
        @click="router.push({ name: 'settings' })"
      />
    </div>
  </header>
</template>
