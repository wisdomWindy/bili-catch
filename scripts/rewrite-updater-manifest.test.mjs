import { describe, expect, it } from "vitest";
import { rewriteUpdaterManifest } from "./rewrite-updater-manifest.mjs";

const releaseBase = "https://github.com/wisdomWindy/bili-catch/releases/download/v0.1.14";

function manifest() {
  return {
    version: "0.1.14",
    notes: "",
    platforms: {
      "windows-x86_64": { signature: "windows-signature", url: "https://api.github.com/assets/1" },
      "windows-x86_64-nsis": { signature: "windows-signature", url: "https://api.github.com/assets/1" },
      "darwin-x86_64": { signature: "intel-signature", url: "https://api.github.com/assets/2" },
      "darwin-x86_64-app": { signature: "intel-signature", url: "https://api.github.com/assets/2" },
      "darwin-aarch64": { signature: "arm-signature", url: "https://api.github.com/assets/3" },
      "darwin-aarch64-app": { signature: "arm-signature", url: "https://api.github.com/assets/3" },
    },
  };
}

function assets() {
  return [
    {
      name: "BiliCatch_0.1.14_Windows_x64-setup.exe",
      browser_download_url: `${releaseBase}/BiliCatch_0.1.14_Windows_x64-setup.exe`,
    },
    {
      name: "BiliCatch_0.1.14_macOS_Intel.app.tar.gz",
      browser_download_url: `${releaseBase}/BiliCatch_0.1.14_macOS_Intel.app.tar.gz`,
    },
    {
      name: "BiliCatch_0.1.14_macOS_AppleSilicon.app.tar.gz",
      browser_download_url: `${releaseBase}/BiliCatch_0.1.14_macOS_AppleSilicon.app.tar.gz`,
    },
  ];
}

describe("updater manifest rewriting", () => {
  it("replaces draft asset URLs with stable public release download URLs", () => {
    const draftAssets = assets().map((asset) => ({
      ...asset,
      browser_download_url: asset.browser_download_url.replace(
        "/download/v0.1.14/",
        "/download/untagged-draft-id/",
      ),
    }));
    const rewritten = rewriteUpdaterManifest(manifest(), draftAssets, "wisdomWindy/bili-catch");

    expect(rewritten.platforms["windows-x86_64"].url).toBe(
      `${releaseBase}/BiliCatch_0.1.14_Windows_x64-setup.exe`,
    );
    expect(rewritten.platforms["windows-x86_64-nsis"].url).toBe(
      `${releaseBase}/BiliCatch_0.1.14_Windows_x64-setup.exe`,
    );
    expect(rewritten.platforms["darwin-x86_64"].url).toBe(
      `${releaseBase}/BiliCatch_0.1.14_macOS_Intel.app.tar.gz`,
    );
    expect(rewritten.platforms["darwin-aarch64"].url).toBe(
      `${releaseBase}/BiliCatch_0.1.14_macOS_AppleSilicon.app.tar.gz`,
    );
    expect(rewritten.platforms["windows-x86_64"].signature).toBe("windows-signature");
  });

  it("rejects an invalid repository identifier", () => {
    expect(() => rewriteUpdaterManifest(manifest(), assets(), "https://example.com/repo")).toThrow(
      "owner/name",
    );
  });
});
