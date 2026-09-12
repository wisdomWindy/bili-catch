import { describe, expect, it } from "vitest";
import { createAppI18n, messages } from "./index";

function leafKeys(value: object, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof child === "object" && child !== null ? leafKeys(child, path) : [path];
  });
}

describe("application locales", () => {
  it("defaults to Simplified Chinese with an explicit fallback", () => {
    const i18n = createAppI18n();
    expect(i18n.global.locale.value).toBe("zh-CN");
    expect(i18n.global.fallbackLocale.value).toBe("zh-CN");
  });

  it("keeps the English and Chinese message trees in sync", () => {
    expect(leafKeys(messages["en-US"])).toEqual(leafKeys(messages["zh-CN"]));
  });
});
