import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

function releaseAssetUrl(assets, name, version) {
  const matches = assets.filter((asset) => asset.name === name);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one release asset named ${name}`);
  }

  const url = new URL(matches[0].browser_download_url);
  const expectedPath = `/releases/download/v${version}/${name}`;
  if (url.protocol !== "https:" || url.hostname !== "github.com" || !url.pathname.endsWith(expectedPath)) {
    throw new Error(`${name} must expose a public GitHub release download URL`);
  }
  return url.toString();
}

export function rewriteUpdaterManifest(manifest, assets) {
  if (!manifest?.version || !manifest?.platforms) {
    throw new Error("Updater manifest is missing its version or platforms");
  }

  const rewritten = structuredClone(manifest);
  const mappings = [
    {
      name: `BiliCatch_${manifest.version}_Windows_x64-setup.exe`,
      platforms: ["windows-x86_64", "windows-x86_64-nsis"],
    },
    {
      name: `BiliCatch_${manifest.version}_macOS_Intel.app.tar.gz`,
      platforms: ["darwin-x86_64", "darwin-x86_64-app"],
    },
    {
      name: `BiliCatch_${manifest.version}_macOS_AppleSilicon.app.tar.gz`,
      platforms: ["darwin-aarch64", "darwin-aarch64-app"],
    },
  ];

  for (const mapping of mappings) {
    const url = releaseAssetUrl(assets, mapping.name, manifest.version);
    const primary = mapping.platforms[0];
    if (!rewritten.platforms[primary]) {
      throw new Error(`Updater manifest is missing ${primary}`);
    }
    for (const platform of mapping.platforms) {
      if (rewritten.platforms[platform]) {
        rewritten.platforms[platform].url = url;
      }
    }
  }

  return rewritten;
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  const [manifestPath, assetsPath] = process.argv.slice(2);
  if (!manifestPath || !assetsPath) {
    throw new Error("Usage: node rewrite-updater-manifest.mjs <latest.json> <release-assets.json>");
  }
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const assets = JSON.parse(fs.readFileSync(assetsPath, "utf8"));
  const rewritten = rewriteUpdaterManifest(manifest, assets);
  fs.writeFileSync(manifestPath, `${JSON.stringify(rewritten, null, 2)}\n`, "utf8");
}
