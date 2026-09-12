import { describe, expect, it } from "vitest";
import { createDeferredReleaseActions } from "./release";

describe("deferred release actions", () => {
  it("does not invent update or license success before system-release", async () => {
    const actions = createDeferredReleaseActions();

    await expect(actions.checkForUpdates()).rejects.toMatchObject({
      code: "E_INTERNAL",
      details: "UPDATE_NOT_CONFIGURED",
    });
    await expect(actions.openLicenses()).rejects.toMatchObject({
      code: "E_INTERNAL",
      details: "LICENSES_NOT_CONFIGURED",
    });
  });
});
