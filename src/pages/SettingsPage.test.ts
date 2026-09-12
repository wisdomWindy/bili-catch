import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { describe, expect, it, vi } from "vitest";
import { createAppI18n } from "../locales";
import type { SettingsSnapshot } from "../features/settings/contracts";
import {
  directoryPickerKey,
  releaseActionsKey,
  settingsEffectSinkKey,
  settingsServiceKey,
} from "../features/settings/injection";
import type { SettingsService } from "../features/settings/service";
import { useSettingsStore } from "../features/settings/store";
import SettingsPage from "./SettingsPage.vue";

const base: SettingsSnapshot = {
  schemaVersion: 1,
  revision: 0,
  values: {
    downloadDirectory: "D:/Downloads/BiliCatch",
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
};

async function renderReady(selectedDirectory: string | null = null) {
  const pinia = createPinia();
  setActivePinia(pinia);
  let current = structuredClone(base);
  const service: SettingsService = {
    getSnapshot: vi.fn().mockResolvedValue(current),
    update: vi.fn(async (patch) => {
      current = {
        ...current,
        revision: current.revision + 1,
        values: { ...current.values, [patch.field]: patch.value },
      };
      return structuredClone(current);
    }),
  };
  const effects = { applyAppearance: vi.fn(), applyCommitted: vi.fn() };
  const selectDirectory = vi.fn().mockResolvedValue(selectedDirectory);
  const checkForUpdates = vi.fn().mockResolvedValue({ status: "latest", currentVersion: "0.1.0" });
  const openLicenses = vi.fn().mockResolvedValue(undefined);
  await useSettingsStore(pinia).initialize(service, effects);
  const wrapper = mount(SettingsPage, {
    global: {
      plugins: [pinia, createAppI18n()],
      provide: {
        [settingsServiceKey as symbol]: service,
        [settingsEffectSinkKey as symbol]: effects,
        [directoryPickerKey as symbol]: { selectDirectory },
        [releaseActionsKey as symbol]: {
          checkForUpdates,
          installUpdate: vi.fn(),
          openLicenses,
        },
      },
    },
  });
  return { wrapper, service, effects, selectDirectory, checkForUpdates, openLicenses };
}

describe("SettingsPage", () => {
  it("renders four continuous sections, 11 controls, and three about rows", async () => {
    const { wrapper } = await renderReady();

    expect(wrapper.get("[data-testid='page-heading']").text()).toBe("设置");
    expect(wrapper.findAll(".settings-section")).toHaveLength(4);
    expect(wrapper.findAll("[data-setting-field]")).toHaveLength(11);
    expect(wrapper.findAll("[data-about-row]")).toHaveLength(3);
    expect(wrapper.text()).not.toContain("手动保存");
    expect(wrapper.find(".empty-state").exists()).toBe(false);
    for (const row of wrapper.findAll("[data-setting-field]")) {
      const label = row.get("label");
      expect(row.get(`#${label.attributes("for")}`).attributes("id")).toBe(label.attributes("for"));
    }
    expect(wrapper.findAll("[role='switch']")).toHaveLength(2);
  });

  it("saves selected directories, treats cancel as a no-op, and restores button focus", async () => {
    const selected = await renderReady("D:/Selected");
    const selectedButton = selected.wrapper.get("[aria-label='选择下载目录']");
    const focus = vi.spyOn(selectedButton.element as HTMLButtonElement, "focus");
    await selectedButton.trigger("click");
    await flushPromises();
    expect(selected.selectDirectory).toHaveBeenCalledWith("D:/Downloads/BiliCatch");
    expect(selected.service.update).toHaveBeenCalledWith({ field: "downloadDirectory", value: "D:/Selected" });
    expect(focus).toHaveBeenCalledOnce();

    const cancelled = await renderReady();
    await cancelled.wrapper.get("[aria-label='选择下载目录']").trigger("click");
    await flushPromises();
    expect(cancelled.service.update).not.toHaveBeenCalled();
  });

  it("keeps update and license actions in-page", async () => {
    const { wrapper, checkForUpdates, openLicenses } = await renderReady();
    await wrapper.get("[data-testid='check-updates']").trigger("click");
    await flushPromises();
    expect(checkForUpdates).toHaveBeenCalledOnce();
    expect(wrapper.text()).toContain("已是最新版本");

    await wrapper.get("[data-testid='open-licenses']").trigger("click");
    await flushPromises();
    expect(openLicenses).toHaveBeenCalledOnce();
  });

  it("commits selects immediately and applies appearance before persistence finishes", async () => {
    const { wrapper, service, effects } = await renderReady();

    await wrapper.get("#setting-theme").setValue("dark");

    expect(effects.applyAppearance).toHaveBeenLastCalledWith({ theme: "dark", locale: "zh-CN" });
    await flushPromises();
    expect(service.update).toHaveBeenCalledWith({ field: "theme", value: "dark" });
  });

  it("shows a stable loading skeleton and a retryable load error", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const pending = new Promise<SettingsSnapshot>(() => undefined);
    const effects = { applyAppearance: vi.fn(), applyCommitted: vi.fn() };
    const loadingService: SettingsService = { getSnapshot: vi.fn().mockReturnValue(pending), update: vi.fn() };
    void useSettingsStore(pinia).initialize(loadingService, effects);
    const loading = mount(SettingsPage, {
      global: { plugins: [pinia, createAppI18n()] },
    });
    expect(loading.findAll(".setting-skeleton")).toHaveLength(11);
    loading.unmount();

    const failedPinia = createPinia();
    setActivePinia(failedPinia);
    const failedService: SettingsService = {
      getSnapshot: vi.fn().mockRejectedValue({ code: "E_INTERNAL", message: "private detail" }),
      update: vi.fn(),
    };
    await useSettingsStore(failedPinia).initialize(failedService, effects);
    const failed = mount(SettingsPage, {
      global: {
        plugins: [failedPinia, createAppI18n()],
        provide: { [settingsServiceKey as symbol]: failedService },
      },
    });
    expect(failed.get("[role='alert']").text()).toContain("无法读取设置");
    expect(failed.text()).not.toContain("private detail");
    expect(failed.get("[data-testid='settings-retry']").element.tagName).toBe("BUTTON");
  });
});
