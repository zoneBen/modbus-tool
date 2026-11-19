import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import type { WritableComputedRef } from "vue";
import { isDevToolId, isNetworkToolId } from "@/lib/tool-catalog";
import type { DevToolId, NetworkToolId } from "@/lib/tool-catalog";

const STORAGE_KEY = "mt.toolbox.v1";

export interface ToolboxConfig {
  version: 1;
  networkTool: NetworkToolId;
  devTool: DevToolId;
  prefs?: Record<string, unknown>;
}

const DEFAULT_CONFIG: ToolboxConfig = {
  version: 1,
  networkTool: "ping",
  devTool: "formatter",
  prefs: {},
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function loadConfig(): ToolboxConfig {
  if (typeof localStorage === "undefined") return { ...DEFAULT_CONFIG };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_CONFIG };
    const parsed = JSON.parse(raw) as Partial<ToolboxConfig>;
    if (parsed.version !== 1) return { ...DEFAULT_CONFIG };
    return {
      version: 1,
      networkTool: isNetworkToolId(parsed.networkTool)
        ? parsed.networkTool
        : DEFAULT_CONFIG.networkTool,
      devTool: isDevToolId(parsed.devTool) ? parsed.devTool : DEFAULT_CONFIG.devTool,
      prefs: isRecord(parsed.prefs) ? { ...parsed.prefs } : {},
    };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

export const useToolboxStore = defineStore("toolbox", () => {
  const config = ref<ToolboxConfig>(loadConfig());

  watch(
    config,
    (value) => {
      if (typeof localStorage === "undefined") return;
      localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
    },
    { deep: true },
  );

  function setNetworkTool(tool: NetworkToolId): void {
    config.value = { ...config.value, networkTool: tool };
  }

  function setDevTool(tool: DevToolId): void {
    config.value = { ...config.value, devTool: tool };
  }

  function toolPref<T>(key: string, fallback: T): WritableComputedRef<T> {
    return computed<T>({
      get: () => {
        const prefs = config.value.prefs ?? {};
        return key in prefs ? (prefs[key] as T) : fallback;
      },
      set: (value) => {
        config.value = {
          ...config.value,
          prefs: { ...(config.value.prefs ?? {}), [key]: value },
        };
      },
    });
  }

  function exportConfig(): ToolboxConfig {
    return { ...config.value, prefs: { ...(config.value.prefs ?? {}) } };
  }

  function replaceConfig(value: unknown): void {
    if (typeof value !== "object" || value === null) return;
    const record = value as Partial<ToolboxConfig>;
    config.value = {
      version: 1,
      networkTool: isNetworkToolId(record.networkTool)
        ? record.networkTool
        : DEFAULT_CONFIG.networkTool,
      devTool: isDevToolId(record.devTool) ? record.devTool : DEFAULT_CONFIG.devTool,
      prefs: isRecord(record.prefs) ? { ...record.prefs } : {},
    };
  }

  return {
    config,
    setNetworkTool,
    setDevTool,
    toolPref,
    exportConfig,
    replaceConfig,
  };
});
