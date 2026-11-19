import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it } from "vitest";
import { useToolboxStore } from "./toolbox";

describe("toolbox store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("keeps the selected tools and exports them", () => {
    const store = useToolboxStore();
    store.setNetworkTool("dns");
    store.setDevTool("base");
    expect(store.exportConfig()).toMatchObject({
      version: 1,
      networkTool: "dns",
      devTool: "base",
    });
  });

  it("applies a valid imported config and ignores invalid ids", () => {
    const store = useToolboxStore();
    store.replaceConfig({ version: 1, networkTool: "http", devTool: "hash" });
    expect(store.config.networkTool).toBe("http");
    expect(store.config.devTool).toBe("hash");

    store.replaceConfig({ version: 1, networkTool: "missing", devTool: "missing" });
    expect(store.config.networkTool).toBe("ping");
    expect(store.config.devTool).toBe("formatter");
  });

  it("stores non-sensitive tool preferences", () => {
    const store = useToolboxStore();
    const timeout = store.toolPref("network.ping.timeout", "1000");
    timeout.value = "2500";
    expect(store.exportConfig().prefs?.["network.ping.timeout"]).toBe("2500");
    store.replaceConfig({
      version: 1,
      networkTool: "dns",
      devTool: "base",
      prefs: { "dev.base.radix": 2 },
    });
    expect(store.toolPref("dev.base.radix", 16).value).toBe(2);
  });

  it("persists the selection in localStorage", async () => {
    const store = useToolboxStore();
    store.setDevTool("search");
    await nextTick();
    expect(localStorage.getItem("mt.toolbox.v1")).toContain('"devTool":"search"');
  });
});
