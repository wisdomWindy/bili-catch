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
      audioFormats: ["m4a", "mp3", "wav", ...(losslessAvailable ? ["flac" as const] : [])],
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
      autoTranscribe: true,
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
      autoTranscribe: false,
    },
    global: { plugins: [createAppI18n()] },
  });
}

describe("DownloadOptions audio mode", () => {
  it("renders fixed MP3 M4A FLAC order and output profiles", () => {
    const wrapper = render(false, false);
    const formatOptions = wrapper.get("[data-testid='audio-format']").findAll("option");
    const profileOptions = wrapper.get("[data-testid='audio-profile']").findAll("option");

    expect(formatOptions.map((option) => option.attributes("value"))).toEqual(["", "mp3", "m4a", "flac", "wav"]);
    expect(formatOptions[3].attributes("disabled")).toBeDefined();
    expect(formatOptions[3].text()).toContain("需登录");
    expect(formatOptions[4].attributes("disabled")).toBeUndefined();
    expect(profileOptions.map((option) => option.attributes("value"))).toEqual(["128", "192", "320"]);
  });

  it("distinguishes source unsupported FLAC from sign-in required", () => {
    const unsupported = render(true, false);
    expect(unsupported.get("[data-testid='audio-format'] option[value='flac']").text()).toContain("源不支持");

    const available = render(true, true);
    expect(available.get("[data-testid='audio-format'] option[value='flac']").attributes("disabled")).toBeUndefined();
  });

  it("explains that MP3 re-encodes and M4A keeps the original stream", async () => {
    const wrapper = render(true, true);
    expect(wrapper.get("[data-testid='audio-quality-hint']").text()).toContain("重新编码");

    await wrapper.setProps({ audioFormat: "m4a", audioBitrateId: "source" });
    expect(wrapper.get("[data-testid='audio-quality-hint']").text()).toContain("不转码");

    await wrapper.setProps({ audioFormat: "wav", audioBitrateId: "source" });
    expect(wrapper.get("[data-testid='audio-quality-hint']").text()).toContain("不压缩");
    expect(wrapper.get("[data-testid='audio-profile']").text()).toContain("PCM");

    await wrapper.setProps({ audioFormat: null, audioBitrateId: null });
    expect(wrapper.find("[data-testid='audio-quality-hint']").exists()).toBe(false);
  });

  it("explains a parse that came back without audio streams", () => {
    const wrapper = mount(DownloadOptions, {
      props: {
        mode: "audio-only",
        qualities: [],
        codecs: [],
        videoVariants: [],
        audioFormats: [],
        audioCapability: { maxLossyKbps: null, losslessAvailable: false, hiResAvailable: false },
        authenticated: false,
        qualityId: null,
        codec: null,
        audioFormat: null,
        audioBitrateId: null,
        autoTranscribe: false,
      },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.get("[data-testid='audio-unavailable-hint']").text()).toContain("重新解析");
    wrapper.unmount();
  });
});

describe("DownloadOptions video mode", () => {
  it("shows only codecs exposed for the selected quality", () => {
    const wrapper = renderVideo("64");
    const options = wrapper.get("[data-testid='video-codec']").findAll("option");

    expect(options.map((option) => option.attributes("value"))).toEqual(["", "hevc"]);
  });
});
