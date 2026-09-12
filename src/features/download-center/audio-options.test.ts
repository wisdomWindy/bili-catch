import { describe, expect, it } from "vitest";
import {
  audioProfileOptions,
  defaultAudioProfile,
  isAudioProfileValid,
} from "./audio-options";

describe("audio output profiles", () => {
  it("keeps output encoding profiles separate from source bitrates", () => {
    expect(audioProfileOptions("mp3")).toEqual([
      { id: "128", label: "128K" },
      { id: "192", label: "192K" },
      { id: "320", label: "320K" },
    ]);
    expect(audioProfileOptions("m4a")).toEqual([{ id: "source", label: "Original" }]);
    expect(audioProfileOptions("flac")).toEqual([{ id: "lossless", label: "Lossless" }]);
  });

  it("uses the approved defaults and rejects cross-format profile ids", () => {
    expect(defaultAudioProfile("mp3")).toBe("320");
    expect(defaultAudioProfile("m4a")).toBe("source");
    expect(defaultAudioProfile("flac")).toBe("lossless");
    expect(isAudioProfileValid("mp3", "320")).toBe(true);
    expect(isAudioProfileValid("mp3", "source")).toBe(false);
    expect(isAudioProfileValid("m4a", "192")).toBe(false);
    expect(isAudioProfileValid("flac", null)).toBe(false);
  });
});
