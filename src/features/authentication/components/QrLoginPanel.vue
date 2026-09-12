<script setup lang="ts">
import { RefreshCw, ShieldCheck } from "@lucide/vue";
import { toCanvas } from "qrcode";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AuthStatus } from "../contracts";

const props = defineProps<{
  status: AuthStatus;
  qrContent: string | null;
  expiresAt: string | null;
  pending: boolean;
  errorMessage: string | null;
}>();
const emit = defineEmits<{ refresh: [] }>();
const { t } = useI18n();
const canvas = ref<HTMLCanvasElement | null>(null);
const renderFailed = ref(false);
const now = ref(Date.now());
let timer: number | undefined;
let renderGeneration = 0;

const isWaiting = computed(() => props.status === "waiting_scan" || props.status === "waiting_confirm");
const isLoading = computed(() => ["restoring", "anonymous", "requesting"].includes(props.status));
const canRefresh = computed(() => ["expired", "cancelled", "error"].includes(props.status) || renderFailed.value);
const statusText = computed(() => renderFailed.value ? t("auth.qrRenderError") : t(`auth.status.${props.status}`));
const remainingSeconds = computed(() => {
  if (!props.expiresAt) return 0;
  const seconds = Math.ceil((Date.parse(props.expiresAt) - now.value) / 1000);
  return Math.max(0, Math.min(180, Number.isFinite(seconds) ? seconds : 0));
});
const remaining = computed(() => {
  const minutes = Math.floor(remainingSeconds.value / 60).toString().padStart(2, "0");
  const seconds = (remainingSeconds.value % 60).toString().padStart(2, "0");
  return `${minutes}:${seconds}`;
});

function isSafeQrContent(value: string): boolean {
  if (value.length === 0 || value.length > 2048) return false;
  try {
    const url = new URL(value);
    return url.protocol === "https:"
      && (url.hostname === "bilibili.com" || url.hostname.endsWith(".bilibili.com"));
  } catch {
    return false;
  }
}

async function renderQr() {
  const generation = ++renderGeneration;
  renderFailed.value = false;
  if (!isWaiting.value || !props.qrContent) return;
  await nextTick();
  if (generation !== renderGeneration) return;
  if (!canvas.value || !isSafeQrContent(props.qrContent)) {
    renderFailed.value = true;
    return;
  }
  try {
    await toCanvas(canvas.value, props.qrContent, {
      width: 224,
      margin: 2,
      errorCorrectionLevel: "M",
      color: { dark: "#161719", light: "#ffffff" },
    });
  } catch {
    if (generation === renderGeneration) renderFailed.value = true;
  }
}

watch(() => [props.status, props.qrContent] as const, () => { void renderQr(); }, { immediate: true, flush: "post" });
onMounted(() => { timer = window.setInterval(() => { now.value = Date.now(); }, 1000); });
onBeforeUnmount(() => {
  renderGeneration += 1;
  if (timer !== undefined) window.clearInterval(timer);
});
</script>

<template>
  <section class="auth-tool" aria-labelledby="qr-panel-title">
    <div class="auth-tool-heading">
      <ShieldCheck :size="20" aria-hidden="true" />
      <h3 id="qr-panel-title">{{ t("auth.qrTitle") }}</h3>
    </div>
    <div class="qr-stage" :class="{ 'qr-stage--confirm': status === 'waiting_confirm' }">
      <div v-if="isLoading" class="qr-skeleton" data-testid="qr-skeleton" aria-hidden="true" />
      <div v-else-if="renderFailed" class="qr-unavailable" data-testid="qr-render-error" aria-hidden="true">
        <RefreshCw :size="34" />
      </div>
      <canvas v-else-if="isWaiting" ref="canvas" width="224" height="224" aria-hidden="true" />
      <div v-else class="qr-unavailable" aria-hidden="true"><RefreshCw :size="34" /></div>
    </div>
    <div class="auth-status" aria-live="polite">
      <strong>{{ statusText }}</strong>
      <span v-if="status === 'waiting_scan'">{{ t("auth.scanHint") }}</span>
      <span v-else-if="status === 'waiting_confirm'">{{ t("auth.confirmHint") }}</span>
      <span v-else-if="errorMessage">{{ errorMessage }}</span>
    </div>
    <span v-if="isWaiting" class="qr-countdown" aria-hidden="true">{{ remaining }}</span>
    <button
      v-if="canRefresh"
      class="primary-button auth-refresh"
      data-testid="refresh-qr"
      type="button"
      :disabled="pending"
      @click="emit('refresh')"
    >
      <RefreshCw :class="{ spin: pending }" :size="17" aria-hidden="true" />
      {{ status === 'error' || renderFailed ? t("auth.retry") : t("auth.refresh") }}
    </button>
  </section>
</template>
