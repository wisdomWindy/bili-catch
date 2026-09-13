import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "../app/router";
import { createAppI18n } from "../locales";
import { useAuthStore } from "../features/authentication/store";
import type { AuthSnapshot } from "../features/authentication/contracts";
import LoginPage from "./LoginPage.vue";

const panelStubs = {
  QrLoginPanel: {
    props: ["errorMessage"],
    template: "<div><span data-testid='error-message'>{{ errorMessage }}</span><button data-testid='refresh-stub' @click='$emit(\"refresh\")' /></div>",
    emits: ["refresh"],
  },
  AuthAccountPanel: { template: "<button data-testid='logout-stub' @click='$emit(\"logout\")' />", emits: ["logout"] },
  LogoutConfirmDialog: { template: "<div />" },
};

async function render(path: string, initialSnapshot?: AuthSnapshot) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const router = createAppRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  const store = useAuthStore();
  if (initialSnapshot) store.snapshot = initialSnapshot;
  const wrapper = mount(LoginPage, {
    global: { plugins: [pinia, router, createAppI18n()], stubs: panelStubs },
  });
  await flushPromises();
  return { wrapper, router, store };
}

describe("LoginPage", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("starts once for an anonymous revision and cancels active login on unmount", async () => {
    const { wrapper, store } = await render("/login");
    store.snapshot = { ...store.snapshot, revision: 1, status: "anonymous" };
    const start = vi.spyOn(store, "start").mockResolvedValue();
    const cancel = vi.spyOn(store, "cancel").mockResolvedValue();
    store.snapshot = { ...store.snapshot, revision: 2, status: "anonymous" };
    await flushPromises();
    expect(start).toHaveBeenCalledOnce();

    store.snapshot = { ...store.snapshot, revision: 3, status: "waiting_scan" };
    wrapper.unmount();
    expect(cancel).toHaveBeenCalledOnce();
  });

  it("returns to a valid source route 800ms after authentication", async () => {
    const { wrapper, router, store } = await render("/login?from=/tasks");
    await wrapper.get("[data-testid='refresh-stub']").trigger("click");
    store.snapshot = {
      ...store.snapshot,
      revision: 4,
      status: "waiting_scan",
    };
    store.snapshot = {
      revision: 5,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: "9001", name: "Fixture", avatarUrl: null },
      error: null,
    };
    await flushPromises();
    await vi.advanceTimersByTimeAsync(800);

    expect(router.currentRoute.value.path).toBe("/tasks");
  });

  it("keeps a restoring session on the account page after authentication", async () => {
    const { router, store } = await render("/login");
    store.snapshot = {
      revision: 5,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: null, name: "", avatarUrl: null },
      error: null,
    };
    await flushPromises();
    await vi.advanceTimersByTimeAsync(800);

    expect(router.currentRoute.value.path).toBe("/login");
  });

  it("keeps an initially authenticated account page open", async () => {
    const authenticated: AuthSnapshot = {
      revision: 5,
      status: "authenticated",
      qrContent: null,
      expiresAt: null,
      account: { mid: "9001", name: "Fixture", avatarUrl: null },
      error: null,
    };
    const { router } = await render("/login?from=/tasks", authenticated);

    await vi.advanceTimersByTimeAsync(801);

    expect(router.currentRoute.value.path).toBe("/login");
  });

  it("maps backend errors to localized public copy", async () => {
    const { wrapper, store } = await render("/login");
    store.snapshot = {
      ...store.snapshot,
      revision: 2,
      status: "error",
      error: { code: "E002", message: "raw adapter timeout" },
    };
    await flushPromises();

    expect(wrapper.get("[data-testid='error-message']").text()).toBe("请求超时");
    expect(wrapper.text()).not.toContain("raw adapter timeout");
  });
});
