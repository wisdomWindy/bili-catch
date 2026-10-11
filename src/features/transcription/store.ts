import { defineStore } from "pinia";
import type { AppError } from "../../contracts/app-error";
import { normalizeIpcError } from "../../contracts/app-error";
import type {
  AsrModel,
  AsrModelId,
  CreateTranscribeTaskRequest,
  ExportTranscriptRequest,
  MediaSourceInfo,
  ModelDownloadProgressEvent,
  SaveTranscriptEditsRequest,
  TranscribeTask,
  TranscribeTaskAction,
  TranscriptResult,
  TranscriptionProgressEvent,
  TranscriptionRemovedEvent,
  TranscriptionSource,
} from "./contracts";
import type { TranscriptionEventSource } from "./events";
import type { TranscriptionService } from "./service";

type BufferedEvent =
  | { kind: "progress"; value: TranscriptionProgressEvent }
  | { kind: "removed"; value: TranscriptionRemovedEvent }
  | { kind: "model"; value: ModelDownloadProgressEvent };

interface TranscriptionInitialization {
  service: TranscriptionService;
  eventSource: TranscriptionEventSource;
  promise: Promise<void>;
}

const transcriptionInitializations = new WeakMap<object, TranscriptionInitialization>();

export const useTranscriptionStore = defineStore("transcription", {
  state: () => ({
    byId: {} as Record<string, TranscribeTask>,
    modelsById: {} as Partial<Record<AsrModelId, AsrModel>>,
    resultsByTaskId: {} as Record<string, TranscriptResult>,
    sequence: 0,
    modelSequence: 0,
    status: "idle" as "idle" | "loading" | "ready" | "failed",
    error: null as AppError | null,
    pending: {} as Record<string, boolean>,
    service: null as TranscriptionService | null,
    unsubscribe: null as (() => void) | null,
    destroyed: false,
  }),
  getters: {
    tasks: (state): TranscribeTask[] => Object.values(state.byId).sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)),
    models: (state): AsrModel[] => Object.values(state.modelsById).filter((value): value is AsrModel => value !== undefined),
  },
  actions: {
    initialize(service: TranscriptionService, events: TranscriptionEventSource): Promise<void> {
      const current = transcriptionInitializations.get(this);
      const reusable = current?.service === service
        && current.eventSource === events
        && !this.destroyed
        && (this.status === "loading" || (this.status === "ready" && Boolean(this.unsubscribe)));
      if (reusable) return current.promise;

      const initialization = {} as TranscriptionInitialization;
      const promise = (async () => {
        this.dispose();
        this.destroyed = false;
        this.service = service;
        this.status = "loading";
        this.error = null;
        const buffered: BufferedEvent[] = [];
        let hydrating = true;
        try {
          const unsubscribe = await events.subscribe(
            (value) => hydrating ? buffered.push({ kind: "progress", value }) : this.applyProgress(value),
            (value) => hydrating ? buffered.push({ kind: "removed", value }) : this.applyRemoved(value),
            (value) => hydrating ? buffered.push({ kind: "model", value }) : this.applyModel(value),
          );
          if (this.destroyed || transcriptionInitializations.get(this) !== initialization) {
            unsubscribe();
            return;
          }
          this.unsubscribe = unsubscribe;
          const [tasks, models] = await Promise.all([service.listTasks(), service.listModels()]);
          if (this.destroyed || transcriptionInitializations.get(this) !== initialization) return;
          this.byId = Object.fromEntries(tasks.tasks.map((task) => [task.id, task]));
          this.modelsById = Object.fromEntries(models.models.map((model) => [model.id, model]));
          this.sequence = tasks.sequence;
          this.modelSequence = models.sequence;
          hydrating = false;
          for (const event of buffered) {
            if (event.kind === "progress") this.applyProgress(event.value);
            else if (event.kind === "removed") this.applyRemoved(event.value);
            else this.applyModel(event.value);
          }
          this.status = "ready";
        } catch (error: unknown) {
          hydrating = false;
          if (!this.destroyed && transcriptionInitializations.get(this) === initialization) {
            this.error = normalizeIpcError(error);
            this.status = "failed";
          }
        }
      })();
      Object.assign(initialization, { service, eventSource: events, promise });
      transcriptionInitializations.set(this, initialization);
      return promise;
    },
    applyProgress(event: TranscriptionProgressEvent) {
      if (this.destroyed || event.sequence <= this.sequence) return;
      const current = this.byId[event.task.id];
      this.sequence = event.sequence;
      if (current && event.task.revision < current.revision) return;
      this.byId[event.task.id] = event.task;
    },
    applyRemoved(event: TranscriptionRemovedEvent) {
      if (this.destroyed || event.sequence <= this.sequence) return;
      delete this.byId[event.taskId];
      delete this.resultsByTaskId[event.taskId];
      this.sequence = event.sequence;
    },
    applyModel(event: ModelDownloadProgressEvent) {
      if (this.destroyed || event.sequence <= this.modelSequence) return;
      this.modelSequence = event.sequence;
      this.mergeModel(event.model);
    },
    mergeModel(model: AsrModel) {
      const current = this.modelsById[model.id];
      if (current && model.revision < current.revision) return;
      this.modelsById[model.id] = model;
    },
    async inspectSource(source: TranscriptionSource): Promise<MediaSourceInfo | null> {
      if (!this.service) return null;
      try { return await this.service.inspectSource(source); }
      catch (error: unknown) { this.error = normalizeIpcError(error); return null; }
    },
    async create(request: CreateTranscribeTaskRequest): Promise<TranscribeTask | null> {
      if (!this.service || this.pending.create) return null;
      this.pending.create = true;
      try {
        const result = await this.service.createTask(request);
        this.byId[result.task.id] = result.task;
        return result.task;
      } catch (error: unknown) { this.error = normalizeIpcError(error); return null; }
      finally { delete this.pending.create; }
    },
    async control(taskId: string, action: TranscribeTaskAction) {
      if (!this.service || this.pending[taskId]) return;
      this.pending[taskId] = true;
      try {
        const task = await this.service.controlTask({ taskId, action });
        if (action === "delete") delete this.byId[taskId];
        else this.byId[task.id] = task;
      } catch (error: unknown) { this.error = normalizeIpcError(error); }
      finally { delete this.pending[taskId]; }
    },
    async loadResult(taskId: string): Promise<TranscriptResult | null> {
      if (!this.service) return null;
      try {
        const result = await this.service.getResult(taskId);
        this.resultsByTaskId[taskId] = result;
        return result;
      } catch (error: unknown) { this.error = normalizeIpcError(error); return null; }
    },
    async saveEdits(request: SaveTranscriptEditsRequest): Promise<TranscriptResult | null> {
      if (!this.service) return null;
      try {
        const result = await this.service.saveEdits(request);
        this.resultsByTaskId[request.taskId] = result;
        return result;
      } catch (error: unknown) { this.error = normalizeIpcError(error); return null; }
    },
    async exportResult(request: ExportTranscriptRequest): Promise<string | null> {
      if (!this.service) return null;
      try { return (await this.service.exportResult(request)).outputPath; }
      catch (error: unknown) { this.error = normalizeIpcError(error); return null; }
    },
    async openOutput(taskId: string) {
      if (!this.service) return;
      try { await this.service.openOutput(taskId); }
      catch (error: unknown) { this.error = normalizeIpcError(error); }
    },
    async clearCompleted() {
      if (!this.service || this.pending.clearCompleted) return;
      this.pending.clearCompleted = true;
      try {
        const removed = await this.service.clearCompleted();
        removed.forEach((id) => {
          delete this.byId[id];
          delete this.resultsByTaskId[id];
        });
      } catch (error: unknown) { this.error = normalizeIpcError(error); }
      finally { delete this.pending.clearCompleted; }
    },
    async downloadModel(modelId: AsrModelId) {
      if (!this.service) return;
      const current = this.modelsById[modelId];
      if (!current || ["queued", "downloading", "verifying"].includes(current.downloadState)) return;
      const startingRevision = current.revision;
      this.modelsById[modelId] = {
        ...current,
        downloadState: "queued",
        downloadedBytes: 0,
        error: undefined,
      };
      try { this.mergeModel(await this.service.downloadModel(modelId)); }
      catch (error: unknown) {
        const normalized = normalizeIpcError(error);
        this.error = normalized;
        const latest = this.modelsById[modelId];
        if (latest && latest.revision <= startingRevision) {
          this.modelsById[modelId] = { ...latest, downloadState: "failed", error: normalized };
        }
      }
    },
    async cancelModelDownload(modelId: AsrModelId) {
      if (!this.service) return;
      try { this.mergeModel(await this.service.cancelModelDownload(modelId)); }
      catch (error: unknown) { this.error = normalizeIpcError(error); }
    },
    async deleteModel(modelId: AsrModelId) {
      if (!this.service) return;
      try {
        const snapshot = await this.service.deleteModel(modelId);
        this.modelsById = Object.fromEntries(snapshot.models.map((model) => [model.id, model]));
        this.modelSequence = Math.max(this.modelSequence, snapshot.sequence);
      } catch (error: unknown) { this.error = normalizeIpcError(error); }
    },
    dispose() {
      this.destroyed = true;
      this.unsubscribe?.();
      this.unsubscribe = null;
      transcriptionInitializations.delete(this);
    },
  },
});
