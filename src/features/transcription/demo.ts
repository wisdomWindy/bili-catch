import type {
  AsrModel,
  AsrModelId,
  CreateTranscribeTaskRequest,
  TranscribeTask,
  TranscriptResult,
} from "./contracts";
import type { TranscriptionEventSource } from "./events";
import type { TranscriptionService } from "./service";

const modelIds: AsrModelId[] = ["tiny", "base", "small", "medium", "large-v3"];

export function createDemoTranscriptionRuntime(): { service: TranscriptionService; events: TranscriptionEventSource } {
  let sequence = 1;
  let tasks: TranscribeTask[] = [];
  const results = new Map<string, TranscriptResult>();
  const models: AsrModel[] = modelIds.map((id, index) => ({
    id,
    displayName: id === "large-v3" ? "Large v3" : id[0].toUpperCase() + id.slice(1),
    fileName: `ggml-${id}.bin`,
    sizeBytes: [77_691_713, 147_951_465, 487_601_967, 1_533_763_059, 3_095_033_483][index],
    installed: id === "base",
    valid: id === "base",
    isDefault: id === "base",
    inUse: false,
    path: id === "base" ? "D:/BiliCatch/models/ggml-base.bin" : undefined,
    recommendedFor: id === "base" ? "Recommended default" : "Optional accuracy and speed profile",
    capabilities: ["multilingual", "timestamps"],
    downloadState: id === "base" ? "completed" : "idle",
    downloadedBytes: id === "base" ? 147_951_465 : 0,
    revision: 1,
  }));

  const service: TranscriptionService = {
    inspectSource: async (source) => ({
      fileName: source.kind === "localFile" ? source.path.split(/[\\/]/).pop() ?? "demo.wav" : "Downloaded demo.mp4",
      path: source.kind === "localFile" ? source.path : source.outputPath,
      durationSeconds: 94,
      sizeBytes: 8_388_608,
      mediaKind: "audio",
      format: "wav",
    }),
    listTasks: async () => ({ sequence, tasks: [...tasks] }),
    createTask: async (request: CreateTranscribeTaskRequest) => {
      const now = new Date().toISOString();
      const id = `demo-transcription-${++sequence}`;
      const task: TranscribeTask = {
        id,
        revision: 1,
        source: request.source,
        sourceInfo: await service.inspectSource(request.source),
        options: request.options,
        status: "completed",
        progress: { progress: 1, processedSeconds: 94, totalSeconds: 94, speed: 2.1, etaSeconds: 0, message: "Completed" },
        resultId: `result-${id}`,
        outputPath: `${request.options.outputDirectory}/demo.en.srt`,
        cacheReused: false,
        createdAt: now,
        updatedAt: now,
      };
      const result: TranscriptResult = {
        id: `result-${id}`,
        taskId: id,
        revision: 1,
        sourceFingerprint: "demo",
        modelId: request.options.modelId,
        requestedLanguage: request.options.language,
        detectedLanguage: "en",
        segments: [{ index: 0, startMs: 0, endMs: 3500, text: "BiliCatch keeps transcription on this device." }],
        plainText: "BiliCatch keeps transcription on this device.",
        createdAt: now,
        updatedAt: now,
      };
      tasks = [task, ...tasks];
      results.set(id, result);
      return { task, reused: false };
    },
    controlTask: async ({ taskId, action }) => {
      const task = tasks.find((item) => item.id === taskId);
      if (!task) throw { code: "E_INTERNAL", message: "Task not found" };
      if (action === "delete") tasks = tasks.filter((item) => item.id !== taskId);
      else task.status = action === "retry" ? "pending" : "cancelled";
      return { ...task };
    },
    clearCompleted: async () => { const ids = tasks.filter((task) => task.status === "completed").map((task) => task.id); tasks = tasks.filter((task) => task.status !== "completed"); return ids; },
    listModels: async () => ({ sequence: 1, models: [...models] }),
    downloadModel: async (modelId) => ({ ...models.find((model) => model.id === modelId)!, downloadState: "downloading" }),
    cancelModelDownload: async (modelId) => ({ ...models.find((model) => model.id === modelId)!, downloadState: "cancelled" }),
    verifyModel: async (modelId) => models.find((model) => model.id === modelId)!,
    deleteModel: async () => ({ sequence: 2, models: [...models] }),
    getResult: async (taskId) => results.get(taskId) ?? Promise.reject({ code: "E_INTERNAL", message: "Result not found" }),
    saveEdits: async (request) => {
      const result = results.get(request.taskId);
      if (!result) throw { code: "E_INTERNAL", message: "Result not found" };
      return { ...result, revision: result.revision + 1, segments: result.segments.map((segment, index) => ({ ...segment, text: request.segments[index] ?? segment.text })) };
    },
    exportResult: async () => ({ outputPath: "D:/BiliCatch/Transcribe/demo.en.srt" }),
    openOutput: async () => undefined,
  };
  return { service, events: { subscribe: async () => () => undefined } };
}
