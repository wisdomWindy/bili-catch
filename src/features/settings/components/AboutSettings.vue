<script setup lang="ts">
import { Download, ExternalLink, LoaderCircle, RefreshCw } from "@lucide/vue";
import SettingSection from "./SettingSection.vue";

defineProps<{ version: string | null; updateStatus: "idle" | "checking" | "installing" | "latest" | "available" | "error"; updateMessage: string | null }>();
defineEmits<{ check: []; install: []; licenses: [] }>();
</script>

<template>
  <SettingSection id="about-settings" :title="$t('settings.sections.about')">
    <div class="setting-row about-row" data-about-row><div class="setting-row__label">{{ $t("settings.about.currentVersion") }}</div><div class="setting-row__control version-value">{{ version ?? $t("settings.about.versionUnavailable") }}</div><span class="field-save-status" /></div>
    <div class="setting-row about-row" data-about-row>
      <div class="setting-row__label">{{ $t("settings.about.checkUpdates") }}</div>
      <div class="setting-row__control about-action">
        <button data-testid="check-updates" type="button" class="command-button" :disabled="updateStatus === 'checking' || updateStatus === 'installing'" @click="$emit('check')"><LoaderCircle v-if="updateStatus === 'checking' || updateStatus === 'installing'" :size="17" class="spin" aria-hidden="true" /><RefreshCw v-else :size="17" aria-hidden="true" /><span>{{ updateStatus === "checking" ? $t("settings.about.checking") : updateStatus === "installing" ? $t("settings.about.installing") : $t("settings.about.checkUpdates") }}</span></button>
        <button v-if="updateStatus === 'available'" data-testid="install-update" type="button" class="command-button command-button--primary" @click="$emit('install')"><Download :size="17" aria-hidden="true" /><span>{{ $t("settings.about.installUpdate") }}</span></button>
        <span v-if="updateMessage" class="about-result" :class="`about-result--${updateStatus}`" aria-live="polite">{{ updateMessage }}</span>
      </div><span class="field-save-status" />
    </div>
    <div class="setting-row about-row" data-about-row><div class="setting-row__label">{{ $t("settings.about.openLicenses") }}</div><div class="setting-row__control"><button data-testid="open-licenses" type="button" class="command-button command-button--quiet" :aria-label="$t('settings.about.openLicenses')" @click="$emit('licenses')"><ExternalLink :size="17" aria-hidden="true" /><span>{{ $t("settings.about.openLicenses") }}</span></button></div><span class="field-save-status" /></div>
  </SettingSection>
</template>
