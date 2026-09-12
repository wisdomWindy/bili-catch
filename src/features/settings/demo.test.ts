import { describe, expect, it } from "vitest";
import { createDemoSettingsRuntime } from "./demo";

describe("demo settings runtime", () => {
  it("persists patches in memory without native plugins", async () => {
    const runtime = createDemoSettingsRuntime("#/?demo=1");
    const initial = await runtime.service.getSnapshot();
    const updated = await runtime.service.update({ field: "theme", value: "dark" });

    expect(initial.revision).toBe(0);
    expect(updated.revision).toBe(1);
    expect((await runtime.service.getSnapshot()).values.theme).toBe("dark");
    await expect(runtime.directoryPicker.selectDirectory("D:/Current")).resolves.toBe("D:/BiliCatch Demo");
    await expect(runtime.releaseActions.checkForUpdates()).resolves.toMatchObject({ status: "latest" });
  });

  it("exposes deterministic load, save, picker and update states from hash flags", async () => {
    await expect(
      createDemoSettingsRuntime("#/?demo=1&theme=dark&locale=en-US").service.getSnapshot(),
    ).resolves.toMatchObject({ values: { theme: "dark", locale: "en-US" } });
    let loadingSettled = false;
    void createDemoSettingsRuntime("#/?demo=1&settings=loading")
      .service.getSnapshot()
      .finally(() => {
        loadingSettled = true;
      });
    await Promise.resolve();
    expect(loadingSettled).toBe(false);

    await expect(
      createDemoSettingsRuntime("#/?demo=1&settings=load-error").service.getSnapshot(),
    ).rejects.toMatchObject({ details: "SETTINGS_STORE_UNAVAILABLE" });
    await expect(
      createDemoSettingsRuntime("#/?demo=1&settings=save-error").service.update({
        field: "locale",
        value: "en-US",
      }),
    ).rejects.toMatchObject({ details: "SETTINGS_STORE_UNAVAILABLE" });
    await expect(
      createDemoSettingsRuntime("#/?demo=1&picker=cancel").directoryPicker.selectDirectory("D:/Current"),
    ).resolves.toBeNull();
    await expect(
      createDemoSettingsRuntime("#/?demo=1&update=available").releaseActions.checkForUpdates(),
    ).resolves.toMatchObject({ status: "available", latestVersion: "0.2.0" });
    await expect(
      createDemoSettingsRuntime("#/?demo=1&update=error").releaseActions.checkForUpdates(),
    ).rejects.toMatchObject({ details: "UPDATE_CHECK_FAILED" });
  });
});
