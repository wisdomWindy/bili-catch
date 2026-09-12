import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import RangeSetting from "./RangeSetting.vue";

describe("RangeSetting", () => {
  it("previews on input and commits once on change with complete range semantics", async () => {
    const wrapper = mount(RangeSetting, {
      props: {
        id: "setting-connections",
        field: "connectionsPerTask",
        label: "单任务连接数",
        value: 8,
        min: 1,
        max: 32,
        status: { status: "idle", error: null },
      },
      global: { plugins: [createAppI18n()] },
    });
    const input = wrapper.get("input[type='range']");
    (input.element as HTMLInputElement).value = "16";
    await input.trigger("input");
    await input.trigger("change");
    await wrapper.setProps({ value: 16 });

    expect(wrapper.emitted("preview")).toEqual([[16]]);
    expect(wrapper.emitted("commit")).toEqual([[16]]);
    expect(input.attributes()).toMatchObject({ min: "1", max: "32", step: "1" });
    expect(input.attributes("aria-describedby")).toBe("setting-connections-status");
    expect(input.attributes("aria-valuetext")).toContain("16");
    expect(wrapper.find("[aria-live]").text()).not.toContain("16");
  });
});
