import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { installThemeSync, resolveTheme, useAppStore } from "./app";

class FakeMediaQueryList {
  matches = false;
  addEventListener = vi.fn((_type: string, listener: (event: MediaQueryListEvent) => void) => {
    this.listener = listener;
  });
  removeEventListener = vi.fn();
  private listener?: (event: MediaQueryListEvent) => void;

  emit(matches: boolean) {
    this.matches = matches;
    this.listener?.({ matches } as MediaQueryListEvent);
  }
}

describe("app theme state", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("resolves all theme preferences", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
    expect(resolveTheme("system", true)).toBe("dark");
  });

  it("follows system changes only while the preference is system", async () => {
    const store = useAppStore();
    const media = new FakeMediaQueryList();
    const root = document.createElement("div");
    const dispose = installThemeSync(store, root, media as unknown as MediaQueryList);

    expect(root.dataset.theme).toBe("light");
    media.emit(true);
    expect(root.dataset.theme).toBe("dark");

    store.setTheme("light");
    await Promise.resolve();
    media.emit(true);
    expect(root.dataset.theme).toBe("light");

    dispose();
    expect(media.removeEventListener).toHaveBeenCalledOnce();
  });

  it("switches between the supported locales", () => {
    const store = useAppStore();
    store.setLocale("en-US");
    expect(store.locale).toBe("en-US");
  });
});
