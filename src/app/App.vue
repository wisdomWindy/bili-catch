<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { darkTheme, NConfigProvider, NDialogProvider, NMessageProvider, NNotificationProvider } from "naive-ui";
import { useI18n } from "vue-i18n";
import { AlertTriangle } from "@lucide/vue";
import type { AppIpcService } from "../services/ipc/app";
import { installThemeSync, useAppStore } from "../stores/app";
import type { ResolvedTheme } from "../contracts/app";
import AppShell from "../components/layout/AppShell.vue";
import BackendBanner from "../components/shared/BackendBanner.vue";
import DownloadNotificationBridge from "../features/download-center/components/DownloadNotificationBridge.vue";
import { useAuthEventSource, useAuthService } from "../features/authentication/injection";
import { useAuthStore } from "../features/authentication/store";
import { useSettingsEffectSink, useSettingsService } from "../features/settings/injection";
import { useSettingsStore } from "../features/settings/store";

const props = defineProps<{ appService: AppIpcService }>();
const store = useAppStore();
const authStore = useAuthStore();
const authService = useAuthService();
const authEvents = useAuthEventSource();
const settingsStore = useSettingsStore();
const settingsService = useSettingsService();
const settingsEffects = useSettingsEffectSink();
const { locale, t } = useI18n();
const resolvedTheme = ref<ResolvedTheme>("light");
const naiveTheme = computed(() => resolvedTheme.value === "dark" ? darkTheme : null);
let disposeTheme: (() => void) | undefined;

watch(() => store.locale, (nextLocale) => {
  locale.value = nextLocale;
}, { immediate: true });

onMounted(() => {
  disposeTheme = installThemeSync(store, document.documentElement, undefined, (theme) => {
    resolvedTheme.value = theme;
  });
  void store.initialize(props.appService);
  void authStore.initialize(authService, authEvents);
  void settingsStore.initialize(settingsService, settingsEffects);
});

onBeforeUnmount(() => {
  disposeTheme?.();
  authStore.dispose();
  settingsStore.dispose();
});
</script>

<template>
  <NConfigProvider :theme="naiveTheme">
    <NDialogProvider>
      <NMessageProvider>
        <DownloadNotificationBridge>
          <NNotificationProvider :max="3">
            <AppShell>
              <template #status>
                <BackendBanner @retry="store.initialize(appService)" />
                <div v-if="authStore.eventError" class="auth-sync-warning" role="status">
                  <AlertTriangle :size="17" aria-hidden="true" />
                  <span>{{ t("auth.syncUnavailable") }}</span>
                </div>
              </template>
            </AppShell>
          </NNotificationProvider>
        </DownloadNotificationBridge>
      </NMessageProvider>
    </NDialogProvider>
  </NConfigProvider>
</template>
