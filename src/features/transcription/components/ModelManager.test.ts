import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { createAppI18n } from "../../../locales";
import type { AsrModel } from "../contracts";
import ModelManager from "./ModelManager.vue";

const models: AsrModel[] = [{
  id: "base",
  displayName: "Base",
  fileName: "ggml-base.bin",
  sizeBytes: 147_951_465,
  installed: false,
  valid: false,
  isDefault: true,
  inUse: false,
  recommendedFor: "Recommended default",
  capabilities: ["multilingual", "timestamps"],
  downloadState: "idle",
  downloadedBytes: 0,
  revision: 0,
}];

afterEach(() => { document.body.innerHTML = ""; });

describe("ModelManager", () => {
  it("groups models by series while preserving their order within each series", async () => {
    const tiny: AsrModel = {
      ...models[0],
      id: "tiny",
      displayName: "Tiny",
      fileName: "ggml-tiny.bin",
      isDefault: false,
    };
    const fireRed: AsrModel = {
      ...models[0],
      id: "fire-red-asr2",
      displayName: "FireRedASR2 CTC (INT8)",
      fileName: "fire-red-asr2-ctc-int8.onnx",
      isDefault: false,
    };
    const wrapper = mount(ModelManager, {
      attachTo: document.body,
      props: { show: true, models: [tiny, fireRed, models[0]] },
      global: { plugins: [createAppI18n()] },
    });

    const sections = [...document.body.querySelectorAll("section[aria-labelledby]")];
    expect(sections).toHaveLength(2);
    expect(sections.map((section) => section.querySelector("h3")?.textContent)).toEqual([
      "Whisper 系列",
      "FireRedASR2 系列",
    ]);
    expect(sections[0].querySelectorAll(".model-row strong")).toHaveLength(2);
    expect([...sections[0].querySelectorAll(".model-row strong")].map((node) => node.textContent)).toEqual([
      "Tiny",
      "Base默认",
    ]);
    expect(sections[1].textContent).toContain("FireRedASR2 CTC (INT8)");
    const downloadButton = sections[0].querySelector<HTMLButtonElement>("button.icon-button");
    expect(downloadButton).not.toBeNull();
    downloadButton?.click();
    expect(wrapper.emitted("download")).toEqual([["tiny"]]);
    wrapper.unmount();
  });

  it("does not render an empty model series", () => {
    const wrapper = mount(ModelManager, {
      attachTo: document.body,
      props: { show: true, models },
      global: { plugins: [createAppI18n()] },
    });

    expect(document.body.textContent).toContain("Whisper 系列");
    expect(document.body.textContent).not.toContain("FireRedASR2 系列");
    wrapper.unmount();
  });

  it("collapses only model series that contain multiple models", async () => {
    const tiny: AsrModel = {
      ...models[0],
      id: "tiny",
      displayName: "Tiny",
      fileName: "ggml-tiny.bin",
      isDefault: false,
    };
    const fireRed: AsrModel = {
      ...models[0],
      id: "fire-red-asr2",
      displayName: "FireRedASR2 CTC (INT8)",
      fileName: "fire-red-asr2-ctc-int8.onnx",
      isDefault: false,
    };
    const wrapper = mount(ModelManager, {
      attachTo: document.body,
      props: { show: true, models: [tiny, models[0], fireRed] },
      global: { plugins: [createAppI18n()] },
    });

    const sections = [...document.body.querySelectorAll<HTMLElement>("section.model-series")];
    const whisperToggle = sections[0].querySelector<HTMLButtonElement>(".model-series-toggle");
    expect(whisperToggle).not.toBeNull();
    expect(whisperToggle?.getAttribute("aria-expanded")).toBe("true");
    expect(sections[0].querySelectorAll(".model-row")).toHaveLength(2);
    expect(sections[1].querySelector(".model-series-toggle")).toBeNull();

    whisperToggle?.click();
    await nextTick();
    expect(sections[0].querySelector(".model-series-toggle")?.getAttribute("aria-expanded")).toBe("false");
    expect(sections[0].querySelector(".model-series-list")).toBeNull();

    sections[0].querySelector<HTMLButtonElement>(".model-series-toggle")?.click();
    await nextTick();
    expect(sections[0].querySelector(".model-series-toggle")?.getAttribute("aria-expanded")).toBe("true");
    expect(sections[0].querySelectorAll(".model-row")).toHaveLength(2);

    sections[0].querySelector<HTMLButtonElement>(".model-series-toggle")?.click();
    await nextTick();
    await wrapper.setProps({ models: [models[0], fireRed] });
    expect(sections[0].querySelector(".model-series-toggle")).toBeNull();
    expect(sections[0].querySelectorAll(".model-row")).toHaveLength(1);
    wrapper.unmount();
  });

  it("renders only the modal's built-in close button", () => {
    const wrapper = mount(ModelManager, {
      attachTo: document.body,
      props: { show: true, models },
      global: { plugins: [createAppI18n()] },
    });

    const closeButtons = document.body.querySelectorAll(
      ".n-card-header__close, button[title='关闭']",
    );
    expect(closeButtons).toHaveLength(1);
    wrapper.unmount();
  });

  it("shows queued and failed download feedback in the model row", async () => {
    const wrapper = mount(ModelManager, {
      attachTo: document.body,
      props: { show: true, models: [{ ...models[0], downloadState: "queued" }] },
      global: { plugins: [createAppI18n()] },
    });
    expect(document.body.textContent).toContain("等待下载");

    await wrapper.setProps({
      models: [{
        ...models[0],
        downloadState: "failed",
        error: { code: "E012", message: "download failed" },
      }],
    });
    expect(document.body.textContent).toContain("语音模型下载失败");
    wrapper.unmount();
  });
});
