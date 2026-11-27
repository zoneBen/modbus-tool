import { createI18n } from "vue-i18n";
import enUS from "./en-US";
import zhCN from "./zh-CN";
import { EN_EXTRA, ZH_EXTRA } from "./tools-extra";

const storedLocale =
  typeof window !== "undefined"
    ? window.localStorage.getItem("mt.locale")?.replace(/^"|"$/g, "")
    : null;
const initialLocale = storedLocale === "en-US" ? "en-US" : "zh-CN";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function deepMerge(
  base: Record<string, unknown>,
  extra: Record<string, unknown>,
): Record<string, unknown> {
  const result: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(extra)) {
    const current = result[key];
    result[key] = isRecord(current) && isRecord(value) ? deepMerge(current, value) : value;
  }
  return result;
}

const zhMessages = deepMerge(
  zhCN as unknown as Record<string, unknown>,
  ZH_EXTRA as unknown as Record<string, unknown>,
) as unknown as typeof zhCN;
const enMessages = deepMerge(
  enUS as unknown as Record<string, unknown>,
  EN_EXTRA as unknown as Record<string, unknown>,
) as unknown as typeof enUS;

export const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": zhMessages,
    "en-US": enMessages,
  },
});
