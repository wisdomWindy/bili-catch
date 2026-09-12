<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { AlertTriangle, LoaderCircle, LogIn, RefreshCw, Trash2 } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import ActionBar from "../features/download-center/components/ActionBar.vue";
import DownloadOptions from "../features/download-center/components/DownloadOptions.vue";
import InputPanel from "../features/download-center/components/InputPanel.vue";
import PartSelector from "../features/download-center/components/PartSelector.vue";
import VideoSummary from "../features/download-center/components/VideoSummary.vue";
import { useDownloadNotifier, useParseVideoService } from "../features/download-center/injection";
import { useDownloadCenterStore } from "../features/download-center/store";
import { useTaskDraftsStore } from "../stores/task-drafts";
import { useAuthStore } from "../features/authentication/store";

const store = useDownloadCenterStore();
const service = useParseVideoService();
const taskDrafts = useTaskDraftsStore();
const auth = useAuthStore();
const { t } = useI18n();
const router = useRouter();
const route = useRoute();
const notifier = useDownloadNotifier();

watch(() => auth.snapshot.status, async (status, previous) => {
  if (previous !== undefined && previous !== status) {
    await store.refreshForAuthChange(service, status === "authenticated");
  } else {
    store.reconcileAudioAvailability(status === "authenticated");
  }
  if (previous === "authenticated" && status === "anonymous") {
    notifier.info(t("download.sessionExpired"));
  }
  if (store.consumeAudioFallbackNotice()) notifier.info(t("download.audioFallback"));
  if (store.consumeVideoFallbackNotice()) notifier.info(t("download.videoFallback"));
}, { immediate: true });

const busy = computed(() => store.status === "parsing" || store.status === "enqueueing");
const errorText = computed(() => store.error ? t(`errors.${store.error.code}`) : "");

function submit(value?: string) {
  if (value !== undefined) store.setInput(value);
  void store.parse(service, value);
}

async function enqueue() {
  if (!store.canEnqueue || store.status === "enqueueing") return;
  const drafts = store.buildDrafts();
  if (drafts.length === 0) return;
  store.status = "enqueueing";
  taskDrafts.append(drafts);
  notifier.success(t("download.enqueued", { count: drafts.length }));
  await router.push("/tasks");
  store.status = "success";
}

function openLogin() {
  void router.push({ path: "/login", query: { from: route.fullPath } });
}

function selectMode(mode: typeof store.mode) {
  store.setMode(mode, auth.snapshot.status === "authenticated");
  if (store.consumeAudioFallbackNotice()) notifier.info(t("download.audioFallback"));
}

function selectAudioFormat(format: typeof store.audioFormat) {
  store.setAudioFormat(format, auth.snapshot.status === "authenticated");
  if (store.consumeAudioFallbackNotice()) notifier.info(t("download.audioFallback"));
}

onMounted(() => {
  if (import.meta.env.DEV && route.query.demo === "1" && route.query.autoparse === "1") {
    const input = route.query.error === "1" ? "av404" : "BV1xx411c7BF";
    submit(input);
  }
});
</script>

<template>
  <div class="download-workbench">
    <span class="sr-only" data-testid="page-heading">{{ t("pages.download.title") }}</span>
    <InputPanel
      :model-value="store.input"
      :busy="busy"
      :can-parse="store.canParse"
      :invalid="store.error?.code === 'E003'"
      @update:model-value="store.setInput"
      @submit="submit"
    />

    <div v-if="store.status === 'parsing'" class="parse-status" role="status" aria-live="polite">
      <LoaderCircle class="spin" :size="22" aria-hidden="true" />
      <div><strong>{{ t("download.parsingTitle") }}</strong><span>{{ t("download.parsingHint") }}</span></div>
    </div>

    <div v-else-if="store.status === 'failed'" class="parse-error" role="alert">
      <AlertTriangle :size="21" aria-hidden="true" />
      <div><strong>{{ errorText }}</strong></div>
      <div class="error-actions">
        <button class="secondary-button" type="button" @click="store.retry(service)">
          <RefreshCw :size="16" aria-hidden="true" />{{ t("actions.retry") }}
        </button>
        <button v-if="store.error?.code === 'E005'" class="secondary-button" type="button" @click="openLogin">
          <LogIn :size="16" aria-hidden="true" />{{ t("download.goLogin") }}
        </button>
        <button class="secondary-button" type="button" @click="store.clear">
          <Trash2 :size="16" aria-hidden="true" />{{ t("download.clear") }}
        </button>
      </div>
    </div>

    <template v-if="store.result && store.status === 'success'">
      <VideoSummary :result="store.result" />
      <div class="configuration-grid">
        <PartSelector
          :parts="store.result.parts"
          :selected-cids="store.selectedCids"
          @toggle="store.togglePart"
          @select-all="store.selectAll"
          @invert="store.invertSelection"
        />
        <DownloadOptions
          :mode="store.mode"
          :qualities="store.result.qualities"
          :codecs="store.result.codecs"
          :video-variants="store.result.videoVariants"
          :audio-formats="store.result.audioFormats"
          :audio-capability="store.result.audioCapability"
          :authenticated="auth.snapshot.status === 'authenticated'"
          :quality-id="store.qualityId"
          :codec="store.codec"
          :audio-format="store.audioFormat"
          :audio-bitrate-id="store.audioBitrateId"
          @mode="selectMode"
          @quality="store.setVideoQuality"
          @codec="store.setVideoCodec"
          @audio-format="selectAudioFormat"
          @audio-bitrate="store.setAudioProfile"
        />
      </div>
      <ActionBar
        :output-dir="store.outputDir"
        :can-enqueue="store.canEnqueue"
        :busy="busy"
        @clear="store.clear"
        @enqueue="enqueue"
      />
    </template>

    <div v-else-if="store.status === 'idle'" class="download-placeholder">
      <span>{{ t("download.idleTitle") }}</span>
      <p>{{ t("download.idleHint") }}</p>
    </div>
  </div>
</template>
