import { describe, expect, it, vi } from "vitest";
import { APP_ERROR_CODES, normalizeIpcError } from "../../contracts/app-error";
import type { IpcTransport } from "../../contracts/ipc";
import { createAppIpcService } from "./app";

describe("normalizeIpcError", () => {
  it("preserves a valid serialized AppError", () => {
    const error = normalizeIpcError({
      code: "E003",
      message: "Invalid link",
      details: "unsupported host",
    });

    expect(error).toEqual({
      code: "E003",
      message: "Invalid link",
      details: "unsupported host",
    });
  });

  it.each(["transport failed", null, 42, { message: "missing code" }])(
    "maps unknown rejection %j to E_INTERNAL",
    (input) => {
      const error = normalizeIpcError(input);

      expect(error.code).toBe("E_INTERNAL");
      expect(error.message).toBeTruthy();
    },
  );

  it("exposes the complete stable error-code set", () => {
    expect(APP_ERROR_CODES).toEqual([
      "E001",
      "E002",
      "E003",
      "E004",
      "E005",
      "E006",
      "E007",
      "E008",
      "E009",
      "E010",
      "E_INTERNAL",
    ]);
  });
});

describe("createAppIpcService", () => {
  it("maps app operations to their exact Tauri commands", async () => {
    const invoke = vi.fn(async (command: string): Promise<unknown> => {
      if (command === "get_app_info") return { name: "BiliCatch", version: "0.1.0" };
      return { status: "ok", timestamp: "2026-09-10T00:00:00Z" };
    });
    const transport: IpcTransport = {
      invoke: async <T>(command: string) => (await invoke(command)) as T,
    };
    const service = createAppIpcService(transport);

    await expect(service.getAppInfo()).resolves.toEqual({
      name: "BiliCatch",
      version: "0.1.0",
    });
    await expect(service.checkBackendHealth()).resolves.toEqual({
      status: "ok",
      timestamp: "2026-09-10T00:00:00Z",
    });
    expect(invoke).toHaveBeenNthCalledWith(1, "get_app_info");
    expect(invoke).toHaveBeenNthCalledWith(2, "health_check");
  });

  it("normalizes transport failures once at the service boundary", async () => {
    const transport: IpcTransport = {
      invoke: async () => Promise.reject({ code: "E007", message: "Backend unavailable" }),
    };
    const service = createAppIpcService(transport);

    await expect(service.checkBackendHealth()).rejects.toEqual({
      code: "E007",
      message: "Backend unavailable",
    });
  });
});
