import { normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import type { AuthSnapshot } from "./contracts";

export interface AuthService {
  getSnapshot(): Promise<AuthSnapshot>;
  start(): Promise<AuthSnapshot>;
  cancel(): Promise<AuthSnapshot>;
  logout(): Promise<AuthSnapshot>;
}

export function createAuthService(transport: IpcTransport): AuthService {
  const call = async (command: string): Promise<AuthSnapshot> => {
    try {
      return await transport.invoke<AuthSnapshot>(command);
    } catch (error: unknown) {
      throw normalizeIpcError(error);
    }
  };

  return {
    getSnapshot: () => call("get_auth_snapshot"),
    start: () => call("start_qr_login"),
    cancel: () => call("cancel_qr_login"),
    logout: () => call("logout"),
  };
}
