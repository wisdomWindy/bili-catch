import { describe, expect, it, vi } from "vitest";
import type { AuthStateEvent } from "./contracts";
import { createAuthEventSource } from "./events";

describe("authentication event source", () => {
  it("uses the stable event name, unwraps its payload, and releases the listener", async () => {
    const unsubscribe = vi.fn();
    let deliver!: (event: { payload: AuthStateEvent }) => void;
    const listen = vi.fn(async (_name, handler) => {
      deliver = handler;
      return unsubscribe;
    });
    const onState = vi.fn();
    const source = createAuthEventSource(listen);

    const dispose = await source.subscribe(onState);
    const event: AuthStateEvent = {
      snapshot: {
        revision: 3,
        status: "anonymous",
        qrContent: null,
        expiresAt: null,
        account: null,
        error: null,
      },
    };
    deliver({ payload: event });
    dispose();

    expect(listen).toHaveBeenCalledWith("auth://state", expect.any(Function));
    expect(onState).toHaveBeenCalledWith(event);
    expect(unsubscribe).toHaveBeenCalledOnce();
  });
});
