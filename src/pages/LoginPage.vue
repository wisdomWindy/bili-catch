<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import AuthAccountPanel from "../features/authentication/components/AuthAccountPanel.vue";
import LogoutConfirmDialog from "../features/authentication/components/LogoutConfirmDialog.vue";
import QrLoginPanel from "../features/authentication/components/QrLoginPanel.vue";
import { useAuthStore } from "../features/authentication/store";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const store = useAuthStore();
const logoutOpen = ref(false);
let autoStartedRevision: number | null = null;
let returnTimer: number | undefined;

const activeLogin = computed(() => ["requesting", "waiting_scan", "waiting_confirm"].includes(store.snapshot.status));
const displayStatus = computed(() => store.error && store.snapshot.status === "anonymous" ? "error" : store.snapshot.status);
const authError = computed(() => store.error ?? store.snapshot.error);
const errorMessage = computed(() => authError.value ? t(`errors.${authError.value.code}`) : null);
const accountName = computed(() => store.snapshot.account?.name.trim() || t("auth.accountFallback"));
const source = typeof route.query.from === "string" ? router.resolve(route.query.from) : null;
const returnTarget = source && ["download", "tasks", "settings"].includes(String(source.name))
  ? source.fullPath
  : "/download";

watch(() => [store.snapshot.status, store.snapshot.revision] as const, ([status, revision]) => {
  if (status === "anonymous" && autoStartedRevision !== revision && !store.pending.start) {
    autoStartedRevision = revision;
    void store.start();
  }
  if (returnTimer !== undefined) window.clearTimeout(returnTimer);
  const holdDemo = import.meta.env.DEV && route.query.hold === "1";
  if (status === "authenticated" && !holdDemo) {
    returnTimer = window.setTimeout(() => { void router.replace(returnTarget); }, 800);
  }
}, { immediate: true });

async function confirmLogout() {
  await store.logout();
  if (store.snapshot.status !== "authenticated") logoutOpen.value = false;
}

onBeforeUnmount(() => {
  if (returnTimer !== undefined) window.clearTimeout(returnTimer);
  if (activeLogin.value) void store.cancel();
});
</script>

<template>
  <div class="auth-page">
    <header class="auth-intro">
      <span class="auth-product">BiliCatch</span>
      <h2 data-testid="page-heading">{{ t("pages.login.title") }}</h2>
    </header>

    <AuthAccountPanel
      v-if="store.snapshot.status === 'authenticated' && store.snapshot.account"
      :account="store.snapshot.account"
      :pending="store.pending.logout"
      @logout="logoutOpen = true"
    />
    <QrLoginPanel
      v-else
      :status="displayStatus"
      :qr-content="store.snapshot.qrContent"
      :expires-at="store.snapshot.expiresAt"
      :pending="store.pending.start"
      :error-message="errorMessage"
      @refresh="store.start"
    />

    <LogoutConfirmDialog
      :show="logoutOpen"
      :pending="store.pending.logout"
      :account-name="accountName"
      :error-message="errorMessage"
      @close="logoutOpen = false"
      @confirm="confirmLogout"
    />
  </div>
</template>
