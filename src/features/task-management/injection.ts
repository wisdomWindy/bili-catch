import type { InjectionKey } from "vue";
import type { TaskEventSource } from "../../services/ipc/events";
import type { TaskService } from "./service";

export const taskServiceKey: InjectionKey<TaskService> = Symbol("task-service");
export const taskEventSourceKey: InjectionKey<TaskEventSource> = Symbol("task-event-source");
