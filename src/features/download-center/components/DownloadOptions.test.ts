import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import DownloadOptions from "./DownloadOptions.vue";

function render(authenticated: boolean, losslessAvailable: boolean) {
  return mount(DownloadOptions, {
    props: {
      mode: "audio-only",
      qualities: [],
      codecs: [],
      videoVariants: [],
      audioFormats: ["m4a", "mp3", ...(losslessAvailable ? ["flac" as const] : [])],
      audioCapability: {
        maxLossyKbps: 64,
        losslessAvailable,
        hiResAvailable: losslessAvailable,
      },
      authenticated,
      qualityId: null,
      codec: null,
      audioFormat: "mp3",
      audioBitrateId: "320",
    },
    global: { plugins: [createAppI18n()] },
  });
}

function renderVideo(qualityId: string) {
  return mount(DownloadOptions, {
    props: {
      mode: "video-only",
      qualities: [
        { id: "80", label: "1080P", requiresLogin: false },
        { id: "64", label: "720P", requiresLogin: false },
      ],
      codecs: ["avc", "hevc"],
      videoVariants: [
        { qualityId: "80", codec: "avc" },
        { qualityId: "64", codec: "hevc" },
      ],
      audioFormats: [],
      audioCapability: { maxLossyKbps: null, losslessAvailable: false, hiResAvailable: false },
      authenticated: false,
      qualityId,
      codec: qualityId === "80" ? "avc" : "hevc",
      audioFormat: null,
      audioBitrateId: null,
    },
    global: { plugins: [createAppI18n()] },
  });
}

describe("DownloadOptions audio mode", () => {
  it("renders fixed MP3 M4A FLAC order and output profiles", () => {
    const wrapper = render(false, false);
    const formatOptions = wrapper.get("[data-testid='audio-format']").findAll("option");
    const profileOptions = wrapper.get("[data-testid='audio-profile']").findAll("option");

    expect(formatOptions.map((option) => option.attributes("value"))).toEqual(["", "mp3", "m4a", "flac"]);
    expect(formatOptions[3].attributes("disabled")).toBeDefined();
    expect(formatOptions[3].text()).toContain("需登录");
    expect(profileOptions.map((option) => option.attributes("value"))).toEqual(["128", "192", "320"]);
  });

  it("distinguishes source unsupported FLAC from sign-in required", () => {
    const unsupported = render(true, false);
    expect(unsupported.get("[data-testid='audio-format'] option[value='flac']").text()).toContain("源不支持");

    const available = render(true, true);
    expect(available.get("[data-testid='audio-format'] option[value='flac']").attributes("disabled")).toBeUndefined();
  });
});

describe("DownloadOptions video mode", () => {
  it("shows only codecs exposed for the selected quality", () => {
    const wrapper = renderVideo("64");
    const options = wrapper.get("[data-testid='video-codec']").findAll("option");

    expect(options.map((option) => option.attributes("value"))).toEqual(["", "hevc"]);
  });
});
