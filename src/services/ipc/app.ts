import { normalizeIpcError } from "../../contracts/app-error";
import type { AppInfo, HealthStatus, IpcTransport } from "../../contracts/ipc";

export interface AppIpcService {
  getAppInfo(): Promise<AppInfo>;
  checkBackendHealth(): Promise<HealthStatus>;
}

async function invokeOrNormalize<T>(transport: IpcTransport, command: string): Promise<T> {
  try {
    return await transport.invoke<T>(command);
  } catch (error: unknown) {
    throw normalizeIpcError(error);
  }
}

export function createAppIpcService(transport: IpcTransport): AppIpcService {
  return {
    getAppInfo: () => invokeOrNormalize<AppInfo>(transport, "get_app_info"),
    checkBackendHealth: () => invokeOrNormalize<HealthStatus>(transport, "health_check"),
  };
}
