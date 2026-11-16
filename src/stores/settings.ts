import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useLocalStorage, usePreferredDark } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed } from "vue";

export type ThemePreference = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

export const useSettingsStore = defineStore("settings", () => {
  const addressBase = useLocalStorage<0 | 1>("mt.address-base", 1);
  const reduceEffects = useLocalStorage("mt.reduce-effects", false);
  const compactRows = useLocalStorage("mt.compact-rows", false);
  const monitorFollow = useLocalStorage("mt.monitor-follow", true);
  const showRawColumn = useLocalStorage("mt.show-raw", true);
  const monitorHeight = useLocalStorage("mt.monitor-height", 258);
  const monitorCollapsed = useLocalStorage("mt.monitor-collapsed", false);
  const theme = useLocalStorage<ThemePreference>("mt.theme", "dark");
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

  function toggleAddressBase(): void {
    addressBase.value = addressBase.value === 0 ? 1 : 0;
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
    applyTheme,
    setTheme,
    toggleTheme,
    toggleAddressBase,
  };
});
