import { describe, expect, it, vi } from "vitest";
import type { IpcTransport } from "../../contracts/ipc";
import { createAuthService } from "./service";

describe("authentication service transport contract", () => {
  it("uses the four exact command names without argument objects", async () => {
    const invoke = vi.fn().mockResolvedValue({ revision: 0, status: "anonymous" });
    const service = createAuthService({ invoke } as IpcTransport);

    await service.getSnapshot();
    await service.start();
    await service.cancel();
    await service.logout();

    expect(invoke.mock.calls).toEqual([
      ["get_auth_snapshot"],
      ["start_qr_login"],
      ["cancel_qr_login"],
      ["logout"],
    ]);
  });

  it("normalizes unknown IPC failures", async () => {
    const service = createAuthService({ invoke: vi.fn().mockRejectedValue("offline") });
    await expect(service.getSnapshot()).rejects.toMatchObject({
      code: "E_INTERNAL",
      message: "offline",
    });
  });
});
