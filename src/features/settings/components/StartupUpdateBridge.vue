<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useReleaseActions } from "../injection";
import { useSettingsStore } from "../store";
import UpdateConfirmDialog from "./UpdateConfirmDialog.vue";

const settings = useSettingsStore();
const release = useReleaseActions();
const { t } = useI18n();
const show = ref(false);
const installing = ref(false);
const installError = ref<string | null>(null);
const currentVersion = ref("");
const latestVersion = ref("");
let checkStarted = false;
let disposed = false;

async function checkAtStartup() {
  try {
    const result = await release.checkForUpdates();
    if (disposed || result.status !== "available") return;
    currentVersion.value = result.currentVersion;
    latestVersion.value = result.latestVersion;
    show.value = true;
  } catch {
    // Startup checks stay silent so update infrastructure cannot block the app.
  }
}

function closeDialog() {
  if (installing.value) return;
  show.value = false;
}

async function installUpdate() {
  if (installing.value) return;
  installing.value = true;
  installError.value = null;
  try {
    await release.installUpdate();
    if (disposed) return;
    installing.value = false;
    show.value = false;
  } catch {
    if (disposed) return;
    installing.value = false;
    installError.value = t("settings.updateDialog.installFailed");
  }
}

watch(
  () => [settings.status, settings.values?.autoCheckUpdates] as const,
  ([status, enabled]) => {
    if (status !== "ready" || !enabled || checkStarted) return;
    checkStarted = true;
    void checkAtStartup();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  disposed = true;
});
</script>

<template>
  <UpdateConfirmDialog
    :show="show"
    :pending="installing"
    :current-version="currentVersion"
    :latest-version="latestVersion"
    :error-message="installError"
    @close="closeDialog"
    @confirm="installUpdate"
  />
</template>
