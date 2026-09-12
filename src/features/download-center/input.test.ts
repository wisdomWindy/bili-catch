import { describe, expect, it } from "vitest";
import { normalizeParseInput } from "./input";

describe("normalizeParseInput", () => {
  it.each([
    ["  BV1xx411c7BF  ", "BV1xx411c7BF"],
    ["AV170001", "AV170001"],
    ["https://www.bilibili.com/video/BV1xx411c7BF?p=3", "https://www.bilibili.com/video/BV1xx411c7BF?p=3"],
    ["https://m.bilibili.com/video/av170001", "https://m.bilibili.com/video/av170001"],
    ["https://b23.tv/abcdef", "https://b23.tv/abcdef"],
    ["https://www.bilibili.com/video/BV1FNb366EH2/?spm\\_id_from=333.1007.top_right_bar_window_history.content.click\\&vd\\_source=80b00eb304b73285fc0a111a90e2b026", "https://www.bilibili.com/video/BV1FNb366EH2/?spm_id_from=333.1007.top_right_bar_window_history.content.click&vd_source=80b00eb304b73285fc0a111a90e2b026"],
  ])("accepts %s", (input, normalized) => {
    expect(normalizeParseInput(input)).toEqual({ ok: true, value: normalized });
  });

  it.each(["", "   \n\t "])("treats blank input as empty", (input) => {
    expect(normalizeParseInput(input)).toEqual({ ok: false, reason: "empty" });
  });

  it.each([
    "BV1 xx411c7BF",
    "BV1xx411c7BF\nhttps://example.com",
    "http://www.bilibili.com/video/BV1xx411c7BF",
    "https://bilibili.com.evil.example/video/BV1xx411c7BF",
    "https://example.com/video/BV1xx411c7BF",
  ])("rejects unsafe or malformed input %s", (input) => {
    expect(normalizeParseInput(input)).toEqual({ ok: false, reason: "invalid" });
  });
});
