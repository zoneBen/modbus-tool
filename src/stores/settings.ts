import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useLocalStorage, usePreferredDark } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, watch } from "vue";
import { i18n } from "@/i18n";

export type ThemePreference = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";
export type LocaleCode = "zh-CN" | "en-US";

export interface AppSettingsSnapshot {
  addressBase: 0 | 1;
  reduceEffects: boolean;
  compactRows: boolean;
  monitorFollow: boolean;
  showRawColumn: boolean;
  monitorHeight: number;
  monitorCollapsed: boolean;
  theme: ThemePreference;
  locale: LocaleCode;
}

export const useSettingsStore = defineStore("settings", () => {
  const addressBase = useLocalStorage<0 | 1>("mt.address-base-v2", 0);
  const reduceEffects = useLocalStorage("mt.reduce-effects", false);
  const compactRows = useLocalStorage("mt.compact-rows", false);
  const monitorFollow = useLocalStorage("mt.monitor-follow", true);
  const showRawColumn = useLocalStorage("mt.show-raw", true);
  const monitorHeight = useLocalStorage("mt.monitor-height", 258);
  const monitorCollapsed = useLocalStorage("mt.monitor-collapsed", false);
  const theme = useLocalStorage<ThemePreference>("mt.theme", "dark");
  const locale = useLocalStorage<LocaleCode>("mt.locale", "zh-CN");
  const prefersDark = usePreferredDark();

  const resolvedTheme = computed<ResolvedTheme>(() =>
    theme.value === "system" ? (prefersDark.value ? "dark" : "light") : theme.value,
  );

  function applyTheme(): void {
    if (typeof document === "undefined") return;
    const next = resolvedTheme.value;
    const root = document.documentElement;
    root.classList.toggle("dark", next === "dark");
    root.dataset.theme = next;
    root.style.colorScheme = next;

    if (isTauri()) {
      // Native title bar theming is a nicety; unsupported platforms can ignore it.
      void getCurrentWindow()
        .setTheme(next)
        .catch(() => undefined);
    }
  }

  function setTheme(next: ThemePreference): void {
    theme.value = next;
    applyTheme();
  }

  function toggleTheme(): void {
    setTheme(resolvedTheme.value === "dark" ? "light" : "dark");
  }

  watch(
    locale,
    (value) => {
      i18n.global.locale.value = value;
      if (typeof document !== "undefined") document.documentElement.lang = value;
    },
    { immediate: true },
  );

  function setLocale(next: LocaleCode): void {
    locale.value = next;
  }

  function toggleAddressBase(): void {
    addressBase.value = addressBase.value === 0 ? 1 : 0;
  }

  function snapshot(): AppSettingsSnapshot {
    return {
      addressBase: addressBase.value === 1 ? 1 : 0,
      reduceEffects: Boolean(reduceEffects.value),
      compactRows: Boolean(compactRows.value),
      monitorFollow: Boolean(monitorFollow.value),
      showRawColumn: Boolean(showRawColumn.value),
      monitorHeight: typeof monitorHeight.value === "number" ? monitorHeight.value : 258,
      monitorCollapsed: Boolean(monitorCollapsed.value),
      theme: theme.value,
      locale: locale.value,
    };
  }

  function applySnapshot(value: Partial<AppSettingsSnapshot>): void {
    if (value.addressBase === 0 || value.addressBase === 1) {
      addressBase.value = value.addressBase;
    }
    if (typeof value.reduceEffects === "boolean") {
      reduceEffects.value = value.reduceEffects;
    }
    if (typeof value.compactRows === "boolean") {
      compactRows.value = value.compactRows;
    }
    if (typeof value.monitorFollow === "boolean") {
      monitorFollow.value = value.monitorFollow;
    }
    if (typeof value.showRawColumn === "boolean") {
      showRawColumn.value = value.showRawColumn;
    }
    if (typeof value.monitorHeight === "number" && Number.isFinite(value.monitorHeight)) {
      monitorHeight.value = Math.min(520, Math.max(160, value.monitorHeight));
    }
    if (typeof value.monitorCollapsed === "boolean") {
      monitorCollapsed.value = value.monitorCollapsed;
    }
    if (value.theme === "dark" || value.theme === "light" || value.theme === "system") {
      setTheme(value.theme);
    }
    if (value.locale === "zh-CN" || value.locale === "en-US") {
      setLocale(value.locale);
    }
  }

  return {
    addressBase,
    reduceEffects,
    compactRows,
    monitorFollow,
    showRawColumn,
    monitorHeight,
    monitorCollapsed,
    theme,
    resolvedTheme,
    locale,
    applyTheme,
    setTheme,
    toggleTheme,
    setLocale,
    toggleAddressBase,
    snapshot,
    applySnapshot,
  };
});
