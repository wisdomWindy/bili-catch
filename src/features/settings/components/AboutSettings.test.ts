import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import AboutSettings from "./AboutSettings.vue";

describe("AboutSettings", () => {
  it("shows runtime version and stable update/license actions", async () => {
    const wrapper = mount(AboutSettings, {
      props: { version: "0.1.0", updateStatus: "checking", updateMessage: null },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.text()).toContain("0.1.0");
    expect(wrapper.findAll("[data-about-row]")).toHaveLength(3);
    expect(wrapper.get("[data-testid='check-updates']").attributes("disabled")).toBeDefined();
    expect(wrapper.get("[data-testid='open-licenses']").attributes("aria-label")).toBe("开源许可");
  });
});
