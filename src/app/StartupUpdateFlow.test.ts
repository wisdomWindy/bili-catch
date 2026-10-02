import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { createPinia } from "pinia";
import { createMemoryHistory } from "vue-router";
import { describe, expect, it, vi } from "vitest";
import type { SettingsEffectSink } from "../features/settings/effects";
import {
  releaseActionsKey,
  settingsEffectSinkKey,
  settingsServiceKey,
} from "../features/settings/injection";
import type { ReleaseActionsPort } from "../features/settings/release";
import type { SettingsService } from "../features/settings/service";
import { useSettingsStore } from "../features/settings/store";
import { createAppI18n } from "../locales";
import App from "./App.vue";
import { createAppRouter } from "./router";

function snapshot(autoCheckUpdates: boolean, revision = 0) {
  return {
    schemaVersion: 1 as const,
    revision,
    values: {
      downloadDirectory: "D:/Media",
      temporaryDirectory: "D:/Temp",
      maxConcurrentTasks: 3,
      connectionsPerTask: 8,
      defaultVideoQuality: "80" as const,
      defaultAudioFormat: "mp3" as const,
      theme: "system" as const,
      locale: "zh-CN" as const,
      notifyOnComplete: true,
      completionSound: true,
      closeBehavior: "minimizeToTray" as const,
      autoCheckUpdates,
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((onResolve, onReject) => {
    resolve = onResolve;
    reject = onReject;
  });
  return { promise, resolve, reject };
}

async function renderApp(
  release: ReleaseActionsPort,
  service: SettingsService,
): Promise<{ wrapper: VueWrapper; store: ReturnType<typeof useSettingsStore>; effects: SettingsEffectSink }> {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn().mockReturnValue({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
  const pinia = createPinia();
  const router = createAppRouter(createMemoryHistory());
  await router.push("/download");
  await router.isReady();
  const effects: SettingsEffectSink = {
    applyAppearance: vi.fn(),
    applyCommitted: vi.fn(),
  };
  const wrapper = mount(App, {
    attachTo: document.body,
    props: {
      appService: {
        getAppInfo: vi.fn().mockResolvedValue({ name: "BiliCatch", version: "0.1.16" }),
        checkBackendHealth: vi.fn().mockResolvedValue({ status: "ok", timestamp: "now" }),
      },
    },
    global: {
      plugins: [pinia, createAppI18n(), router],
      provide: {
        [settingsServiceKey as symbol]: service,
        [settingsEffectSinkKey as symbol]: effects,
        [releaseActionsKey as symbol]: release,
      },
      stubs: {
        transition: false,
        AppShell: { template: "<main><slot name='status' /><slot /></main>" },
        BackendBanner: true,
        DownloadNotificationBridge: { template: "<div><slot /></div>" },
      },
    },
  });
  await flushPromises();
  return { wrapper, store: useSettingsStore(pinia), effects };
}

function serviceFor(autoCheckUpdates: boolean): SettingsService {
  return {
    getSnapshot: vi.fn().mockResolvedValue(snapshot(autoCheckUpdates)),
    update: vi.fn(),
  };
}

describe("startup update flow", () => {
  it("checks once after enabled settings load and shows the available versions", async () => {
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();
    const checkForUpdates = vi.fn().mockResolvedValue({
      status: "available",
      currentVersion: "0.1.16",
      latestVersion: "0.2.0",
    });
    const release: ReleaseActionsPort = {
      checkForUpdates,
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    };
    const { wrapper, store, effects } = await renderApp(release, serviceFor(true));

    expect(checkForUpdates).toHaveBeenCalledOnce();
    const dialog = document.body.querySelector("[role='alertdialog']");
    expect(dialog?.textContent).toContain("0.1.16");
    expect(dialog?.textContent).toContain("0.2.0");
    await vi.waitFor(() => {
      expect(document.activeElement?.getAttribute("data-testid")).toBe("update-later");
    });
    document.body.querySelector<HTMLButtonElement>("[data-testid='update-later']")?.click();
    await flushPromises();
    await vi.waitFor(() => {
      expect(document.body.querySelector<HTMLElement>("[role='alertdialog']")?.style.display).toBe("none");
      expect(document.activeElement).toBe(opener);
    });

    store.applySnapshot(snapshot(true, 1), effects);
    await flushPromises();
    expect(checkForUpdates).toHaveBeenCalledOnce();
    wrapper.unmount();
    opener.remove();
  });

  it("does not check when automatic updates are disabled or settings fail to load", async () => {
    const disabledCheck = vi.fn();
    const disabled = await renderApp({
      checkForUpdates: disabledCheck,
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    }, serviceFor(false));
    expect(disabledCheck).not.toHaveBeenCalled();
    disabled.wrapper.unmount();

    const failedCheck = vi.fn();
    const failed = await renderApp({
      checkForUpdates: failedCheck,
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    }, {
      getSnapshot: vi.fn().mockRejectedValue(new Error("settings failed")),
      update: vi.fn(),
    });
    expect(failedCheck).not.toHaveBeenCalled();
    failed.wrapper.unmount();
  });

  it("keeps latest and failed startup checks silent", async () => {
    const latest = await renderApp({
      checkForUpdates: vi.fn().mockResolvedValue({ status: "latest", currentVersion: "0.1.16" }),
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    }, serviceFor(true));
    expect(document.body.querySelector("[role='alertdialog']")).toBeNull();
    latest.wrapper.unmount();

    const failed = await renderApp({
      checkForUpdates: vi.fn().mockRejectedValue(new Error("network detail")),
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    }, serviceFor(true));
    expect(document.body.querySelector("[role='alertdialog']")).toBeNull();
    failed.wrapper.unmount();
  });

  it("ignores an available update that resolves after the app unmounts", async () => {
    const pendingCheck = deferred<Awaited<ReturnType<ReleaseActionsPort["checkForUpdates"]>>>();
    const release: ReleaseActionsPort = {
      checkForUpdates: vi.fn().mockReturnValue(pendingCheck.promise),
      installUpdate: vi.fn(),
      openLicenses: vi.fn(),
    };
    const { wrapper } = await renderApp(release, serviceFor(true));

    wrapper.unmount();
    pendingCheck.resolve({
      status: "available",
      currentVersion: "0.1.16",
      latestVersion: "0.2.0",
    });
    await flushPromises();

    expect(document.body.querySelector("[role='alertdialog']")).toBeNull();
  });

  it("locks the dialog during installation and allows retry after failure", async () => {
    const firstInstall = deferred<void>();
    const installUpdate = vi.fn()
      .mockReturnValueOnce(firstInstall.promise)
      .mockResolvedValueOnce(undefined);
    const { wrapper } = await renderApp({
      checkForUpdates: vi.fn().mockResolvedValue({
        status: "available",
        currentVersion: "0.1.16",
        latestVersion: "0.2.0",
      }),
      installUpdate,
      openLicenses: vi.fn(),
    }, serviceFor(true));

    const install = document.body.querySelector<HTMLButtonElement>("[data-testid='install-startup-update']");
    install?.click();
    await flushPromises();
    expect(installUpdate).toHaveBeenCalledOnce();
    expect(document.body.querySelector("[role='alertdialog']")).not.toBeNull();
    expect(document.body.querySelector("[data-testid='update-later']")?.hasAttribute("disabled")).toBe(true);
    const pendingDialog = document.body.querySelector<HTMLElement>("[role='alertdialog']");
    const tabEvent = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    pendingDialog?.dispatchEvent(tabEvent);
    expect(tabEvent.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(pendingDialog);
    pendingDialog?.dispatchEvent(new KeyboardEvent("keydown", {
      key: "Escape",
      bubbles: true,
    }));
    await flushPromises();
    expect(document.body.querySelector<HTMLElement>("[role='alertdialog']")?.style.display).not.toBe("none");

    install?.click();
    await flushPromises();
    expect(installUpdate).toHaveBeenCalledOnce();
    firstInstall.reject(new Error("private installer detail"));
    await flushPromises();
    expect(document.body.textContent).toContain("更新安装失败");

    document.body.querySelector<HTMLButtonElement>("[data-testid='install-startup-update']")?.click();
    await flushPromises();
    expect(installUpdate).toHaveBeenCalledTimes(2);
    await vi.waitFor(() => {
      expect(document.body.querySelector<HTMLElement>("[role='alertdialog']")?.style.display).toBe("none");
    });
    wrapper.unmount();
  });
});
