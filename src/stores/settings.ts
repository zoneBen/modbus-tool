import { useLocalStorage } from "@vueuse/core";
import { defineStore } from "pinia";

export const useSettingsStore = defineStore("settings", () => {
  const addressBase = useLocalStorage<0 | 1>("mt.address-base", 1);
  const reduceEffects = useLocalStorage("mt.reduce-effects", false);
  const compactRows = useLocalStorage("mt.compact-rows", false);
  const monitorFollow = useLocalStorage("mt.monitor-follow", true);
  const showRawColumn = useLocalStorage("mt.show-raw", true);
  const monitorHeight = useLocalStorage("mt.monitor-height", 258);
  const monitorCollapsed = useLocalStorage("mt.monitor-collapsed", false);

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
    toggleAddressBase,
  };
});
