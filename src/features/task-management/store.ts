import { defineStore } from "pinia";
import type { AppError } from "../../contracts/app-error";
import { normalizeIpcError } from "../../contracts/app-error";
import type { TaskEventSource } from "../../services/ipc/events";
import type { DownloadTaskDraft } from "../download-center/contracts";
import { compareTasks } from "./formatters";
import type {
  DownloadTask,
  TaskAction,
  TaskProgressEvent,
  TaskRemovedEvent,
} from "./contracts";
import type { TaskService } from "./service";

export type TaskFilter = "all" | "active" | "completed" | "failed";
type LoadStatus = "idle" | "subscribing" | "loading" | "ready" | "failed";
type BufferedEvent = { kind: "progress"; value: TaskProgressEvent } | { kind: "removed"; value: TaskRemovedEvent };

interface HandoffSource {
  items: DownloadTaskDraft[];
  handoffId: string | null;
  clearIfMatches(id: string): void;
}

interface TaskState {
  byId: Record<string, DownloadTask>;
  sequence: number;
  capacity: number;
  filter: TaskFilter;
  status: LoadStatus;
  error: AppError | null;
  eventError: AppError | null;
  pending: Record<string, boolean>;
  deletedTaskIds: Record<string, boolean>;
  service: TaskService | null;
  unsubscribe: (() => void) | null;
  destroyed: boolean;
}

interface TaskInitialization {
  service: TaskService;
  eventSource: TaskEventSource;
  promise: Promise<void>;
}

const activeStatuses = new Set<DownloadTask["status"]>((["queued", "downloading", "paused", "processing"]));
const taskInitializations = new WeakMap<object, TaskInitialization>();

export const useTaskManagementStore = defineStore("task-management", {
  state: (): TaskState => ({
    byId: {}, sequence: 0, capacity: 100, filter: "all", status: "idle",
    error: null, eventError: null, pending: {}, deletedTaskIds: {}, service: null, unsubscribe: null,
    destroyed: false,
  }),
  getters: {
    allTasks: (state): DownloadTask[] => Object.values(state.byId),
    visibleTasks(): DownloadTask[] {
      return this.allTasks.filter((task) => matchesFilter(task, this.filter)).sort(compareTasks);
    },
    counts(): Record<TaskFilter, number> {
      const tasks = this.allTasks;
      return {
        all: tasks.length,
        active: tasks.filter((task) => activeStatuses.has(task.status)).length,
        completed: tasks.filter((task) => task.status === "completed").length,
        failed: tasks.filter((task) => task.status === "failed").length,
      };
    },
    isNearCapacity(): boolean { return this.allTasks.length >= 90; },
    isFull(): boolean { return this.allTasks.length >= this.capacity; },
  },
  actions: {
    initialize(service: TaskService, eventSource: TaskEventSource, handoff?: HandoffSource): Promise<void> {
      const current = taskInitializations.get(this);
      const consumeHandoff = async (owner: TaskInitialization): Promise<boolean> => {
        const handoffId = handoff?.handoffId;
        if (!handoffId || handoff.items.length === 0 || this.destroyed) return true;
        try {
          const result = await service.create({ requestId: handoffId, drafts: [...handoff.items] });
          if (!this.destroyed && taskInitializations.get(this) === owner) {
            this.mergeResponse(result.sequence, result.tasks);
            handoff.clearIfMatches(handoffId);
          }
          return true;
        } catch (error: unknown) {
          if (!this.destroyed && taskInitializations.get(this) === owner) {
            this.error = normalizeIpcError(error);
            this.status = "failed";
          }
          return false;
        }
      };
      const reusable = current?.service === service
        && current.eventSource === eventSource
        && !this.destroyed
        && (this.status === "subscribing" || this.status === "loading" || (this.status === "ready" && Boolean(this.unsubscribe)));
      if (reusable) return current.promise.then(async () => { await consumeHandoff(current); });

      const initialization = {} as TaskInitialization;
      const promise = (async () => {
        this.unsubscribe?.();
        this.destroyed = false;
        this.service = service;
        this.status = "subscribing";
        this.error = null;
        this.eventError = null;
        this.deletedTaskIds = {};
        const buffered: BufferedEvent[] = [];
        let hydrating = true;
        const onProgress = (value: TaskProgressEvent) => hydrating ? buffered.push({ kind: "progress", value }) : this.applyProgress(value);
        const onRemoved = (value: TaskRemovedEvent) => hydrating ? buffered.push({ kind: "removed", value }) : this.applyRemoved(value);
        try {
          const unsubscribe = await eventSource.subscribe(onProgress, onRemoved);
          if (this.destroyed || taskInitializations.get(this) !== initialization) {
            unsubscribe();
            return;
          }
          this.unsubscribe = unsubscribe;
        } catch (error: unknown) {
          this.eventError = normalizeIpcError(error);
        }
        if (this.destroyed || taskInitializations.get(this) !== initialization) return;

        this.status = "loading";
        try {
          const snapshot = await service.list();
          if (this.destroyed || taskInitializations.get(this) !== initialization) return;
          this.byId = Object.fromEntries(snapshot.tasks.map((task) => [task.id, task]));
          this.sequence = snapshot.sequence;
          this.capacity = snapshot.capacity;
          hydrating = false;
          for (const event of buffered) {
            if (event.kind === "progress") this.applyProgress(event.value);
            else this.applyRemoved(event.value);
          }
          if (!await consumeHandoff(initialization)) return;
          this.status = "ready";
        } catch (error: unknown) {
          hydrating = false;
          if (!this.destroyed && taskInitializations.get(this) === initialization) {
            this.error = normalizeIpcError(error);
            this.status = "failed";
          }
        }
      })();
      Object.assign(initialization, { service, eventSource, promise });
      taskInitializations.set(this, initialization);
      return promise;
    },
    applyProgress(event: TaskProgressEvent) {
      if (event.sequence <= this.sequence || this.destroyed) return;
      this.sequence = event.sequence;
      if (this.deletedTaskIds[event.task.id]) return;
      const current = this.byId[event.task.id];
      if (current && current.revision > event.task.revision) return;
      this.byId[event.task.id] = event.task;
    },
    applyRemoved(event: TaskRemovedEvent) {
      if (event.sequence <= this.sequence || this.destroyed) return;
      delete this.byId[event.taskId];
      delete this.deletedTaskIds[event.taskId];
      this.sequence = event.sequence;
    },
    mergeResponse(sequence: number, tasks: DownloadTask[]) {
      if (sequence < this.sequence || this.destroyed) return;
      for (const task of tasks) this.byId[task.id] = task;
      this.sequence = sequence;
    },
    async control(taskId: string, action: TaskAction) {
      if (!this.service || this.pending[taskId]) return;
      this.pending[taskId] = true;
      this.error = null;
      try {
        const task = await this.service.control({ taskId, action });
        if (action === "delete") {
          delete this.byId[taskId];
          this.deletedTaskIds[taskId] = true;
        }
        else this.byId[task.id] = task;
      } catch (error: unknown) {
        this.error = normalizeIpcError(error);
      } finally {
        delete this.pending[taskId];
      }
    },
    async pauseAll() {
      if (!this.service || this.pending.batchPause) return;
      this.pending.batchPause = true;
      try { await this.service.pauseAll(); }
      catch (error: unknown) { this.error = normalizeIpcError(error); }
      finally { delete this.pending.batchPause; }
    },
    async clearCompleted() {
      if (!this.service || this.pending.batchClear) return;
      this.pending.batchClear = true;
      try {
        const result = await this.service.clearCompleted();
        result.removedTaskIds.forEach((id) => delete this.byId[id]);
        this.sequence = Math.max(this.sequence, result.sequence);
      } catch (error: unknown) { this.error = normalizeIpcError(error); }
      finally { delete this.pending.batchClear; }
    },
    async open(taskId: string, target: "file" | "directory") {
      if (!this.service) return;
      try { await this.service.open({ taskId, target }); }
      catch (error: unknown) { this.error = normalizeIpcError(error); }
    },
    dispose() {
      this.destroyed = true;
      this.unsubscribe?.();
      this.unsubscribe = null;
      taskInitializations.delete(this);
    },
  },
});

function matchesFilter(task: DownloadTask, filter: TaskFilter): boolean {
  if (filter === "all") return true;
  if (filter === "active") return activeStatuses.has(task.status);
  return task.status === filter;
}
