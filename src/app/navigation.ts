import type { Component } from "vue";
import { Download, ListTodo, Settings } from "@lucide/vue";
import type { AppRouteName } from "../contracts/app";

export interface NavigationItem {
  name: Exclude<AppRouteName, "login" | "not-found">;
  icon: Component;
}

export const primaryNavigation: readonly NavigationItem[] = [
  { name: "download", icon: Download },
  { name: "tasks", icon: ListTodo },
  { name: "settings", icon: Settings },
];
