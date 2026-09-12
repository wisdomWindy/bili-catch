import { normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import type { ParseVideoResult } from "./contracts";

export interface ParseVideoService {
  parseVideo(input: string): Promise<ParseVideoResult>;
}

export function createParseVideoService(transport: IpcTransport): ParseVideoService {
  return {
    async parseVideo(input: string): Promise<ParseVideoResult> {
      try {
        return await transport.invoke<ParseVideoResult>("parse_video", { input });
      } catch (error: unknown) {
        throw normalizeIpcError(error);
      }
    },
  };
}
