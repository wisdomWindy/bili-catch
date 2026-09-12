import { describe, expect, it } from "vitest";
import type { MediaOption, VideoVariant } from "./contracts";
import {
  availableCodecsForQuality,
  isVideoVariantValid,
  selectVideoVariant,
} from "./video-options";

const qualities: MediaOption[] = [
  { id: "120", label: "4K", requiresLogin: true },
  { id: "80", label: "1080P", requiresLogin: false },
  { id: "64", label: "720P", requiresLogin: false },
];

const variants: VideoVariant[] = [
  { qualityId: "80", codec: "av1" },
  { qualityId: "64", codec: "hevc" },
  { qualityId: "80", codec: "avc" },
  { qualityId: "80", codec: "hevc" },
];

describe("video variant selection", () => {
  it("filters codecs by quality and applies the stable codec order", () => {
    expect(availableCodecsForQuality(variants, "80")).toEqual(["avc", "hevc", "av1"]);
    expect(availableCodecsForQuality(variants, "64")).toEqual(["hevc"]);
  });

  it("rejects quality and codec cross products that the source did not expose", () => {
    expect(isVideoVariantValid(variants, "80", "hevc")).toBe(true);
    expect(isVideoVariantValid(variants, "64", "avc")).toBe(false);
  });

  it("prefers the configured quality and preserves its valid codec", () => {
    expect(selectVideoVariant(qualities, variants, "80", "hevc")).toEqual({
      qualityId: "80",
      codec: "hevc",
    });
  });

  it("falls back to the first available quality and preserves a supported codec", () => {
    expect(selectVideoVariant(qualities, variants, "127", "av1")).toEqual({
      qualityId: "80",
      codec: "av1",
    });
  });

  it("does not invent a selection when no variant is available", () => {
    expect(selectVideoVariant(qualities, [], "80", "avc")).toBeNull();
  });
});
