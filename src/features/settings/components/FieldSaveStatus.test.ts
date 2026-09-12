import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import FieldSaveStatus from "./FieldSaveStatus.vue";

describe("FieldSaveStatus", () => {
  it("announces stable localized states without exposing raw errors", async () => {
    const wrapper = mount(FieldSaveStatus, {
      props: {
        id: "field-status",
        state: { status: "error", error: { code: "E_INTERNAL", message: "private path" } },
      },
      global: { plugins: [createAppI18n()] },
    });

    expect(wrapper.attributes("aria-live")).toBe("polite");
    expect(wrapper.text()).toBe("保存失败");
    expect(wrapper.text()).not.toContain("private path");
    await wrapper.setProps({ state: { status: "saved", error: null } });
    expect(wrapper.text()).toBe("已保存");
  });
});
