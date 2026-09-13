import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ParseVideoResult } from "./contracts";
import type { ParseVideoService } from "./service";
import { useDownloadCenterStore } from "./store";

const fixture: ParseVideoResult = {
  canonicalUrl: "https://www.bilibili.com/video/BV1xx411c7BF",
  bvid: "BV1xx411c7BF",
  aid: 170001,
  title: "Fixture video",
  ownerName: "Fixture owner",
  coverUrl: "https://i0.hdslb.com/fixture.jpg",
  durationSeconds: 183,
  requestedPage: 2,
  parts: [
    { cid: 1001, page: 1, title: "Opening", durationSeconds: 90 },
    { cid: 1002, page: 2, title: "Deep dive", durationSeconds: 93 },
  ],
  qualities: [
    { id: "120", label: "4K", requiresLogin: true },
    { id: "80", label: "1080P", requiresLogin: false },
  ],
  codecs: ["avc", "hevc"],
  videoVariants: [
    { qualityId: "80", codec: "avc" },
    { qualityId: "80", codec: "hevc" },
  ],
  audioFormats: ["m4a", "mp3"],
  audioBitrates: [{ id: "192", label: "192 kbps", requiresLogin: false }],
  audioCapability: { maxLossyKbps: 192, losslessAvailable: false, hiResAvailable: false },
};

function service(result = fixture): ParseVideoService {
  return { parseVideo: vi.fn().mockResolvedValue(result) };
}

describe("download center store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("applies requested part and first available capabilities", async () => {
    const store = useDownloadCenterStore();
    await store.parse(service(), " BV1xx411c7BF ");

    expect(store.status).toBe("success");
    expect(store.selectedCids).toEqual([1002]);
    expect(store.qualityId).toBe("80");
    expect(store.canEnqueue).toBe(true);
  });

  it("supports all, inverse and empty part selections", async () => {
    const store = useDownloadCenterStore();
    await store.parse(service(), "BV1xx411c7BF");
    store.selectAll();
    expect(store.selectedCids).toEqual([1001, 1002]);
    store.invertSelection();
    expect(store.selectedCids).toEqual([]);
    expect(store.canEnqueue).toBe(false);
  });

  it("clears mutually exclusive fields when mode changes", async () => {
    const store = useDownloadCenterStore();
    await store.parse(service(), "BV1xx411c7BF");
    store.setMode("audio-only");
    expect([store.qualityId, store.codec]).toEqual([null, null]);
    expect([store.audioFormat, store.audioBitrateId]).toEqual(["m4a", "source"]);
    store.setMode("video-audio");
    expect([store.audioFormat, store.audioBitrateId]).toEqual([null, null]);
    store.setMode("video-only");
    expect([store.audioFormat, store.audioBitrateId]).toEqual([null, null]);
  });

  it("builds one mutually exclusive draft for every selected part", async () => {
    const store = useDownloadCenterStore();
    await store.parse(service(), "BV1xx411c7BF");
    store.selectAll();
    const drafts = store.buildDrafts();
    expect(drafts).toHaveLength(2);
    expect(drafts[0]).toMatchObject({
      videoTitle: "Fixture video",
      partCount: 2,
      cid: 1001,
      mode: "video-audio",
      qualityId: "80",
      codec: "avc",
      audioFormat: null,
      audioBitrateId: null,
    });

    store.setMode("audio-only");
    expect(store.buildDrafts()[0]).toMatchObject({
      qualityId: null,
      codec: null,
      audioFormat: "m4a",
      audioBitrateId: "source",
    });
  });

  it("ignores stale responses from an older request", async () => {
    let releaseFirst: ((result: ParseVideoResult) => void) | undefined;
    const pending = new Promise<ParseVideoResult>((resolve) => { releaseFirst = resolve; });
    const parseVideo = vi.fn()
      .mockReturnValueOnce(pending)
      .mockResolvedValueOnce({ ...fixture, title: "Latest" });
    const store = useDownloadCenterStore();

    const first = store.parse({ parseVideo }, "BV1xx411c7BF");
    await store.parse({ parseVideo }, "av170001");
    releaseFirst?.({ ...fixture, title: "Stale" });
    await first;

    expect(store.result?.title).toBe("Latest");
  });

  it("refreshes the result for every explicit parse and clear invalidates pending work", async () => {
    const parseVideo = vi.fn()
      .mockResolvedValueOnce(fixture)
      .mockResolvedValueOnce({
        ...fixture,
        title: "Refreshed fixture",
        requestedPage: 1,
      });
    const store = useDownloadCenterStore();
    await store.parse({ parseVideo }, "BV1xx411c7BF");
    await store.parse({ parseVideo }, "BV1xx411c7BF");

    expect(parseVideo).toHaveBeenCalledTimes(2);
    expect(store.result?.title).toBe("Refreshed fixture");
    expect(store.selectedCids).toEqual([1001]);
    store.clear();
    expect(store.status).toBe("idle");
    expect(store.result).toBeNull();
  });

  it("clears parse cache and falls back an unavailable video pair after auth changes", async () => {
    const store = useDownloadCenterStore();
    const parseVideo = vi.fn()
      .mockResolvedValueOnce(fixture)
      .mockResolvedValueOnce({
        ...fixture,
        videoVariants: [{ qualityId: "80", codec: "avc" }],
      });
    await store.parse({ parseVideo }, "BV1xx411c7BF");
    store.codec = "hevc";

    await store.refreshForAuthChange({ parseVideo }, false);

    expect(parseVideo).toHaveBeenCalledTimes(2);
    expect(store.codec).toBe("avc");
    expect(store.consumeVideoFallbackNotice()).toBe(true);
  });

  it("refreshes equivalent BV ID and URL inputs independently", async () => {
    const mock = service();
    const store = useDownloadCenterStore();
    await store.parse(mock, "BV1xx411c7BF");
    await store.parse(mock, "https://www.bilibili.com/video/BV1xx411c7BF");
    expect(mock.parseVideo).toHaveBeenCalledTimes(2);
  });

  it("rejects a login-only capability even when selected directly", async () => {
    const store = useDownloadCenterStore();
    await store.parse(service(), "BV1xx411c7BF");
    store.qualityId = "120";
    expect(store.canEnqueue).toBe(false);
    expect(store.buildDrafts()).toEqual([]);
  });

  it("rejects a quality and codec cross product absent from video variants", () => {
    const store = useDownloadCenterStore();
    store.applyResult({
      ...fixture,
      qualities: [...fixture.qualities, { id: "64", label: "720P", requiresLogin: false }],
      videoVariants: [
        ...fixture.videoVariants,
        { qualityId: "64", codec: "hevc" },
      ],
    });

    store.qualityId = "64";
    store.codec = "avc";

    expect(store.canEnqueue).toBe(false);
    expect(store.buildDrafts()).toEqual([]);
  });

  it("reconciles the codec when the selected quality changes", () => {
    const store = useDownloadCenterStore();
    store.applyResult({
      ...fixture,
      qualities: [...fixture.qualities, { id: "64", label: "720P", requiresLogin: false }],
      videoVariants: [
        ...fixture.videoVariants,
        { qualityId: "64", codec: "hevc" },
      ],
    });

    store.setVideoQuality("64");

    expect([store.qualityId, store.codec]).toEqual(["64", "hevc"]);
    expect(store.canEnqueue).toBe(true);
  });

  it("keeps video mode selected but not enqueueable when no variants exist", () => {
    const store = useDownloadCenterStore();
    store.applyResult({ ...fixture, videoVariants: [] });

    expect(store.mode).toBe("video-audio");
    expect([store.qualityId, store.codec]).toEqual([null, null]);
    expect(store.canEnqueue).toBe(false);
  });

  it("applies configured download and media defaults on the next result", () => {
    const store = useDownloadCenterStore();
    store.configureDefaults({
      downloadDirectory: "D:/Media",
      defaultVideoQuality: "80",
      defaultAudioFormat: "mp3",
    });

    expect(store.outputDir).toBe("D:/Media");
    store.applyResult(fixture);
    expect(store.qualityId).toBe("80");
    store.setMode("audio-only");
    expect(store.audioFormat).toBe("mp3");
  });

  it("falls back when a configured media default is unavailable", () => {
    const store = useDownloadCenterStore();
    store.configureDefaults({
      downloadDirectory: "D:/Media",
      defaultVideoQuality: "127",
      defaultAudioFormat: "flac",
    });

    store.applyResult(fixture);
    expect(store.qualityId).toBe("80");
    store.setMode("audio-only");
    expect(store.audioFormat).toBe("m4a");
  });

  it("does not overwrite current media choices and uses new defaults after clear", () => {
    const store = useDownloadCenterStore();
    store.applyResult(fixture);
    store.qualityId = "80";
    store.setMode("audio-only");
    store.audioFormat = "m4a";

    store.configureDefaults({
      downloadDirectory: "D:/Next",
      defaultVideoQuality: "64",
      defaultAudioFormat: "mp3",
    });
    expect(store.audioFormat).toBe("m4a");

    store.clear();
    store.applyResult({
      ...fixture,
      qualities: [
        { id: "64", label: "720P", requiresLogin: false },
        ...fixture.qualities,
      ],
      videoVariants: [
        { qualityId: "64", codec: "avc" },
        ...fixture.videoVariants,
      ],
    });
    expect(store.qualityId).toBe("64");
    store.setMode("audio-only");
    expect(store.audioFormat).toBe("mp3");
  });

  it("keeps output profiles independent from the lower source bitrate", () => {
    const store = useDownloadCenterStore();
    store.applyResult({
      ...fixture,
      audioFormats: ["m4a", "mp3"],
      audioBitrates: [{ id: "64", label: "64 kbps", requiresLogin: false }],
      audioCapability: { maxLossyKbps: 64, losslessAvailable: false, hiResAvailable: false },
    });

    store.setMode("audio-only", false);
    store.setAudioFormat("mp3", false);

    expect(store.audioBitrateId).toBe("320");
    expect(store.canEnqueue).toBe(true);
    expect(store.buildDrafts()[0].audioBitrateId).toBe("320");
  });

  it("remembers the last MP3 profile across format switches", () => {
    const store = useDownloadCenterStore();
    store.applyResult(fixture);
    store.setMode("audio-only", false);
    store.setAudioFormat("mp3", false);
    store.setAudioProfile("128");
    store.setAudioFormat("m4a", false);
    store.setAudioFormat("mp3", false);

    expect(store.audioBitrateId).toBe("128");
  });

  it("enables real lossless only for authenticated users and falls back once", () => {
    const store = useDownloadCenterStore();
    store.applyResult({
      ...fixture,
      audioFormats: ["m4a", "mp3", "flac"],
      audioCapability: { maxLossyKbps: 192, losslessAvailable: true, hiResAvailable: true },
    });
    store.setMode("audio-only", true);
    store.setAudioFormat("flac", true);

    expect(store.audioBitrateId).toBe("lossless");
    expect(store.canEnqueue).toBe(true);

    store.reconcileAudioAvailability(false);
    expect([store.audioFormat, store.audioBitrateId]).toEqual(["m4a", "source"]);
    expect(store.consumeAudioFallbackNotice()).toBe(true);
    expect(store.consumeAudioFallbackNotice()).toBe(false);
  });

  it("falls back from an unavailable FLAC default when audio mode opens", () => {
    const store = useDownloadCenterStore();
    store.configureDefaults({
      downloadDirectory: "D:/Media",
      defaultVideoQuality: "80",
      defaultAudioFormat: "flac",
    });
    store.applyResult(fixture);

    store.setMode("audio-only", false);

    expect([store.audioFormat, store.audioBitrateId]).toEqual(["m4a", "source"]);
    expect(store.consumeAudioFallbackNotice()).toBe(true);
  });
});
