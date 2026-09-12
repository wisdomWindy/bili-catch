import { listen as tauriListen } from "@tauri-apps/api/event";
import type { TaskProgressEvent, TaskRemovedEvent } from "../../features/task-management/contracts";

type Unsubscribe = () => void;
type EventEnvelope<T> = { payload: T };
type Listen = <T>(event: string, handler: (event: EventEnvelope<T>) => void) => Promise<Unsubscribe>;

export interface TaskEventSource {
  subscribe(
    onProgress: (event: TaskProgressEvent) => void,
    onRemoved: (event: TaskRemovedEvent) => void,
  ): Promise<Unsubscribe>;
}

export function createTaskEventSource(listen: Listen = tauriListen as Listen): TaskEventSource {
  return {
    async subscribe(onProgress, onRemoved) {
      const unsubscribers: Unsubscribe[] = [];
      try {
        unsubscribers.push(await listen<TaskProgressEvent>("download://progress", ({ payload }) => onProgress(payload)));
        unsubscribers.push(await listen<TaskRemovedEvent>("download://removed", ({ payload }) => onRemoved(payload)));
      } catch (error) {
        unsubscribers.forEach((unsubscribe) => unsubscribe());
        throw error;
      }
      return () => unsubscribers.forEach((unsubscribe) => unsubscribe());
    },
  };
}
