import type { AppError } from "../../contracts/app-error";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
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
  let checkInFlight: Promise<Update | null> | null = null;

  async function resolvePendingUpdate(): Promise<Update | null> {
    if (pendingUpdate) return pendingUpdate;
    checkInFlight ??= check();
    try {
      const update = await checkInFlight;
      pendingUpdate ??= update;
      return pendingUpdate;
    } finally {
      checkInFlight = null;
    }
  }

  return {
    async checkForUpdates() {
      const update = await resolvePendingUpdate();
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
      const update = await resolvePendingUpdate();
      if (!update) return;
      await update.downloadAndInstall();
      pendingUpdate = null;
      await relaunch();
    },
    async openLicenses() {
      await openUrl("https://github.com/wisdomWindy/bili-catch/blob/main/LICENSE");
    },
  };
}
