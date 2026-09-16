<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { AlertTriangle } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import AboutSettings from "../features/settings/components/AboutSettings.vue";
import AppearanceSettings from "../features/settings/components/AppearanceSettings.vue";
import DownloadSettings from "../features/settings/components/DownloadSettings.vue";
import SystemSettings from "../features/settings/components/SystemSettings.vue";
import type { SettingsPatch } from "../features/settings/contracts";
import { useDirectoryPicker, useReleaseActions, useSettingsEffectSink, useSettingsService } from "../features/settings/injection";
import { useSettingsStore } from "../features/settings/store";
import { useAppStore } from "../stores/app";

const { t } = useI18n();
const store = useSettingsStore();
const appStore = useAppStore();
const service = useSettingsService();
const effects = useSettingsEffectSink();
const picker = useDirectoryPicker();
const release = useReleaseActions();
const picking = reactive({ downloadDirectory: false, temporaryDirectory: false });
const updateStatus = ref<"idle" | "checking" | "installing" | "latest" | "available" | "error">("idle");
const updateMessage = ref<string | null>(null);
const actionError = ref<string | null>(null);

function preview(patch: SettingsPatch) { store.preview(patch, effects); }
function commit(patch: SettingsPatch) { void store.commit(patch, service, effects); }

async function selectDirectory(field: "downloadDirectory" | "temporaryDirectory", button: HTMLButtonElement) {
  if (!store.values) return;
  picking[field] = true;
  actionError.value = null;
  try {
    const selected = await picker.selectDirectory(store.values[field]);
    if (selected !== null) {
      const patch: SettingsPatch = field === "downloadDirectory"
        ? { field: "downloadDirectory", value: selected }
        : { field: "temporaryDirectory", value: selected };
      await store.commit(patch, service, effects);
    }
  } catch {
    actionError.value = t("settings.errors.directoryPicker");
  } finally {
    picking[field] = false;
    button.focus();
  }
}

async function checkForUpdates() {
  updateStatus.value = "checking";
  updateMessage.value = null;
  actionError.value = null;
  try {
    const result = await release.checkForUpdates();
    updateStatus.value = result.status;
    updateMessage.value = result.status === "latest"
      ? t("settings.about.latest")
      : t("settings.about.available", { version: result.latestVersion });
  } catch {
    updateStatus.value = "error";
    updateMessage.value = t("settings.about.updateFailed");
  }
}

async function installUpdate() {
  updateStatus.value = "installing";
  updateMessage.value = null;
  actionError.value = null;
  try {
    await release.installUpdate();
  } catch {
    updateStatus.value = "error";
    updateMessage.value = t("settings.about.installFailed");
  }
}

async function openLicenses() {
  actionError.value = null;
  try {
    await release.openLicenses();
  } catch {
    actionError.value = t("settings.about.licenseFailed");
  }
}

function handleTrayUpdateCheck() {
  void checkForUpdates();
}

onMounted(() => window.addEventListener("bilicatch:update-check", handleTrayUpdateCheck));
onBeforeUnmount(() => window.removeEventListener("bilicatch:update-check", handleTrayUpdateCheck));
</script>

<template>
  <div class="settings-page">
    <header class="settings-page__header"><h1 data-testid="page-heading">{{ t("pages.settings.title") }}</h1></header>
    <div v-if="actionError" class="settings-alert settings-alert--error" role="alert"><AlertTriangle :size="18" aria-hidden="true" /><span>{{ actionError }}</span></div>
    <div v-if="store.status === 'load_error'" class="settings-alert settings-alert--error" role="alert">
      <AlertTriangle :size="18" aria-hidden="true" /><span>{{ t("settings.errors.load") }}</span>
      <button data-testid="settings-retry" type="button" class="command-button" @click="store.retryInitialize(service, effects)">{{ t("actions.retry") }}</button>
    </div>
    <div v-else-if="store.status !== 'ready' || !store.values" class="settings-skeleton" aria-busy="true" :aria-label="t('settings.loading')">
      <div v-for="index in 12" :key="index" class="setting-skeleton"><span /><span /><span /></div>
    </div>
    <div v-else class="settings-form">
      <DownloadSettings :values="store.values" :field-states="store.fieldStates" :picking="picking" @preview="preview" @commit="commit" @pick="selectDirectory" />
      <AppearanceSettings :values="store.values" :field-states="store.fieldStates" @commit="commit" />
      <SystemSettings :values="store.values" :field-states="store.fieldStates" @commit="commit" />
      <AboutSettings :version="appStore.appInfo?.version ?? null" :update-status="updateStatus" :update-message="updateMessage" @check="checkForUpdates" @install="installUpdate" @licenses="openLicenses" />
    </div>
  </div>
</template>
