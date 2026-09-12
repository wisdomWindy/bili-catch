import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { useTaskDraftsStore } from "./task-drafts";

describe("task draft handoff store", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("stores only download intent fields", () => {
    const store = useTaskDraftsStore();
    store.append([
      {
        canonicalUrl: "https://www.bilibili.com/video/BV1xx411c7BF",
        bvid: "BV1xx411c7BF",
        cid: 1,
        page: 1,
        partTitle: "P1",
        videoTitle: "Example video",
        partCount: 1,
        mode: "video-audio",
        outputDir: "~/Downloads/BiliCatch",
        qualityId: "32",
        codec: "avc",
        audioFormat: null,
        audioBitrateId: null,
      },
    ]);

    expect(store.items).toHaveLength(1);
    expect(store.items[0]).not.toHaveProperty("id");
    expect(store.items[0]).not.toHaveProperty("status");
    expect(store.items[0]).not.toHaveProperty("progress");
  });
});
