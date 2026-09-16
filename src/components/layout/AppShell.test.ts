import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppI18n } from "../../locales";
import { createAppRouter } from "../../app/router";
import AppShell from "./AppShell.vue";
import { useAuthStore } from "../../features/authentication/store";
import { useAppStore } from "../../stores/app";

async function mountShell(path = "/download") {
  const pinia = createPinia();
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(AppShell, {
    global: { plugins: [pinia, router, createAppI18n()] },
  });
  return { wrapper, router, pinia };
}

describe("AppShell", () => {
  it("renders the stable brand, navigation, top bar, and content regions", async () => {
    const { wrapper } = await mountShell();

    wrapper.get("[data-testid='app-shell']");
    expect(wrapper.get("[data-testid='brand-link']").text()).toContain("BiliCatch");
    expect(wrapper.findAll("[data-testid='primary-nav'] a").map((link) => link.text())).toEqual([
      "下载中心",
      "任务列表",
      "设置",
    ]);
    expect(wrapper.get("[data-testid='topbar-title']").text()).toBe("下载中心");
    expect(wrapper.get("main").attributes("tabindex")).toBe("-1");
  });

  it("updates the active navigation state from the router", async () => {
    const { wrapper, router } = await mountShell();
    await router.push("/tasks");

    const active = wrapper.get("[data-testid='nav-tasks']");
    expect(active.attributes("aria-current")).toBe("page");
    expect(wrapper.get("[data-testid='topbar-title']").text()).toBe("任务列表");
  });

  it("keeps icon commands accessible and navigates to login and settings", async () => {
    const { wrapper, router } = await mountShell();
    const back = wrapper.get("[data-testid='back-button']");
    expect(back.attributes("aria-label")).toBe("返回");
    expect(back.attributes()).toHaveProperty("disabled");

    await wrapper.get("[data-testid='auth-button']").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/login");
    await wrapper.get("[data-testid='settings-button']").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/settings");
  });

  it("shows the authenticated account from the root store", async () => {
    const { wrapper, pinia } = await mountShell();
    const auth = useAuthStore(pinia);
    auth.snapshot = {
      revision: 2,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: "9001", name: "Fixture account", avatarUrl: null },
      error: null,
    };
    await flushPromises();

    expect(wrapper.get("[data-testid='auth-button']").text()).toContain("Fixture account");
    expect(wrapper.get("[data-testid='auth-button']").attributes("aria-label")).toContain("Fixture account");
  });

  it("shows the runtime package version instead of a hard-coded release", async () => {
    const { wrapper, pinia } = await mountShell();
    useAppStore(pinia).appInfo = { name: "BiliCatch", version: "0.1.12" };
    await flushPromises();

    expect(wrapper.get(".version-label").text()).toContain("0.1.12");
    expect(wrapper.get(".version-label").text()).not.toContain("0.1.0");
  });

  it("distinguishes restoring authentication from an anonymous account", async () => {
    const { wrapper, pinia } = await mountShell();
    const auth = useAuthStore(pinia);
    auth.snapshot = {
      revision: 1,
      status: "restoring",
      qrContent: null,
      expiresAt: null,
      account: null,
      error: null,
    };
    await flushPromises();

    expect(wrapper.get("[data-testid='auth-spinner']").classes()).toContain("spin");
    expect(wrapper.get("[data-testid='auth-button']").text()).toContain("正在恢复登录状态");
  });
});
