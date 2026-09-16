import layoutCss from "./layout.css?raw";
import { describe, expect, it } from "vitest";

describe("content scroll styling", () => {
  it("keeps overflow scrolling and exposes a stable WebView scrollbar", () => {
    expect(layoutCss).toMatch(
      /\.content-scroll\s*\{[^}]*overflow:\s*auto;[^}]*scrollbar-gutter:\s*stable;/s,
    );
    expect(layoutCss).toMatch(
      /\.content-scroll::\-webkit-scrollbar\s*\{[^}]*width:\s*10px;/s,
    );
    expect(layoutCss).toMatch(/\.content-scroll::\-webkit-scrollbar-thumb\s*\{/);
    expect(layoutCss).toMatch(/\.content-scroll::\-webkit-scrollbar-thumb:hover\s*\{/);
  });
});
