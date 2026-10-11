import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import "../task-management.css";
import TaskToolbar from "./TaskToolbar.vue";

describe("TaskToolbar", () => {
  it("keeps the task filters free of nested scrollbars", () => {
    const wrapper = mount(TaskToolbar, {
      props: {
        filter: "all",
        counts: { all: 1, download: 0, transcription: 1, active: 0, completed: 0, failed: 1 },
        canPause: false,
        canClear: false,
        busy: false,
      },
      global: { plugins: [createAppI18n()] },
    });

    const style = getComputedStyle(wrapper.get("[role='tablist']").element);
    expect(style.overflowX).toBe("visible");
    expect(style.overflowY).toBe("visible");
  });
});
