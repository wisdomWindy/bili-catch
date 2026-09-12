import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import AuthAccountPanel from "./AuthAccountPanel.vue";

describe("AuthAccountPanel", () => {
  it("accepts only HTTPS Bilibili image hosts and preserves a nullable UID", async () => {
    const wrapper = mount(AuthAccountPanel, {
      props: {
        account: { mid: null, name: "", avatarUrl: "https://example.com/avatar.png" },
        pending: false,
      },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.find("img").exists()).toBe(false);
    expect(wrapper.text()).toContain("已登录账号");
    expect(wrapper.text()).not.toContain("UID");
    await wrapper.get("[data-testid='logout-button']").trigger("click");
    expect(wrapper.emitted("logout")).toHaveLength(1);
  });

  it("falls back when an allowed avatar fails to load", async () => {
    const wrapper = mount(AuthAccountPanel, {
      props: {
        account: {
          mid: "9001",
          name: "Fixture account",
          avatarUrl: "https://i0.hdslb.com/avatar.png",
        },
        pending: false,
      },
      global: { plugins: [createAppI18n()] },
    });

    await wrapper.get("img").trigger("error");
    expect(wrapper.find("img").exists()).toBe(false);
    expect(wrapper.find("[data-testid='avatar-fallback']").exists()).toBe(true);
  });
});
