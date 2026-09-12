import {
  createRouter,
  createWebHashHistory,
  type RouteRecordRaw,
  type Router,
  type RouterHistory,
} from "vue-router";
import type { AppRouteName } from "../contracts/app";
import DownloadPage from "../pages/DownloadPage.vue";
import TasksPage from "../pages/TasksPage.vue";
import LoginPage from "../pages/LoginPage.vue";
import SettingsPage from "../pages/SettingsPage.vue";
import NotFoundPage from "../pages/NotFoundPage.vue";

declare module "vue-router" {
  interface RouteMeta {
    titleKey: string;
  }
}

export const appRoutes: RouteRecordRaw[] = [
  { path: "/", redirect: "/download", meta: { titleKey: "nav.download" } },
  { path: "/download", name: "download" satisfies AppRouteName, component: DownloadPage, meta: { titleKey: "nav.download" } },
  { path: "/tasks", name: "tasks" satisfies AppRouteName, component: TasksPage, meta: { titleKey: "nav.tasks" } },
  { path: "/login", name: "login" satisfies AppRouteName, component: LoginPage, meta: { titleKey: "nav.login" } },
  { path: "/settings", name: "settings" satisfies AppRouteName, component: SettingsPage, meta: { titleKey: "nav.settings" } },
  { path: "/:pathMatch(.*)*", name: "not-found" satisfies AppRouteName, component: NotFoundPage, meta: { titleKey: "errors.notFound" } },
];

export function createAppRouter(history: RouterHistory = createWebHashHistory()): Router {
  return createRouter({ history, routes: appRoutes });
}
