import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { createAppI18n } from "../../../locales";
import LogoutConfirmDialog from "./LogoutConfirmDialog.vue";

describe("LogoutConfirmDialog", () => {
  it("exposes destructive confirmation as an alert dialog", async () => {
    const wrapper = mount(LogoutConfirmDialog, {
      attachTo: document.body,
      props: { show: true, pending: false, accountName: "Fixture", errorMessage: null },
      global: { plugins: [createAppI18n()] },
    });

    await flushPromises();

    expect(document.body.querySelector("[role='alertdialog']")).not.toBeNull();
    wrapper.unmount();
  });

  it("focuses the safe action and restores the opener when closed", async () => {
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();
    const wrapper = mount(LogoutConfirmDialog, {
      attachTo: document.body,
      props: { show: false, pending: false, accountName: "Fixture", errorMessage: null },
      global: { plugins: [createAppI18n()] },
    });

    await wrapper.setProps({ show: true });
    await flushPromises();
    await vi.waitFor(() => {
      expect(document.activeElement?.getAttribute("data-testid")).toBe("keep-login");
    });
    await wrapper.setProps({ show: false });
    await flushPromises();

    expect(document.activeElement).toBe(opener);
    wrapper.unmount();
  });
});
