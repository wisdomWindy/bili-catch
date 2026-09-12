export type InvokeArguments = Record<string, unknown>;

export interface IpcTransport {
  invoke<T>(command: string, args?: InvokeArguments): Promise<T>;
}

export interface AppInfo {
  name: string;
  version: string;
}

export interface HealthStatus {
  status: "ok";
  timestamp: string;
}
