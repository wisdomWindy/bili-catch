import { normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import type {
  AsrModel,
  AsrModelId,
  AsrModelListSnapshot,
  ControlTranscribeTaskRequest,
  CreateTranscribeTaskRequest,
  CreateTranscribeTaskResult,
  ExportTranscriptRequest,
  ExportTranscriptResult,
  MediaSourceInfo,
  SaveTranscriptEditsRequest,
  TranscribeTask,
  TranscribeTaskListSnapshot,
  TranscriptResult,
  TranscriptionSource,
} from "./contracts";

export interface TranscriptionService {
  inspectSource(source: TranscriptionSource): Promise<MediaSourceInfo>;
  listTasks(): Promise<TranscribeTaskListSnapshot>;
  createTask(request: CreateTranscribeTaskRequest): Promise<CreateTranscribeTaskResult>;
  controlTask(request: ControlTranscribeTaskRequest): Promise<TranscribeTask>;
  clearCompleted(): Promise<string[]>;
  listModels(): Promise<AsrModelListSnapshot>;
  downloadModel(modelId: AsrModelId): Promise<AsrModel>;
  cancelModelDownload(modelId: AsrModelId): Promise<AsrModel>;
  verifyModel(modelId: AsrModelId): Promise<AsrModel>;
  deleteModel(modelId: AsrModelId): Promise<AsrModelListSnapshot>;
  getResult(taskId: string): Promise<TranscriptResult>;
  saveEdits(request: SaveTranscriptEditsRequest): Promise<TranscriptResult>;
  exportResult(request: ExportTranscriptRequest): Promise<ExportTranscriptResult>;
  openOutput(taskId: string): Promise<void>;
}

export function createTranscriptionService(transport: IpcTransport): TranscriptionService {
  const call = async <T>(command: string, request?: unknown): Promise<T> => {
    try {
      return await transport.invoke<T>(command, request === undefined ? undefined : { request });
    } catch (error: unknown) {
      throw normalizeIpcError(error);
    }
  };
  return {
    inspectSource: (source) => call("inspect_transcription_source", { source }),
    listTasks: () => call("list_transcribe_tasks"),
    createTask: (request) => call("create_transcribe_task", request),
    controlTask: (request) => call("control_transcribe_task", request),
    clearCompleted: () => call("clear_completed_transcribe_tasks"),
    listModels: () => call("list_asr_models"),
    downloadModel: (modelId) => call("download_asr_model", { modelId }),
    cancelModelDownload: (modelId) => call("cancel_asr_model_download", { modelId }),
    verifyModel: (modelId) => call("verify_asr_model", { modelId }),
    deleteModel: (modelId) => call("delete_asr_model", { modelId }),
    getResult: (taskId) => call("get_transcript_result", { taskId }),
    saveEdits: (request) => call("save_transcript_edits", request),
    exportResult: (request) => call("export_transcript", request),
    openOutput: (taskId) => call("open_transcript_output", { taskId }),
  };
}
