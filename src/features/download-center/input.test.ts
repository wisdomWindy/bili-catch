import { describe, expect, it } from "vitest";
import { normalizeParseInput } from "./input";

describe("normalizeParseInput", () => {
  it.each([
    ["  BV1xx411c7BF  ", "BV1xx411c7BF"],
    ["AV170001", "AV170001"],
    ["https://www.bilibili.com/video/BV1xx411c7BF?p=3", "https://www.bilibili.com/video/BV1xx411c7BF?p=3"],
    ["https://m.bilibili.com/video/av170001", "https://m.bilibili.com/video/av170001"],
    ["https://b23.tv/abcdef", "https://b23.tv/abcdef"],
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
