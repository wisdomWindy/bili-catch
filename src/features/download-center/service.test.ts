import { describe, expect, it, vi } from "vitest";
import type { IpcTransport } from "../../contracts/ipc";
import { createParseVideoService } from "./service";

describe("createParseVideoService", () => {
  it("invokes parse_video with the stable input request", async () => {
    const result = {
      canonicalUrl: "https://www.bilibili.com/video/BV1xx411c7BF",
      bvid: "BV1xx411c7BF",
      aid: 170001,
      title: "Example",
      ownerName: "Uploader",
      coverUrl: "https://i0.hdslb.com/example.jpg",
      durationSeconds: 90,
      requestedPage: null,
      parts: [{ cid: 1, page: 1, title: "P1", durationSeconds: 90 }],
      qualities: [{ id: "32", label: "480P", requiresLogin: false }],
      codecs: ["avc"],
      videoVariants: [{ qualityId: "32", codec: "avc" }],
      audioFormats: ["mp3", "m4a"],
      audioBitrates: [{ id: "128", label: "128K", requiresLogin: false }],
      audioCapability: { maxLossyKbps: 128, losslessAvailable: false, hiResAvailable: false },
    } as const;
    const invoke = vi.fn().mockResolvedValue(result);
    const transport: IpcTransport = { invoke };

    await expect(createParseVideoService(transport).parseVideo("BV1xx411c7BF")).resolves.toEqual(result);
    expect(invoke).toHaveBeenCalledWith("parse_video", { input: "BV1xx411c7BF" });
  });

  it("normalizes command failures at the service boundary", async () => {
    const transport: IpcTransport = {
      invoke: vi.fn().mockRejectedValue({ code: "E003", message: "Invalid input" }),
    };

    await expect(createParseVideoService(transport).parseVideo("invalid")).rejects.toEqual({
      code: "E003",
      message: "Invalid input",
    });
  });
});
