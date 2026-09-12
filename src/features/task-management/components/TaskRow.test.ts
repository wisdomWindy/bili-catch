import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import type { DownloadTask } from "../contracts";
import TaskRow from "./TaskRow.vue";

const task = (status: DownloadTask["status"]): DownloadTask => ({
  id: "task-1", revision: 1, createdAt: "2026-09-10T00:00:00Z", updatedAt: "2026-09-10T00:00:00Z",
  fileName: "A very long task filename.mp4", outputDir: "Downloads", outputPath: status === "completed" ? "C:/Downloads/file.mp4" : null,
  bvid: "BV1", cid: 1, page: 1, partTitle: "P1", mode: "video-only", qualityId: "80", codec: "avc",
  audioFormat: null, audioBitrateId: null, status, controlRequest: "none", progressPercent: 45,
  bytesDownloaded: "1048576", totalBytes: "2097152", speedBytesPerSecond: "524288", etaSeconds: 65,
  automaticRetryCount: 0, nextRetryAt: null, error: null,
});

describe("TaskRow", () => {
  it("shows accessible pause and cancel commands while downloading", () => {
    const wrapper = mount(TaskRow, { props: { task: task("downloading"), pending: false }, global: { plugins: [createAppI18n()] } });
    expect(wrapper.get("[aria-label='暂停']").attributes("type")).toBe("button");
    expect(wrapper.get("[aria-label='取消任务']").attributes("type")).toBe("button");
    expect(wrapper.get("[role='progressbar']").attributes("aria-valuenow")).toBe("45");
  });

  it("shows trusted open choices and delete for completed tasks", () => {
    const wrapper = mount(TaskRow, { props: { task: task("completed"), pending: false }, global: { plugins: [createAppI18n()] } });
    expect(wrapper.get("[aria-label='打开选项']").element.tagName).toBe("SUMMARY");
    expect(wrapper.get("[aria-label='打开文件']").attributes("type")).toBe("button");
    expect(wrapper.get("[aria-label='打开所在目录']").attributes("type")).toBe("button");
    expect(wrapper.get("[aria-label='删除记录']").attributes("type")).toBe("button");
  });

  it("localizes indeterminate progress semantics", () => {
    const i18n = createAppI18n();
    i18n.global.locale.value = "en-US";
    const wrapper = mount(TaskRow, { props: { task: task("processing"), pending: false }, global: { plugins: [i18n] } });
    expect(wrapper.get("[role='progressbar']").attributes("aria-valuetext")).toBe("Processing");
  });

  it("shows the output profile and never exposes backend error text or details", () => {
    const failed = {
      ...task("failed"),
      mode: "audio-only" as const,
      codec: null,
      qualityId: null,
      audioFormat: "mp3" as const,
      audioBitrateId: "320",
      error: { code: "E009" as const, message: "secret signed URL", details: "https://evil.example/token" },
    };
    const wrapper = mount(TaskRow, {
      props: { task: failed, pending: false },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.get(".task-row__file > span").text()).toContain("仅音频 · MP3 320K");
    expect(wrapper.get(".task-row__error").text()).toBe("网络中断，已保留下载进度");
    expect(wrapper.html()).not.toContain("secret signed URL");
    expect(wrapper.html()).not.toContain("evil.example");
  });

  it("keeps the safe network message visible while an automatic retry is queued", () => {
    const retrying = {
      ...task("queued"),
      error: { code: "E009" as const, message: "raw network failure" },
      automaticRetryCount: 1,
      nextRetryAt: "2026-09-10T00:00:01Z",
    };
    const wrapper = mount(TaskRow, {
      props: { task: retrying, pending: false },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.get(".task-row__error").text()).toBe("网络中断，已保留下载进度");
    expect(wrapper.html()).not.toContain("raw network failure");
  });
});
