import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import PathSetting from "./PathSetting.vue";

describe("PathSetting", () => {
  it("renders a readonly full path and an accessible icon command", async () => {
    const wrapper = mount(PathSetting, {
      props: {
        id: "setting-download-directory",
        field: "downloadDirectory",
        label: "下载目录",
        value: "D:/A very long folder/BiliCatch",
        status: { status: "idle", error: null },
        picking: false,
        selectLabel: "选择下载目录",
      },
      global: { plugins: [createAppI18n()] },
    });
    const input = wrapper.get("input");
    const button = wrapper.get("button");
    expect(input.attributes("readonly")).toBeDefined();
    expect(input.attributes("title")).toBe("D:/A very long folder/BiliCatch");
    expect(input.attributes("aria-describedby")).toBe("setting-download-directory-status");
    expect(button.attributes("aria-label")).toBe("选择下载目录");
    expect(button.attributes("title")).toBe("选择下载目录");
    await button.trigger("click");
    expect(wrapper.emitted("select")?.[0]?.[0]).toBe(button.element);
  });

  it("describes the input with both the optional description and save status", () => {
    const wrapper = mount(PathSetting, {
      props: {
        id: "setting-temporary-directory",
        field: "temporaryDirectory",
        label: "临时目录",
        description: "用于处理中间文件",
        value: "D:/Temp",
        status: { status: "idle", error: null },
        picking: false,
        selectLabel: "选择临时目录",
      },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.get("input").attributes("aria-describedby")).toBe(
      "setting-temporary-directory-description setting-temporary-directory-status",
    );
  });
});
