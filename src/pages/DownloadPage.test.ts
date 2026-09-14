import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { defineComponent } from "vue";
import { NMessageProvider } from "naive-ui";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory } from "vue-router";
import DownloadPage from "./DownloadPage.vue";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import {
  clipboardReaderKey,
  downloadNotifierKey,
  parseVideoServiceKey,
} from "../features/download-center/injection";
import type { ParseVideoResult } from "../features/download-center/contracts";
import { useDownloadCenterStore } from "../features/download-center/store";
import { useTaskDraftsStore } from "../stores/task-drafts";
import { useAuthStore } from "../features/authentication/store";

const mountedWrappers: Array<{ unmount(): void }> = [];
let readClipboard = vi.fn().mockResolvedValue("");

afterEach(() => {
  mountedWrappers.splice(0).forEach((wrapper) => wrapper.unmount());
  readClipboard = vi.fn().mockResolvedValue("");
});

const result: ParseVideoResult = {
  canonicalUrl: "https://www.bilibili.com/video/BV1xx411c7BF",
  bvid: "BV1xx411c7BF",
  aid: 170001,
  title: "Fixture video",
  ownerName: "Fixture owner",
  coverUrl: "",
  durationSeconds: 183,
  requestedPage: 1,
  parts: [{ cid: 1001, page: 1, title: "Opening", durationSeconds: 183 }],
  qualities: [{ id: "80", label: "1080P", requiresLogin: false }],
  codecs: ["avc"],
  videoVariants: [{ qualityId: "80", codec: "avc" }],
  audioFormats: ["m4a"],
  audioBitrates: [{ id: "192", label: "192 kbps", requiresLogin: false }],
  audioCapability: { maxLossyKbps: 192, losslessAvailable: false, hiResAvailable: false },
};

function render(
  parseVideo = vi.fn().mockResolvedValue(result),
  sessionInfo = vi.fn(),
  configure?: (pinia: ReturnType<typeof createPinia>) => void,
) {
  const pinia = createPinia();
  setActivePinia(pinia);
  configure?.(pinia);
  const router = createAppRouter(createMemoryHistory());
  const Host = defineComponent({
    components: { DownloadPage, NMessageProvider },
    template: "<NMessageProvider><DownloadPage /></NMessageProvider>",
  });
  const wrapper = mount(Host, {
    global: {
      plugins: [pinia, router, createAppI18n()],
      provide: {
        [parseVideoServiceKey as symbol]: { parseVideo },
        [clipboardReaderKey as symbol]: { readText: () => readClipboard() },
        [downloadNotifierKey as symbol]: { success: vi.fn(), info: sessionInfo },
      },
    },
  });
  mountedWrappers.push(wrapper);
  return { wrapper, parseVideo, router, pinia, sessionInfo };
}

describe("DownloadPage", () => {
  it("parses a valid value and exposes the complete configuration workflow", async () => {
    const { wrapper, parseVideo } = render();
    await wrapper.get("#video-input").setValue("BV1xx411c7BF");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(parseVideo).toHaveBeenCalledWith("BV1xx411c7BF");
    expect(wrapper.get("#video-title").text()).toBe("Fixture video");
    expect(wrapper.get(".cover-frame svg").attributes("aria-hidden")).toBe("true");
    expect(wrapper.get("#parts-title").text()).toBe("选择分 P");
    expect(wrapper.get("fieldset").attributes("class")).toContain("mode-selector");
    expect(wrapper.get("[aria-label='下载操作']").element.tagName).toBe("SECTION");
  });

  it("keeps invalid input visible without invoking the service", async () => {
    const { wrapper, parseVideo } = render();
    await wrapper.get("#video-input").setValue("https://example.com/video/BV1xx411c7BF");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(parseVideo).not.toHaveBeenCalled();
    expect(wrapper.get("#video-input").attributes("aria-invalid")).toBe("true");
    expect(wrapper.get("[role='alert']").text()).toContain("链接无效");
    await wrapper.get("[role='alert'] .error-actions button:last-child").trigger("click");
    expect((wrapper.get("#video-input").element as HTMLInputElement).value).toBe("");
  });

  it("auto-submits plain text pasted into the input", async () => {
    const { wrapper, parseVideo } = render();
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", {
      value: { getData: () => "av170001" },
    });
    wrapper.get("#video-input").element.dispatchEvent(event);
    await flushPromises();

    expect(parseVideo).toHaveBeenCalledWith("av170001");
    expect((wrapper.get("#video-input").element as HTMLInputElement).value).toBe("av170001");
  });

  it("reads and parses a new valid clipboard value when the window regains focus", async () => {
    const readText = vi.fn()
      .mockResolvedValueOnce("not a Bilibili address")
      .mockResolvedValueOnce("https://www.bilibili.com/video/BV1xx411c7BF");
    readClipboard = readText;
    const { wrapper, parseVideo } = render();
    await flushPromises();

    window.dispatchEvent(new Event("focus"));
    await flushPromises();

    expect((wrapper.get("#video-input").element as HTMLInputElement).value)
      .toBe("https://www.bilibili.com/video/BV1xx411c7BF");
    expect(wrapper.get("#video-title").text()).toBe("Fixture video");
    expect(parseVideo).toHaveBeenCalledOnce();
    expect(parseVideo).toHaveBeenCalledWith("https://www.bilibili.com/video/BV1xx411c7BF");
  });

  it("does not parse the same clipboard value again on repeated focus", async () => {
    const readText = vi.fn().mockResolvedValue("av170001");
    readClipboard = readText;
    const { wrapper, parseVideo } = render();
    await flushPromises();

    window.dispatchEvent(new Event("focus"));
    await flushPromises();

    expect((wrapper.get("#video-input").element as HTMLInputElement).value).toBe("av170001");
    expect(parseVideo).toHaveBeenCalledTimes(1);
  });

  it("ignores an older clipboard read that finishes after a focus read", async () => {
    let resolveInitialRead: ((value: string) => void) | undefined;
    const initialRead = new Promise<string>((resolve) => {
      resolveInitialRead = resolve;
    });
    const readText = vi.fn()
      .mockReturnValueOnce(initialRead)
      .mockResolvedValueOnce("BV1xx411c7BF");
    readClipboard = readText;
    const { wrapper, parseVideo } = render();

    window.dispatchEvent(new Event("focus"));
    await flushPromises();
    resolveInitialRead?.("av170001");
    await flushPromises();

    expect((wrapper.get("#video-input").element as HTMLInputElement).value).toBe("BV1xx411c7BF");
    expect(parseVideo).toHaveBeenCalledOnce();
    expect(parseVideo).toHaveBeenCalledWith("BV1xx411c7BF");
  });

  it("auto-submits valid plain text dropped on the input panel", async () => {
    const { wrapper, parseVideo } = render();
    const event = new Event("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: { getData: () => "BV1xx411c7BF" } });
    wrapper.get(".input-panel").element.dispatchEvent(event);
    await flushPromises();

    expect(parseVideo).toHaveBeenCalledWith("BV1xx411c7BF");
    expect(wrapper.get("#video-input").attributes("aria-describedby")).toBe("download-input-hint");
  });

  it("offers the login route for E005 and preserves the submitted input", async () => {
    const { wrapper, router } = render(vi.fn().mockRejectedValue({ code: "E005", message: "private media URL" }));
    await wrapper.get("#video-input").setValue("BV1xx411c7BF");
    await wrapper.get("form").trigger("submit");
    await flushPromises();
    await wrapper.get("[role='alert'] .error-actions button:nth-child(2)").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/login");
    expect((wrapper.get("#video-input").element as HTMLInputElement).value).toBe("BV1xx411c7BF");
    expect(wrapper.text()).not.toContain("private media URL");
  });

  it("hands one draft per selected part to the task module and navigates", async () => {
    const { wrapper, router } = render();
    await wrapper.get("#video-input").setValue("BV1xx411c7BF");
    await wrapper.get("form").trigger("submit");
    await flushPromises();
    await wrapper.get(".action-buttons .primary-button").trigger("click");
    await flushPromises();

    expect(useTaskDraftsStore().items).toHaveLength(1);
    expect(useTaskDraftsStore().items[0]).not.toHaveProperty("status");
    expect(router.currentRoute.value.path).toBe("/tasks");
  });

  it("notifies once when remote validation drops an authenticated session", async () => {
    const { pinia, sessionInfo } = render();
    const auth = useAuthStore(pinia);
    auth.snapshot = {
      revision: 2,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: "9001", name: "Fixture", avatarUrl: null },
      error: null,
    };
    await flushPromises();

    auth.snapshot = {
      revision: 3,
      status: "anonymous",
      qrContent: null,
      expiresAt: null,
      account: null,
      error: null,
    };
    await flushPromises();

    expect(sessionInfo).toHaveBeenCalledOnce();
    expect(sessionInfo).toHaveBeenCalledWith("登录已失效，已切换为匿名解析");
  });

  it("refreshes login-gated qualities when authentication succeeds", async () => {
    const anonymousResult: ParseVideoResult = {
      ...result,
      qualities: [{ id: "80", label: "1080P", requiresLogin: true }],
      videoVariants: [],
    };
    const parseVideo = vi.fn()
      .mockResolvedValueOnce(anonymousResult)
      .mockResolvedValueOnce(result);
    const { wrapper, pinia } = render(parseVideo);
    const auth = useAuthStore(pinia);

    await wrapper.get("#video-input").setValue("BV1xx411c7BF");
    await wrapper.get("form").trigger("submit");
    await flushPromises();
    expect(wrapper.get("option[value='80']").attributes("disabled")).toBeDefined();
    expect(wrapper.get("option[value='80']").text()).toContain("需登录");

    auth.snapshot = {
      revision: 1,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: "9001", name: "Fixture", avatarUrl: null },
      error: null,
    };
    await flushPromises();

    expect(parseVideo).toHaveBeenCalledTimes(2);
    expect(wrapper.get("option[value='80']").attributes("disabled")).toBeUndefined();
    expect(wrapper.get("option[value='80']").text()).not.toContain("需登录");
  });

  it("refreshes stale anonymous qualities when returning after login", async () => {
    const anonymousResult: ParseVideoResult = {
      ...result,
      qualities: [
        { id: "120", label: "4K", requiresLogin: true },
        { id: "80", label: "1080P", requiresLogin: false },
      ],
    };
    const authenticatedResult: ParseVideoResult = {
      ...anonymousResult,
      qualities: [
        { id: "120", label: "4K", requiresLogin: false },
        { id: "80", label: "1080P", requiresLogin: false },
      ],
      videoVariants: [
        { qualityId: "120", codec: "hevc" },
        { qualityId: "80", codec: "avc" },
      ],
    };
    const parseVideo = vi.fn().mockResolvedValue(authenticatedResult);
    const { wrapper } = render(parseVideo, vi.fn(), (pinia) => {
      const download = useDownloadCenterStore(pinia);
      download.input = "BV1xx411c7BF";
      download.normalizedInput = "BV1xx411c7BF";
      download.applyResult(anonymousResult);

      const auth = useAuthStore(pinia);
      auth.snapshot = {
        revision: 4,
        status: "authenticated",
        qrContent: null,
        expiresAt: null,
        account: { mid: "9001", name: "Fixture", avatarUrl: null },
        error: null,
      };
    });

    await flushPromises();

    expect(parseVideo).toHaveBeenCalledWith("BV1xx411c7BF");
    expect(wrapper.get("option[value='120']").attributes("disabled")).toBeUndefined();
  });
});
