<script setup lang="ts">
import { computed, inject, onMounted, reactive, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { Clipboard, Cpu, Play, RefreshCw, Settings2, XCircle } from "@lucide/vue";
import { useMessage } from "naive-ui";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import ModelManager from "../features/transcription/components/ModelManager.vue";
import TranscriptEditor from "../features/transcription/components/TranscriptEditor.vue";
import TranscriptionSourcePanel from "../features/transcription/components/TranscriptionSourcePanel.vue";
import type { AsrModelId, MediaSourceInfo, TranscribeTask, TranscriptFormat, TranscriptionOptions, TranscriptionSource } from "../features/transcription/contracts";
import { transcriptionEventSourceKey, transcriptionServiceKey } from "../features/transcription/injection";
import { useTranscriptionStore } from "../features/transcription/store";
import { useSettingsStore } from "../features/settings/store";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import { useTaskManagementStore } from "../features/task-management/store";

const service = inject(transcriptionServiceKey, null);
const events = inject(transcriptionEventSourceKey, null);
const store = useTranscriptionStore();
const settings = useSettingsStore();
const route = useRoute();
const { t } = useI18n();
const message = useMessage();
const taskService = inject(taskServiceKey, null);
const taskEvents = inject(taskEventSourceKey, null);
const downloadTasks = useTaskManagementStore();
const sourcePath = ref("");
const sourceInfo = ref<MediaSourceInfo | null>(null);
const selectedSource = ref<TranscriptionSource | null>(null);
const inspecting = ref(false);
const showModels = ref(false);
const selectedTaskId = ref<string | null>(null);
const saving = ref(false);
const form = reactive<TranscriptionOptions>({
  modelId: "base" as AsrModelId,
  language: "zh",
  outputFormat: "srt",
  timestamps: true,
  punctuation: true,
  gpuMode: "auto",
  outputDirectory: "Transcribe",
});

const selectedModel = computed(() => store.modelsById[form.modelId]);
const selectedTask = computed(() => selectedTaskId.value ? store.byId[selectedTaskId.value] : null);
const selectedResult = computed(() => selectedTaskId.value ? store.resultsByTaskId[selectedTaskId.value] : null);
const canStart = computed(() => Boolean(sourceInfo.value && selectedModel.value?.valid && form.outputDirectory.trim() && !store.pending.create));
const activeTasks = computed(() => store.tasks.filter((task) => !["completed", "failed", "cancelled"].includes(task.status)));

watch(() => settings.values, (values) => {
  if (!values) return;
  form.modelId = values.defaultAsrModel;
  form.language = values.defaultTranscriptionLanguage;
  form.outputFormat = values.defaultTranscriptFormat;
  form.timestamps = values.transcriptionTimestamps;
  form.punctuation = values.transcriptionPunctuation;
  form.gpuMode = values.asrGpuMode;
  form.outputDirectory = `${values.downloadDirectory}/Transcribe`;
}, { immediate: true });

watch(selectedTask, (task) => {
  if (task?.status === "completed" && !selectedResult.value) void store.loadResult(task.id);
});

onMounted(async () => {
  const initializers: Promise<unknown>[] = [];
  if (service && events) initializers.push(store.initialize(service, events));
  if (taskService && taskEvents) initializers.push(downloadTasks.initialize(taskService, taskEvents));
  await Promise.all(initializers);
  const taskId = typeof route.query.taskId === "string" ? route.query.taskId.trim() : "";
  if (!taskId) return;
  if (store.byId[taskId]) {
    selectedTaskId.value = taskId;
    if (route.query.action === "retranscribe") await retranscribeTask(store.byId[taskId]);
    return;
  }
  const downloadTask = downloadTasks.byId[taskId];
  if (downloadTask?.status === "completed" && downloadTask.outputPath && downloadTask.mode !== "video-only") {
    selectedSource.value = {
      kind: "downloadTask",
      taskId: downloadTask.id,
      outputPath: downloadTask.outputPath,
      bvid: downloadTask.bvid,
      cid: downloadTask.cid,
    };
    sourcePath.value = downloadTask.outputPath;
    inspecting.value = true;
    sourceInfo.value = await store.inspectSource(selectedSource.value);
    inspecting.value = false;
  } else {
    message.error(t("transcribe.errors.downloadSourceUnavailable"));
  }
});
async function browse() {
  const path = await open({ multiple: false, directory: false, filters: [{ name: "Media", extensions: ["mp3", "m4a", "aac", "flac", "wav", "ogg", "mp4", "mkv", "flv", "mov", "webm"] }] });
  if (typeof path === "string") { sourcePath.value = path; selectedSource.value = { kind: "localFile", path }; await inspect(); }
}

async function inspect() {
  if (!sourcePath.value.trim()) return;
  inspecting.value = true;
  selectedSource.value = { kind: "localFile", path: sourcePath.value.trim() };
  sourceInfo.value = await store.inspectSource(selectedSource.value);
  inspecting.value = false;
}

function handleDrop(event: DragEvent) {
  const files = Array.from(event.dataTransfer?.files ?? []);
  if (files.length !== 1) { message.warning(t("transcribe.errors.oneFileOnly")); return; }
  const path = (files[0] as File & { path?: string }).path;
  if (!path) { message.error(t("transcribe.errors.filePathUnavailable")); return; }
  sourcePath.value = path;
  selectedSource.value = { kind: "localFile", path };
  void inspect();
}

async function start() {
  if (!sourceInfo.value || !canStart.value) return;
  const source: TranscriptionSource = selectedSource.value ?? { kind: "localFile", path: sourceInfo.value.path };
  const task = await store.create({ source, options: { ...form }, force: false });
  if (task) { selectedTaskId.value = task.id; message.success(t("transcribe.messages.queued")); }
}

function selectTask(task: TranscribeTask) {
  selectedTaskId.value = task.id;
  if (task.status === "completed") void store.loadResult(task.id);
}

async function saveEdits(texts: string[]) {
  if (!selectedResult.value) return;
  saving.value = true;
  const result = await store.saveEdits({ taskId: selectedResult.value.taskId, expectedRevision: selectedResult.value.revision, segments: texts });
  saving.value = false;
  if (result) message.success(t("transcribe.messages.saved"));
}

async function copyResult() {
  if (!selectedResult.value) return;
  await navigator.clipboard.writeText(selectedResult.value.plainText);
  message.success(t("transcribe.messages.copied"));
}

async function exportResult(format: TranscriptFormat) {
  if (!selectedResult.value) return;
  const timestamps = form.timestamps || ["srt", "vtt", "json"].includes(format);
  const output = await store.exportResult({
    taskId: selectedResult.value.taskId,
    resultRevision: selectedResult.value.revision,
    format,
    timestamps,
    outputDirectory: form.outputDirectory,
  });
  if (output) message.success(t("transcribe.messages.exported"));
}

async function retranscribeTask(task = selectedTask.value) {
  if (!task) return;
  const created = await store.create({ source: task.source, options: task.options, force: true });
  if (created) {
    selectedTaskId.value = created.id;
    message.success(t("transcribe.messages.queued"));
  }
}
</script>

<template>
  <main class="transcribe-page">
    <header class="transcribe-heading">
      <div><h1 data-testid="page-heading">{{ t("pages.transcribe.title") }}</h1><p>{{ t("transcribe.subtitle") }}</p></div>
      <button class="secondary-button" type="button" @click="showModels = true"><Cpu :size="17" />{{ t("transcribe.models.manage") }}</button>
    </header>

    <TranscriptionSourcePanel v-model:path="sourcePath" :info="sourceInfo" :inspecting="inspecting"
      @browse="browse" @inspect="inspect" @drop="handleDrop" />

    <div class="transcribe-grid">
      <section class="transcribe-section options-panel">
        <header><Settings2 :size="19" /><h2>{{ t("transcribe.options.title") }}</h2></header>
        <div class="option-grid">
          <label>{{ t("transcribe.options.model") }}<select v-model="form.modelId"><option v-for="model in store.models" :key="model.id" :value="model.id">{{ model.displayName }}{{ model.valid ? "" : ` (${t('transcribe.models.notInstalled')})` }}</option></select></label>
          <label>{{ t("transcribe.options.language") }}<select v-model="form.language"><option value="auto">{{ t("settings.options.transcriptionLanguage.auto") }}</option><option value="zh">{{ t("settings.options.transcriptionLanguage.zh") }}</option><option value="zh-TW">{{ t("settings.options.transcriptionLanguage.zhTW") }}</option><option value="en">{{ t("settings.options.transcriptionLanguage.en") }}</option><option value="ja">{{ t("settings.options.transcriptionLanguage.ja") }}</option><option value="ko">{{ t("settings.options.transcriptionLanguage.ko") }}</option></select></label>
          <label>{{ t("transcribe.options.format") }}<select v-model="form.outputFormat"><option value="txt">TXT</option><option value="srt">SRT</option><option value="vtt">VTT</option><option value="json">JSON</option><option value="markdown">Markdown</option></select></label>
          <label>{{ t("transcribe.options.gpu") }}<select v-model="form.gpuMode"><option value="auto">{{ t("settings.options.gpuMode.auto") }}</option><option value="enabled">{{ t("settings.options.gpuMode.enabled") }}</option><option value="disabled">{{ t("settings.options.gpuMode.disabled") }}</option></select></label>
          <label class="option-wide">{{ t("transcribe.options.outputDirectory") }}<input v-model="form.outputDirectory" /></label>
          <label class="check-option"><input v-model="form.timestamps" type="checkbox" :disabled="['srt','vtt','json'].includes(form.outputFormat)" />{{ t("transcribe.options.timestamps") }}</label>
          <label class="check-option"><input v-model="form.punctuation" type="checkbox" />{{ t("transcribe.options.punctuation") }}</label>
        </div>
        <div v-if="selectedModel && !selectedModel.valid" class="inline-warning">{{ t("transcribe.models.installRequired") }}</div>
        <button class="primary-button start-transcription" type="button" :disabled="!canStart" @click="start"><Play :size="17" />{{ t("transcribe.actions.start") }}</button>
      </section>

      <section class="transcribe-section run-panel">
        <header><RefreshCw :size="19" /><h2>{{ t("transcribe.tasks.title") }}</h2><span>{{ t("transcribe.tasks.active", { count: activeTasks.length }) }}</span></header>
        <div v-if="!store.tasks.length" class="compact-empty">{{ t("transcribe.tasks.empty") }}</div>
        <div v-for="task in store.tasks" :key="task.id" class="transcribe-task-row" :class="{ selected: selectedTaskId === task.id }"
          role="button" tabindex="0" @click="selectTask(task)" @keydown.enter="selectTask(task)" @keydown.space.prevent="selectTask(task)">
          <span><strong>{{ task.sourceInfo.fileName }}</strong><small>{{ t(`tasks.transcriptionStatuses.${task.status}`) }}</small></span>
          <progress :value="task.progress.progress" max="1" />
          <button v-if="!['completed','failed','cancelled','saving'].includes(task.status)" class="icon-button" type="button" :title="t('tasks.actions.cancel')" @click.stop="store.control(task.id, 'cancel')"><XCircle :size="17" /></button>
          <button v-else-if="['failed','cancelled'].includes(task.status)" class="icon-button" type="button" :title="t('tasks.actions.retry')" @click.stop="store.control(task.id, 'retry')"><RefreshCw :size="17" /></button>
        </div>
      </section>
    </div>

    <TranscriptEditor v-if="selectedResult" :result="selectedResult" :saving="saving" @save="saveEdits" @copy="copyResult"
      @open="service && selectedTaskId && service.openOutput(selectedTaskId)" @export="exportResult" @retranscribe="retranscribeTask()" />
    <section v-else class="transcribe-section result-placeholder"><Clipboard :size="22" /><span>{{ t("transcribe.result.placeholder") }}</span></section>

    <ModelManager :show="showModels" :models="store.models" @close="showModels = false"
      @download="store.downloadModel" @cancel="store.cancelModelDownload" @delete="store.deleteModel" />
  </main>
</template>
