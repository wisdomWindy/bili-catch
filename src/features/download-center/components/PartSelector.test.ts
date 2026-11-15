import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { createAppI18n } from "../../../locales";
import type { ParseContainer, VideoPart } from "../contracts";
import PartSelector from "./PartSelector.vue";

function unit(overrides: Partial<VideoPart> & Pick<VideoPart, "cid">): VideoPart {
  return {
    bvid: "BV1xx411c7BF",
    epId: null,
    page: 1,
    partCount: 1,
    groupIndex: 1,
    title: `Unit ${overrides.cid}`,
    durationSeconds: 90,
    availability: "playable",
    unavailableReason: null,
    ...overrides,
  };
}

function render(parts: VideoPart[], container: ParseContainer, selectedCids: number[] = []) {
  return mount(PartSelector, {
    props: {
      parts,
      selectedCids,
      source: container.kind === "season" ? "bangumi" : "ugc",
      container,
    },
    global: { plugins: [createAppI18n()] },
  });
}

const season: ParseContainer = {
  kind: "season",
  id: "9015689",
  title: "AI信息分享",
  unitCount: 2,
};

describe("PartSelector", () => {
  it("labels bangumi units with EP and disables unavailable episodes", () => {
    const wrapper = render(
      [
        unit({ cid: 1, epId: 733316, groupIndex: 1, title: "第1话" }),
        unit({
          cid: 2,
          epId: 733317,
          groupIndex: 2,
          title: "第2话",
          availability: "unavailable",
          unavailableReason: "vip-required",
        }),
      ],
      season,
    );

    expect(wrapper.findAll(".part-index").map((node) => node.text())).toEqual(["EP1", "EP2"]);
    const rows = wrapper.findAll(".part-row");
    expect(rows[1].classes()).toContain("is-disabled");
    expect(rows[1].find(".part-reason").text()).toBe("需大会员");
    expect(rows[1].find("input").attributes("disabled")).toBeDefined();
  });

  it("renders a group row for a multi-part collection member", async () => {
    const wrapper = render(
      [
        unit({ cid: 10, groupIndex: 1, page: 1, partCount: 2, title: "P1" }),
        unit({ cid: 11, groupIndex: 1, page: 2, partCount: 2, title: "P2" }),
      ],
      season,
    );

    expect(wrapper.findAll(".part-index").map((node) => node.text())).toEqual(["EP1", "P1", "P2"]);
    expect(wrapper.findAll(".part-child")).toHaveLength(2);

    const groupInput = wrapper.find(".part-group input");
    expect((groupInput.element as HTMLInputElement).indeterminate).toBe(false);
    await groupInput.trigger("change");
    expect(wrapper.emitted("toggleGroup")?.at(-1)).toEqual([[10, 11]]);
  });

  it("keeps the flat P list for plain videos", () => {
    const wrapper = render(
      [
        unit({ cid: 20, page: 1, partCount: 2, title: "Opening" }),
        unit({ cid: 21, page: 2, partCount: 2, title: "Closing" }),
      ],
      { kind: "video", id: "BV1xx411c7BF", title: "Video", unitCount: 2 },
    );

    expect(wrapper.findAll(".part-index").map((node) => node.text())).toEqual(["P1", "P2"]);
    expect(wrapper.findAll(".part-group")).toHaveLength(0);
  });

  it("marks a partially selected group and keeps unavailable children out of toggles", async () => {
    const wrapper = render(
      [
        unit({ cid: 30, groupIndex: 1, page: 1, partCount: 2 }),
        unit({
          cid: 31,
          groupIndex: 1,
          page: 2,
          partCount: 2,
          availability: "unavailable",
          unavailableReason: "paid",
        }),
      ],
      season,
      [30],
    );

    await wrapper.find(".part-group input").trigger("change");
    expect(wrapper.emitted("toggleGroup")?.at(-1)).toEqual([[30]]);
  });
});
