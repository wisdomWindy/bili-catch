import type { AppError } from "../../contracts/app-error";
import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateCheckResult =
  | { status: "latest"; currentVersion: string }
  | { status: "available"; currentVersion: string; latestVersion: string };

export interface ReleaseActionsPort {
  checkForUpdates(): Promise<UpdateCheckResult>;
  installUpdate(): Promise<void>;
  openLicenses(): Promise<void>;
}

function deferredError(details: "UPDATE_NOT_CONFIGURED" | "LICENSES_NOT_CONFIGURED"): AppError {
  return {
    code: "E_INTERNAL",
    message: "Release information is not configured",
    details,
  };
}

export function createDeferredReleaseActions(): ReleaseActionsPort {
  return {
    async checkForUpdates() {
      throw deferredError("UPDATE_NOT_CONFIGURED");
    },
    async installUpdate() {
      throw deferredError("UPDATE_NOT_CONFIGURED");
    },
    async openLicenses() {
      throw deferredError("LICENSES_NOT_CONFIGURED");
    },
  };
}

export function createTauriReleaseActions(): ReleaseActionsPort {
  let pendingUpdate: Update | null = null;

  return {
    async checkForUpdates() {
      const update = await check();
      pendingUpdate = update;
      if (!update) {
        return { status: "latest", currentVersion: await getVersion() };
      }
      return {
        status: "available",
        currentVersion: update.currentVersion,
        latestVersion: update.version,
      };
    },
    async installUpdate() {
      const update = pendingUpdate ?? await check();
      if (!update) return;
      pendingUpdate = update;
      await update.downloadAndInstall();
      pendingUpdate = null;
      await relaunch();
    },
    async openLicenses() {
      throw deferredError("LICENSES_NOT_CONFIGURED");
    },
  };
}
