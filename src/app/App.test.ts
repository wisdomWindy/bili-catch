import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppI18n } from "../locales";
import type { SettingsEffectSink } from "../features/settings/effects";
import { settingsEffectSinkKey, settingsServiceKey } from "../features/settings/injection";
import type { SettingsService } from "../features/settings/service";
import { createAppRouter } from "./router";
import App from "./App.vue";

function installMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
}

function settingsFixture(): SettingsService {
  return {
    getSnapshot: vi.fn().mockResolvedValue({
      schemaVersion: 1,
      revision: 0,
      values: {
        downloadDirectory: "D:/Media",
        temporaryDirectory: "D:/Temp",
        maxConcurrentTasks: 3,
        connectionsPerTask: 8,
        defaultVideoQuality: "80",
        defaultAudioFormat: "mp3",
        theme: "system",
        locale: "zh-CN",
        notifyOnComplete: true,
        closeBehavior: "minimizeToTray",
        autoCheckUpdates: true,
      },
    }),
    update: vi.fn(),
  };
}

describe("App settings lifecycle", () => {
  it("hydrates settings once at the root regardless of the current route", async () => {
    installMatchMedia();
    const service = settingsFixture();
    const effects: SettingsEffectSink = {
      applyAppearance: vi.fn(),
      applyCommitted: vi.fn(),
    };
    const router = createAppRouter(createMemoryHistory());
    await router.push("/download");
    await router.isReady();
    const wrapper = mount(App, {
      props: {
        appService: {
          getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.0" }),
          checkBackendHealth: vi.fn().mockResolvedValue({ status: "ok", timestamp: "now" }),
        },
      },
      global: {
        plugins: [createPinia(), createAppI18n(), router],
        provide: {
          [settingsServiceKey as symbol]: service,
          [settingsEffectSinkKey as symbol]: effects,
        },
        stubs: {
          AppShell: { template: "<main><slot name='status' /><slot /></main>" },
          BackendBanner: true,
          DownloadNotificationBridge: { template: "<div><slot /></div>" },
        },
      },
    });
    await flushPromises();

    expect(wrapper.get(".app-provider").classes()).toContain("n-config-provider");
    expect(service.getSnapshot).toHaveBeenCalledOnce();
    expect(effects.applyCommitted).toHaveBeenCalledOnce();
    wrapper.unmount();
  });

  it("routes tray update requests to settings before starting the visible check", async () => {
    installMatchMedia();
    const router = createAppRouter(createMemoryHistory());
    await router.push("/download");
    await router.isReady();
    const updateRequested = vi.fn();
    window.addEventListener("bilicatch:update-check", updateRequested);
    const wrapper = mount(App, {
      props: {
        appService: {
          getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.12" }),
          checkBackendHealth: vi.fn().mockResolvedValue({ status: "ok", timestamp: "now" }),
        },
      },
      global: {
        plugins: [createPinia(), createAppI18n(), router],
        provide: {
          [settingsServiceKey as symbol]: settingsFixture(),
          [settingsEffectSinkKey as symbol]: {
            applyAppearance: vi.fn(),
            applyCommitted: vi.fn(),
          } satisfies SettingsEffectSink,
        },
        stubs: {
          AppShell: { template: "<main><slot name='status' /><slot /></main>" },
          BackendBanner: true,
          DownloadNotificationBridge: { template: "<div><slot /></div>" },
        },
      },
    });

    window.dispatchEvent(new Event("bilicatch:tray-update-check"));
    await flushPromises();

    expect(router.currentRoute.value.name).toBe("settings");
    expect(updateRequested).toHaveBeenCalledOnce();
    window.removeEventListener("bilicatch:update-check", updateRequested);
    wrapper.unmount();
  });
});
