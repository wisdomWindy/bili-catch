import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./app/App.vue";
import { createAppRouter } from "./app/router";
import { createAppI18n } from "./locales";
import { createAppIpcService } from "./services/ipc/app";
import { createTauriIpcTransport } from "./services/ipc/client";
import { createParseVideoService } from "./features/download-center/service";
import { clipboardReaderKey, parseVideoServiceKey } from "./features/download-center/injection";
import { createTauriClipboardReader } from "./features/download-center/clipboard";
import { createDemoParseVideoService } from "./features/download-center/demo-service";
import { createTaskService } from "./features/task-management/service";
import { taskEventSourceKey, taskServiceKey } from "./features/task-management/injection";
import { createTaskEventSource } from "./services/ipc/events";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { createDemoTaskRuntime } from "./features/task-management/demo-service";
import { createAuthService } from "./features/authentication/service";
import { createAuthEventSource } from "./features/authentication/events";
import { authEventSourceKey, authServiceKey } from "./features/authentication/injection";
import { createDemoAuthRuntime } from "./features/authentication/demo";
import type { AuthStatus } from "./features/authentication/contracts";
import { useAppStore } from "./stores/app";
import { useDownloadCenterStore } from "./features/download-center/store";
import { createSettingsService } from "./features/settings/service";
import { createTauriDirectoryPicker } from "./features/settings/dialog";
import { createTauriReleaseActions } from "./features/settings/release";
import { createDemoSettingsRuntime } from "./features/settings/demo";
import {
  directoryPickerKey,
  releaseActionsKey,
  settingsEffectSinkKey,
  settingsServiceKey,
} from "./features/settings/injection";
import { createSettingsEffectSink } from "./app/settings-effects";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/layout.css";
import "./styles/pages.css";
import "./features/download-center/download-center.css";
import "./features/task-management/task-management.css";
import "./features/authentication/authentication.css";
import "./features/settings/settings.css";

const router = createAppRouter();
const pinia = createPinia();
const i18n = createAppI18n();
const transport = createTauriIpcTransport();
const demoMode = import.meta.env.DEV && window.location.hash.includes("demo=1");
const appService = demoMode
  ? {
      getAppInfo: async () => ({ name: "BiliCatch", version: "0.1.0" }),
      checkBackendHealth: async () => ({ status: "ok" as const, timestamp: new Date().toISOString() }),
    }
  : createAppIpcService(transport);
const parseVideoService = demoMode
  ? createDemoParseVideoService()
  : createParseVideoService(transport);
const clipboardReader = demoMode
  ? { readText: async () => "" }
  : createTauriClipboardReader();
const demoTaskMode = window.location.hash.includes("empty=1") ? "empty" : window.location.hash.includes("error=1") ? "error" : "normal";
const taskRuntime = demoMode
  ? createDemoTaskRuntime(demoTaskMode)
  : { service: createTaskService(transport), events: createTaskEventSource() };
const authDemoStatus = ([
  "restoring", "anonymous", "requesting", "waiting_scan", "waiting_confirm",
  "authenticated", "expired", "cancelled", "error",
] as AuthStatus[]).find((status) => window.location.hash.includes(`auth=${status}`));
const authRuntime = demoMode
  ? createDemoAuthRuntime(authDemoStatus)
  : { service: createAuthService(transport), events: createAuthEventSource() };
const settingsRuntime = demoMode
  ? createDemoSettingsRuntime(window.location.hash)
  : {
      service: createSettingsService(transport),
      directoryPicker: createTauriDirectoryPicker(),
      releaseActions: createTauriReleaseActions(),
    };
const settingsEffects = createSettingsEffectSink(
  useAppStore(pinia),
  useDownloadCenterStore(pinia),
);
if (demoMode) {
  if (window.location.hash.includes("theme=light")) useAppStore(pinia).setTheme("light");
  if (window.location.hash.includes("theme=dark")) useAppStore(pinia).setTheme("dark");
  if (window.location.hash.includes("locale=en-US")) useAppStore(pinia).setLocale("en-US");
}

createApp(App, { appService })
  .provide(parseVideoServiceKey, parseVideoService)
  .provide(clipboardReaderKey, clipboardReader)
  .provide(taskServiceKey, taskRuntime.service)
  .provide(taskEventSourceKey, taskRuntime.events)
  .provide(authServiceKey, authRuntime.service)
  .provide(authEventSourceKey, authRuntime.events)
  .provide(settingsServiceKey, settingsRuntime.service)
  .provide(directoryPickerKey, settingsRuntime.directoryPicker)
  .provide(releaseActionsKey, settingsRuntime.releaseActions)
  .provide(settingsEffectSinkKey, settingsEffects)
  .use(pinia)
  .use(router)
  .use(i18n)
  .mount("#app");

if (!demoMode) {
  void tauriListen("system:update-check-requested", () => {
    window.dispatchEvent(new Event("bilicatch:update-check"));
  });
}
