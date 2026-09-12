import { defineStore } from "pinia";
import { watch } from "vue";
import type {
  AppLocale,
  BackendStatus,
  ResolvedTheme,
  ThemePreference,
} from "../contracts/app";
import type { AppError } from "../contracts/app-error";
import type { AppInfo } from "../contracts/ipc";
import { normalizeIpcError } from "../contracts/app-error";
import type { AppIpcService } from "../services/ipc/app";

interface AppState {
  themePreference: ThemePreference;
  locale: AppLocale;
  backendStatus: BackendStatus;
  backendError: AppError | null;
  appInfo: AppInfo | null;
}

export const useAppStore = defineStore("app", {
  state: (): AppState => ({
    themePreference: "system",
    locale: "zh-CN",
    backendStatus: "idle",
    backendError: null,
    appInfo: null,
  }),
  actions: {
    setTheme(theme: ThemePreference) {
      this.themePreference = theme;
    },
    setLocale(locale: AppLocale) {
      this.locale = locale;
    },
    async initialize(service: AppIpcService) {
      this.backendStatus = "pending";
      this.backendError = null;
      try {
        const [appInfo] = await Promise.all([
          service.getAppInfo(),
          service.checkBackendHealth(),
        ]);
        this.appInfo = appInfo;
        this.backendStatus = "healthy";
      } catch (error: unknown) {
        this.backendError = normalizeIpcError(error);
        this.backendStatus = "failed";
      }
    },
  },
});

export function resolveTheme(preference: ThemePreference, systemIsDark: boolean): ResolvedTheme {
  return preference === "system" ? (systemIsDark ? "dark" : "light") : preference;
}

export function installThemeSync(
  store: ReturnType<typeof useAppStore>,
  root: HTMLElement = document.documentElement,
  mediaQuery: MediaQueryList = window.matchMedia("(prefers-color-scheme: dark)"),
  onResolve?: (theme: ResolvedTheme) => void,
): () => void {
  const applyTheme = () => {
    const theme = resolveTheme(store.themePreference, mediaQuery.matches);
    root.dataset.theme = theme;
    onResolve?.(theme);
  };
  const handleSystemChange = () => {
    if (store.themePreference === "system") applyTheme();
  };

  mediaQuery.addEventListener("change", handleSystemChange);
  const stopWatching = watch(() => store.themePreference, applyTheme, { immediate: true });

  return () => {
    stopWatching();
    mediaQuery.removeEventListener("change", handleSystemChange);
  };
}
