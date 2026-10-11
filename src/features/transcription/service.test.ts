import { describe, expect, it, vi } from "vitest";
import { createTranscriptionService } from "./service";
import type { CreateTranscribeTaskRequest } from "./contracts";

describe("transcription service", () => {
  it("uses the frozen Tauri command names and request envelope", async () => {
    const invoke = vi.fn(async (_command: string, _args?: Record<string, unknown>) => ({ sequence: 0, tasks: [] }));
    const service = createTranscriptionService({
      invoke: async <T>(command: string, args?: Record<string, unknown>) =>
        (await invoke(command, args)) as T,
    });
    const request = {
      source: { kind: "localFile", path: "D:/voice.wav" },
      options: {
        modelId: "base", language: "auto", outputFormat: "srt", timestamps: true,
        punctuation: true, outputDirectory: "D:/out", gpuMode: "auto",
      },
      force: false,
    } satisfies CreateTranscribeTaskRequest;

    await service.listTasks();
    await service.createTask(request);
    await service.controlTask({ taskId: "task-1", action: "cancel" });
    await service.listModels();
    await service.getResult("task-1");

    expect(invoke.mock.calls).toEqual([
      ["list_transcribe_tasks", undefined],
      ["create_transcribe_task", { request }],
      ["control_transcribe_task", { request: { taskId: "task-1", action: "cancel" } }],
      ["list_asr_models", undefined],
      ["get_transcript_result", { request: { taskId: "task-1" } }],
    ]);
  });
});
