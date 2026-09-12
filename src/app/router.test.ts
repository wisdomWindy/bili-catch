import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "./router";

describe("application router", () => {
  it("redirects the root route to the download center", async () => {
    const router = createAppRouter(createMemoryHistory());
    await router.push("/");
    await router.isReady();

    expect(router.currentRoute.value.path).toBe("/download");
    expect(router.currentRoute.value.meta.titleKey).toBe("nav.download");
  });

  it.each([
    ["/download", "download", "nav.download"],
    ["/tasks", "tasks", "nav.tasks"],
    ["/login", "login", "nav.login"],
    ["/settings", "settings", "nav.settings"],
  ])("registers %s as %s", async (path, name, titleKey) => {
    const router = createAppRouter(createMemoryHistory());
    await router.push(path);

    expect(router.currentRoute.value.name).toBe(name);
    expect(router.currentRoute.value.meta.titleKey).toBe(titleKey);
  });

  it("resolves unknown locations to the not-found route", async () => {
    const router = createAppRouter(createMemoryHistory());
    await router.push("/not-a-page");

    expect(router.currentRoute.value.name).toBe("not-found");
  });
});
