import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
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
