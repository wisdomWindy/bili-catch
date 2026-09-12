import { describe, expect, it } from "vitest";
import type { AuthSnapshot, AuthStateEvent, AuthStatus } from "./contracts";

describe("authentication contracts", () => {
  it("preserves nullable account and QR fields in the public snapshot", () => {
    const snapshot: AuthSnapshot = {
      revision: 7,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: {
        mid: "9007199254740993",
        name: "测试账号",
        avatarUrl: "https://i0.hdslb.com/bfs/face/example.jpg",
      },
      error: null,
    };
    const event: AuthStateEvent = { snapshot };

    expect(event.snapshot).toEqual(snapshot);
    expect(event.snapshot.account?.mid).toBe("9007199254740993");
    expect(event.snapshot.qrContent).toBeNull();
  });

  it("keeps all lifecycle wire values explicit", () => {
    const statuses: AuthStatus[] = [
      "restoring",
      "anonymous",
      "requesting",
      "waiting_scan",
      "waiting_confirm",
      "authenticated",
      "expired",
      "cancelled",
      "error",
    ];

    expect(statuses).toHaveLength(9);
    expect(statuses[4]).toBe("waiting_confirm");
  });
});
