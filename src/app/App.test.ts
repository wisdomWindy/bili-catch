import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi } from "vitest";
import { createAppI18n } from "../locales";
import type { SettingsEffectSink } from "../features/settings/effects";
import { settingsEffectSinkKey, settingsServiceKey } from "../features/settings/injection";
import type { SettingsService } from "../features/settings/service";
import App from "./App.vue";

describe("App settings lifecycle", () => {
  it("hydrates settings once at the root regardless of the current route", async () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockReturnValue({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }),
    });
    const service: SettingsService = {
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
    const effects: SettingsEffectSink = {
      applyAppearance: vi.fn(),
      applyCommitted: vi.fn(),
    };
    const wrapper = mount(App, {
      props: {
        appService: {
          getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.0" }),
          checkBackendHealth: vi.fn().mockResolvedValue({ status: "ok", timestamp: "now" }),
        },
      },
      global: {
        plugins: [createPinia(), createAppI18n()],
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
});
