import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import AppShell from "../components/layout/AppShell.vue";

async function renderRoute(path: string) {
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const wrapper = mount(AppShell, {
    global: { plugins: [createPinia(), router, createAppI18n()] },
  });
  await flushPromises();
  return { wrapper, router };
}

describe("route pages", () => {
  it.each([
    ["/download", "下载中心"],
    ["/tasks", "任务列表"],
    ["/login", "账号登录"],
    ["/settings", "设置"],
  ])("renders the %s page", async (path, title) => {
    const { wrapper } = await renderRoute(path);
    expect(wrapper.get("[data-testid='page-heading']").text()).toBe(title);
  });

  it("offers a working recovery action on unknown routes", async () => {
    const { wrapper, router } = await renderRoute("/missing");
    expect(wrapper.get("[data-testid='page-heading']").text()).toBe("页面不存在");
    await wrapper.get("[data-testid='not-found-home']").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/download");
  });
});
