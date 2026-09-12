import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toCanvas } from "qrcode";
import { createAppI18n } from "../../../locales";
import QrLoginPanel from "./QrLoginPanel.vue";

vi.mock("qrcode", () => ({ toCanvas: vi.fn(async () => undefined) }));

describe("QrLoginPanel", () => {
  beforeEach(() => vi.mocked(toCanvas).mockResolvedValue(undefined));

  it("renders the ephemeral content only into a fixed canvas", async () => {
    const qrContent = "https://passport.bilibili.com/fixture-private-content";
    const wrapper = mount(QrLoginPanel, {
      props: {
        status: "waiting_scan",
        qrContent,
        expiresAt: "2026-09-11T00:03:00Z",
        pending: false,
        errorMessage: null,
      },
      global: { plugins: [createAppI18n()] },
    });
    await flushPromises();

    expect(toCanvas).toHaveBeenCalledWith(
      expect.any(HTMLCanvasElement),
      qrContent,
      expect.objectContaining({ width: 224 }),
    );
    expect(wrapper.html()).not.toContain(qrContent);
    expect(wrapper.get("canvas").attributes()).toMatchObject({ width: "224", height: "224" });
    expect(wrapper.get("[aria-live='polite']").text()).not.toMatch(/\d{2}:\d{2}/);
  });

  it("replaces a failed canvas with a stable retry state", async () => {
    vi.mocked(toCanvas).mockRejectedValueOnce(new Error("fixture QR failure"));
    const wrapper = mount(QrLoginPanel, {
      props: {
        status: "waiting_scan",
        qrContent: "https://passport.bilibili.com/fixture",
        expiresAt: null,
        pending: false,
        errorMessage: null,
      },
      global: { plugins: [createAppI18n()] },
    });
    await flushPromises();

    expect(wrapper.find("canvas").exists()).toBe(false);
    expect(wrapper.find("[data-testid='qr-render-error']").exists()).toBe(true);
    await wrapper.get("[data-testid='refresh-qr']").trigger("click");
    expect(wrapper.emitted("refresh")).toHaveLength(1);
  });

  it("ignores a late canvas failure after the auth state has changed", async () => {
    let rejectRender!: (reason: unknown) => void;
    vi.mocked(toCanvas).mockImplementationOnce(() => new Promise<void>((_, reject) => {
      rejectRender = reject;
    }));
    const wrapper = mount(QrLoginPanel, {
      props: {
        status: "waiting_scan",
        qrContent: "https://passport.bilibili.com/fixture",
        expiresAt: null,
        pending: false,
        errorMessage: null,
      },
      global: { plugins: [createAppI18n()] },
    });
    await vi.waitFor(() => expect(toCanvas).toHaveBeenCalledOnce());

    await wrapper.setProps({
      status: "error",
      qrContent: null,
      errorMessage: "请求超时",
    });
    rejectRender(new Error("late fixture failure"));
    await flushPromises();

    expect(wrapper.get("[aria-live='polite']").text()).toContain("登录暂时不可用");
    expect(wrapper.get("[aria-live='polite']").text()).toContain("请求超时");
    expect(wrapper.find("[data-testid='qr-render-error']").exists()).toBe(false);
  });

  it("clears its countdown timer when unmounted", () => {
    const clearInterval = vi.spyOn(window, "clearInterval");
    const wrapper = mount(QrLoginPanel, {
      props: {
        status: "requesting",
        qrContent: null,
        expiresAt: null,
        pending: true,
        errorMessage: null,
      },
      global: { plugins: [createAppI18n()] },
    });

    wrapper.unmount();

    expect(clearInterval).toHaveBeenCalledOnce();
    clearInterval.mockRestore();
  });

  it.each([
    ["restoring", "正在恢复登录状态"],
    ["anonymous", "正在准备登录"],
    ["requesting", "正在生成二维码"],
    ["waiting_scan", "等待扫描"],
    ["waiting_confirm", "等待确认"],
    ["expired", "二维码已过期"],
    ["cancelled", "登录已取消"],
    ["error", "登录暂时不可用"],
  ] as const)("renders the %s public state", async (status, label) => {
    const waiting = status === "waiting_scan" || status === "waiting_confirm";
    const wrapper = mount(QrLoginPanel, {
      props: {
        status,
        qrContent: waiting ? "https://passport.bilibili.com/fixture" : null,
        expiresAt: waiting ? "2026-09-11T00:03:00Z" : null,
        pending: false,
        errorMessage: null,
      },
      global: { plugins: [createAppI18n()] },
    });
    await flushPromises();

    expect(wrapper.get("[aria-live='polite']").text()).toContain(label);
    wrapper.unmount();
  });
});
