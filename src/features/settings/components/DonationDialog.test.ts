import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import DonationDialog from "./DonationDialog.vue";

describe("DonationDialog", () => {
  it("shows both local payment QR codes without enabling image preview", async () => {
    const wrapper = mount(DonationDialog, {
      attachTo: document.body,
      props: { show: false },
      global: { plugins: [createAppI18n()] },
    });

    await wrapper.setProps({ show: true });
    await nextTick();
    await nextTick();

    const dialog = document.body.querySelector<HTMLElement>("[role='dialog']");
    expect(dialog?.textContent).toContain("打赏");
    expect(dialog?.textContent).toContain("支付宝");
    expect(dialog?.textContent).toContain("微信支付");

    const alipay = document.body.querySelector<HTMLImageElement>("img[alt='支付宝收款码']");
    const wechat = document.body.querySelector<HTMLImageElement>("img[alt='微信收款码']");
    expect(alipay?.src).toContain("alipay-donation-qr");
    expect(wechat?.src).toContain("wechat-donation-qr");
    expect(dialog?.querySelectorAll(".n-image--preview-disabled")).toHaveLength(2);

    wrapper.unmount();
  });

  it("emits close for the explicit action and Escape key", async () => {
    const wrapper = mount(DonationDialog, {
      attachTo: document.body,
      props: { show: false },
      global: { plugins: [createAppI18n()] },
    });

    await wrapper.setProps({ show: true });
    await nextTick();
    await nextTick();

    const closeButton = document.body.querySelector<HTMLButtonElement>("[data-testid='close-donation']");
    expect(closeButton?.disabled).toBe(false);
    closeButton?.click();
    expect(wrapper.emitted("close")).toHaveLength(1);

    document.body.querySelector<HTMLElement>("[role='dialog']")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(wrapper.emitted("close")).toHaveLength(2);

    wrapper.unmount();
  });

  it("restores the previously focused trigger when hidden", async () => {
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();

    const wrapper = mount(DonationDialog, {
      attachTo: document.body,
      props: { show: false },
      global: { plugins: [createAppI18n()] },
    });
    await wrapper.setProps({ show: true });
    await nextTick();
    await nextTick();
    await wrapper.setProps({ show: false });
    await nextTick();
    await nextTick();

    expect(document.activeElement).toBe(trigger);
    wrapper.unmount();
  });
});
