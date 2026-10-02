import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createDeferredReleaseActions, createTauriReleaseActions } from "./release";

vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));

function updateFixture(version: string): Update {
  return {
    available: true,
    currentVersion: "0.1.16",
    version,
    date: "2026-10-02T00:00:00Z",
    body: "Fixture release",
    rawJson: { version },
    download: vi.fn(),
    install: vi.fn(),
    downloadAndInstall: vi.fn(),
    close: vi.fn(),
  } as unknown as Update;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((onResolve) => {
    resolve = onResolve;
  });
  return { promise, resolve };
}

beforeEach(() => {
  vi.mocked(check).mockReset();
  vi.mocked(getVersion).mockReset();
  vi.mocked(relaunch).mockReset();
});

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

describe("Tauri release actions", () => {
  it("keeps the checked update bound across repeated consumers until installation", async () => {
    const confirmed = updateFixture("0.2.0");
    const later = updateFixture("0.3.0");
    const firstCheck = deferred<Update | null>();
    vi.mocked(check).mockReturnValueOnce(firstCheck.promise).mockResolvedValueOnce(later);
    vi.mocked(relaunch).mockResolvedValue(undefined);
    const actions = createTauriReleaseActions();

    const firstConsumer = actions.checkForUpdates();
    const secondConsumer = actions.checkForUpdates();
    firstCheck.resolve(confirmed);

    await expect(firstConsumer).resolves.toMatchObject({
      status: "available",
      latestVersion: "0.2.0",
    });
    await expect(secondConsumer).resolves.toMatchObject({
      status: "available",
      latestVersion: "0.2.0",
    });
    expect(check).toHaveBeenCalledOnce();

    await actions.installUpdate();

    expect(confirmed.downloadAndInstall).toHaveBeenCalledOnce();
    expect(later.downloadAndInstall).not.toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalledOnce();
  });
});
