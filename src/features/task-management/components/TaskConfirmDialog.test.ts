import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import TaskConfirmDialog from "./TaskConfirmDialog.vue";

describe("TaskConfirmDialog", () => {
  it("focuses the safe action and returns focus after closing", async () => {
    const trigger = document.createElement("button");
    document.body.append(trigger); trigger.focus();
    const wrapper = mount(TaskConfirmDialog, { attachTo: document.body, props: { open: false, title: "Cancel", message: "Confirm" }, global: { plugins: [createAppI18n()] } });
    await wrapper.setProps({ open: true }); await nextTick();
    expect(document.activeElement?.textContent).toBe("返回");
    await wrapper.setProps({ open: false }); await nextTick();
    expect(document.activeElement).toBe(trigger);
    wrapper.unmount(); trigger.remove();
  });
});
