import { createI18n } from "vue-i18n";
import { enUS } from "./en-US";
import { zhCN } from "./zh-CN";

export const messages = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

export function createAppI18n() {
  return createI18n({
    legacy: false,
    locale: "zh-CN",
    fallbackLocale: "zh-CN",
    messages,
    missingWarn: import.meta.env.DEV,
    fallbackWarn: import.meta.env.DEV,
  });
}
