import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { useDebugStore } from "./debug";

describe("debug store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("keeps chunk boundaries and filters other sessions", () => {
    const store = useDebugStore();
    store.applyState({ sessionId: "s1", active: true });

    store.applyData({
      sessionId: "s1",
      direction: "rx",
      data: [0x41],
      timestamp: 1000,
    });
    store.applyData({
      sessionId: "s1",
      direction: "rx",
      data: [0x42],
      timestamp: 1050,
    });
    store.applyData({
      sessionId: "s1",
      direction: "tx",
      data: [0x43],
      timestamp: 1080,
    });
    store.applyData({
      sessionId: "s2",
      direction: "rx",
      data: [0x44],
      timestamp: 1090,
    });

    expect(store.active).toBe(true);
    expect(store.entries).toHaveLength(3);
    expect(store.entries[0]?.data).toEqual([0x41]);
    expect(store.entries[1]?.data).toEqual([0x42]);
    expect(store.entries[2]?.direction).toBe("tx");
  });

  it("keeps the log while surfacing a link error", () => {
    const store = useDebugStore();
    store.applyState({ sessionId: "s1", active: true });
    store.applyData({
      sessionId: "s1",
      direction: "rx",
      data: [0x01],
      timestamp: 1000,
    });
    store.applyState({
      sessionId: "s1",
      active: false,
      error: "连接已关闭",
    });

    expect(store.active).toBe(false);
    expect(store.error).toBe("连接已关闭");
    expect(store.entries).toHaveLength(1);
  });
});
