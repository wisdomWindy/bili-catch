import { open } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTauriDirectoryPicker } from "./dialog";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

describe("Tauri directory picker", () => {
  beforeEach(() => vi.mocked(open).mockReset());

  it("requests exactly one directory and preserves string or cancel", async () => {
    const picker = createTauriDirectoryPicker();
    vi.mocked(open).mockResolvedValueOnce("D:/Media").mockResolvedValueOnce(null);

    await expect(picker.selectDirectory("D:/Current")).resolves.toBe("D:/Media");
    expect(open).toHaveBeenNthCalledWith(1, {
      directory: true,
      multiple: false,
      defaultPath: "D:/Current",
    });
    await expect(picker.selectDirectory("D:/Current")).resolves.toBeNull();
  });

  it("rejects arrays and plugin failures with a stable local error", async () => {
    const picker = createTauriDirectoryPicker();
    vi.mocked(open).mockResolvedValueOnce(["D:/One"] as never);
    await expect(picker.selectDirectory("D:/Current")).rejects.toMatchObject({
      code: "E_INTERNAL",
      details: "DIRECTORY_PICKER_FAILED",
    });

    vi.mocked(open).mockRejectedValueOnce(new Error("private operating system detail"));
    await expect(picker.selectDirectory("D:/Current")).rejects.toEqual({
      code: "E_INTERNAL",
      message: "Directory picker is unavailable",
      details: "DIRECTORY_PICKER_FAILED",
    });
  });
});
