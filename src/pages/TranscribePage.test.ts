import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { defineComponent, h, nextTick } from "vue";
import { NMessageProvider } from "naive-ui";
import { createMemoryHistory, RouterView } from "vue-router";
import { describe, expect, it, vi } from "vitest";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import type { TranscriptionEventSource } from "../features/transcription/events";
import { transcriptionEventSourceKey, transcriptionServiceKey } from "../features/transcription/injection";
import type { TranscriptionService } from "../features/transcription/service";
import { useTranscriptionStore } from "../features/transcription/store";
import type { TaskEventSource } from "../services/ipc/events";
import { taskEventSourceKey, taskServiceKey } from "../features/task-management/injection";
import type { TaskService } from "../features/task-management/service";
import { useTaskManagementStore } from "../features/task-management/store";
import TranscribePage from "./TranscribePage.vue";

describe("TranscribePage", () => {
  it("keeps shared task subscriptions alive when navigating to the task list", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const transcriptionUnsubscribe = vi.fn();
    const taskUnsubscribe = vi.fn();
    const transcriptionService = {
      listTasks: vi.fn(async () => ({ sequence: 0, tasks: [] })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [] })),
    } as unknown as TranscriptionService;
    const taskService = {
      list: vi.fn(async () => ({ sequence: 0, capacity: 100, tasks: [] })),
      create: vi.fn(), control: vi.fn(), pauseAll: vi.fn(), clearCompleted: vi.fn(), open: vi.fn(),
    } as unknown as TaskService;
    const router = createAppRouter(createMemoryHistory());
    await router.push("/transcribe");
    await router.isReady();
    const transcriptionSubscribe = vi.fn(async () => transcriptionUnsubscribe);
    const taskSubscribe = vi.fn(async () => taskUnsubscribe);
    const Host = defineComponent({ setup: () => () => h(NMessageProvider, null, { default: () => h(RouterView) }) });
    const wrapper = mount(Host, {
      global: {
        plugins: [pinia, router, createAppI18n()],
        provide: {
          [transcriptionServiceKey as symbol]: transcriptionService,
          [transcriptionEventSourceKey as symbol]: { subscribe: transcriptionSubscribe } satisfies TranscriptionEventSource,
          [taskServiceKey as symbol]: taskService,
          [taskEventSourceKey as symbol]: { subscribe: taskSubscribe } satisfies TaskEventSource,
        },
      },
    });
    await flushPromises();

    await router.push("/tasks");
    await flushPromises();
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => window.setTimeout(resolve, 0)));
    await flushPromises();

    expect(transcriptionUnsubscribe).not.toHaveBeenCalled();
    expect(taskUnsubscribe).not.toHaveBeenCalled();
    expect(transcriptionSubscribe).toHaveBeenCalledOnce();
    expect(taskSubscribe).toHaveBeenCalledOnce();
    wrapper.unmount();
    useTranscriptionStore(pinia).dispose();
    useTaskManagementStore(pinia).dispose();
  });

  it("defaults the language to simplified Chinese before settings load", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createAppRouter(createMemoryHistory());
    await router.push("/transcribe");
    await router.isReady();
    const Host = defineComponent({ setup: () => () => h(NMessageProvider, null, { default: () => h(TranscribePage) }) });
    const wrapper = mount(Host, { global: { plugins: [pinia, router, createAppI18n()] } });

    const languageField = wrapper.findAll(".option-grid label")
      .find((field) => field.text().includes("语言"));
    expect(languageField).toBeDefined();
    if (!languageField) throw new Error("language field should render");
    expect((languageField.get("select").element as HTMLSelectElement).value).toBe("zh");

    wrapper.unmount();
  });

  it("keeps start disabled until source inspection and model validation succeed", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const createTask = vi.fn(async () => ({
      reused: false,
      task: { id: "task-new" },
    }));
    const service = {
      listTasks: vi.fn(async () => ({ sequence: 0, tasks: [] })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [{
        id: "base", displayName: "Base", fileName: "ggml-base.bin", sizeBytes: 100,
        installed: false, valid: false, isDefault: true, inUse: false,
        recommendedFor: "Balanced", capabilities: [], downloadState: "idle",
        downloadedBytes: 0, revision: 0,
      }] })),
      inspectSource: vi.fn(async () => ({
        fileName: "voice.wav", path: "D:/voice.wav", durationSeconds: 12,
        sizeBytes: 200, mediaKind: "audio", format: "wav",
      })),
      createTask,
    } as unknown as TranscriptionService;
    const events: TranscriptionEventSource = { subscribe: vi.fn(async () => () => undefined) };
    const router = createAppRouter(createMemoryHistory());
    await router.push("/transcribe");
    await router.isReady();
    const Host = defineComponent({ setup: () => () => h(NMessageProvider, null, { default: () => h(TranscribePage) }) });
    const wrapper = mount(Host, {
      global: {
        plugins: [pinia, router, createAppI18n()],
        provide: {
          [transcriptionServiceKey as symbol]: service,
          [transcriptionEventSourceKey as symbol]: events,
        },
      },
    });
    await flushPromises();
    const start = wrapper.get(".start-transcription");
    expect(start.attributes("disabled")).toBeDefined();
    await wrapper.get("#transcription-source").setValue("D:/voice.wav");
    await wrapper.get(".source-input-row .secondary-button").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("voice.wav");
    expect(start.attributes("disabled")).toBeDefined();

    const model = useTranscriptionStore(pinia).modelsById.base;
    if (model) model.valid = true;
    await nextTick();
    expect(start.attributes("disabled")).toBeUndefined();
  });

  it("disables unsupported language and GPU choices for FireRedASR2", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const service = {
      listTasks: vi.fn(async () => ({ sequence: 0, tasks: [] })),
      listModels: vi.fn(async () => ({ sequence: 0, models: [{
        id: "fire-red-asr2", displayName: "FireRedASR2 CTC (INT8)",
        fileName: "fire-red-asr2-ctc-int8.onnx", sizeBytes: 100,
        installed: true, valid: true, isDefault: false, inUse: false,
        recommendedFor: "Chinese and English", capabilities: ["zh-en", "cpu"],
        downloadState: "completed", downloadedBytes: 100, revision: 0,
      }] })),
    } as unknown as TranscriptionService;
    const events: TranscriptionEventSource = { subscribe: vi.fn(async () => () => undefined) };
    const router = createAppRouter(createMemoryHistory());
    await router.push("/transcribe");
    await router.isReady();
    const Host = defineComponent({ setup: () => () => h(NMessageProvider, null, { default: () => h(TranscribePage) }) });
    const wrapper = mount(Host, {
      global: {
        plugins: [pinia, router, createAppI18n()],
        provide: {
          [transcriptionServiceKey as symbol]: service,
          [transcriptionEventSourceKey as symbol]: events,
        },
      },
    });
    await flushPromises();

    const selects = wrapper.findAll(".option-grid select");
    await selects[0].setValue("fire-red-asr2");

    expect(selects[1].get('option[value="ja"]').attributes("disabled")).toBeDefined();
    expect(selects[1].get('option[value="ko"]').attributes("disabled")).toBeDefined();
    expect(selects[3].get('option[value="enabled"]').attributes("disabled")).toBeDefined();
  });
});
