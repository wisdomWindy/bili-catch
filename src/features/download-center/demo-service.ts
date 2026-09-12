import type { ParseVideoService } from "./service";

export function createDemoParseVideoService(): ParseVideoService {
  return {
    async parseVideo(input) {
      await new Promise((resolve) => window.setTimeout(resolve, 250));
      if (/^av404$/i.test(input)) {
        throw { code: "E004", message: "Demo video is unavailable" };
      }
      return {
        canonicalUrl: "https://www.bilibili.com/video/BV1xx411c7BF",
        bvid: "BV1xx411c7BF",
        aid: 170001,
        title: "从零构建桌面媒体工作流：解析、选择与任务编排",
        ownerName: "BiliCatch Lab",
        coverUrl: "",
        durationSeconds: 1482,
        requestedPage: 2,
        parts: [
          { cid: 1001, page: 1, title: "需求与数据边界", durationSeconds: 312 },
          { cid: 1002, page: 2, title: "解析链路设计", durationSeconds: 408 },
          { cid: 1003, page: 3, title: "媒体能力选择", durationSeconds: 366 },
          { cid: 1004, page: 4, title: "任务交接与验证", durationSeconds: 396 },
        ],
        qualities: [
          { id: "120", label: "4K 超清", requiresLogin: true },
          { id: "80", label: "1080P 高清", requiresLogin: false },
          { id: "64", label: "720P 高清", requiresLogin: false },
        ],
        codecs: ["avc", "hevc", "av1"],
        videoVariants: [
          { qualityId: "80", codec: "avc" },
          { qualityId: "80", codec: "hevc" },
          { qualityId: "80", codec: "av1" },
          { qualityId: "64", codec: "avc" },
        ],
        audioFormats: ["m4a", "mp3", "flac"],
        audioBitrates: [
          { id: "192", label: "192 kbps", requiresLogin: false },
          { id: "128", label: "128 kbps", requiresLogin: false },
        ],
        audioCapability: { maxLossyKbps: 192, losslessAvailable: true, hiResAvailable: true },
      };
    },
  };
}
