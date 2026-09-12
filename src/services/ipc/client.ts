import { invoke } from "@tauri-apps/api/core";
import type { InvokeArguments, IpcTransport } from "../../contracts/ipc";

export function createTauriIpcTransport(): IpcTransport {
  return {
    invoke<T>(command: string, args?: InvokeArguments): Promise<T> {
      return invoke<T>(command, args);
    },
  };
}
