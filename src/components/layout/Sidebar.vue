<script setup lang="ts">
import { Clapperboard } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { primaryNavigation } from "../../app/navigation";
import { useAppStore } from "../../stores/app";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const appStore = useAppStore();

function titleKeyFor(name: string): string {
  return router.resolve({ name }).meta.titleKey;
}
</script>

<template>
  <aside class="sidebar" aria-label="BiliCatch">
    <RouterLink class="brand-link" data-testid="brand-link" :to="{ name: 'download' }" :title="t('brand.name')">
      <span class="brand-mark" aria-hidden="true"><Clapperboard :size="20" /></span>
      <span class="brand-copy">
        <strong>{{ t("brand.name") }}</strong>
        <small>{{ t("brand.tagline") }}</small>
      </span>
    </RouterLink>

    <nav class="primary-nav" data-testid="primary-nav" aria-label="Primary">
      <RouterLink
        v-for="item in primaryNavigation"
        :key="item.name"
        class="nav-link"
        :data-testid="`nav-${item.name}`"
        :to="{ name: item.name }"
        :title="t(titleKeyFor(item.name))"
        :aria-current="route.name === item.name ? 'page' : undefined"
      >
        <component :is="item.icon" class="nav-icon" :size="20" :stroke-width="1.8" aria-hidden="true" />
        <span class="nav-label">{{ t(titleKeyFor(item.name)) }}</span>
        <span class="nav-tail" aria-hidden="true" />
      </RouterLink>
    </nav>

    <footer class="sidebar-footer">
      <span class="version-label">{{ t("shell.version", { version: appStore.appInfo?.version ?? "…" }) }}</span>
      <span class="status-dot" :title="t('status.checking')" aria-hidden="true" />
    </footer>
  </aside>
</template>
