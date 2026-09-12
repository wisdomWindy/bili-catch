import { normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import type { SettingsPatch, SettingsSnapshot } from "./contracts";

export interface SettingsService {
  getSnapshot(): Promise<SettingsSnapshot>;
  update(patch: SettingsPatch): Promise<SettingsSnapshot>;
}

export function createSettingsService(transport: IpcTransport): SettingsService {
  return {
    async getSnapshot() {
      try {
        return await transport.invoke<SettingsSnapshot>("get_settings_snapshot");
      } catch (error: unknown) {
        throw normalizeIpcError(error);
      }
    },
    async update(patch) {
      try {
        return await transport.invoke<SettingsSnapshot>("update_setting", {
          request: { patch },
        });
      } catch (error: unknown) {
        throw normalizeIpcError(error);
      }
    },
  };
}
