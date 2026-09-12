import { normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import type {
  BatchTaskResult,
  ClearTasksResult,
  ControlDownloadTaskRequest,
  CreateDownloadTasksRequest,
  CreateTasksResult,
  DownloadTask,
  OpenDownloadTaskRequest,
  TaskListSnapshot,
} from "./contracts";

export interface TaskService {
  list(): Promise<TaskListSnapshot>;
  create(request: CreateDownloadTasksRequest): Promise<CreateTasksResult>;
  control(request: ControlDownloadTaskRequest): Promise<DownloadTask>;
  pauseAll(): Promise<BatchTaskResult>;
  clearCompleted(): Promise<ClearTasksResult>;
  open(request: OpenDownloadTaskRequest): Promise<void>;
}

export function createTaskService(transport: IpcTransport): TaskService {
  const call = async <T>(command: string, args?: Record<string, unknown>): Promise<T> => {
    try {
      return args === undefined
        ? await transport.invoke<T>(command)
        : await transport.invoke<T>(command, args);
    } catch (error: unknown) {
      throw normalizeIpcError(error);
    }
  };
  return {
    list: () => call("list_download_tasks"),
    create: (request) => call("create_download_tasks", { request }),
    control: (request) => call("control_download_task", { request }),
    pauseAll: () => call("pause_all_download_tasks"),
    clearCompleted: () => call("clear_completed_tasks"),
    open: (request) => call("open_download_task", { request }),
  };
}
