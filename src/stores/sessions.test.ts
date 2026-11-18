import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import type { DefinitionSnapshot, Session, WorkspaceSnapshot } from "@/types/models";
import { useSessionsStore } from "./sessions";

const session: Session = {
  id: "s1",
  name: "Test PLC",
  role: "master",
  transport: "tcp",
  framing: "mbap",
  endpoint: "127.0.0.1:502",
  status: "connected",
  polling: true,
  accent: "signal",
  startedAt: 0,
  stats: { tx: 1, rx: 1, errors: 0, latencyMs: 5 },
  definitions: [
    {
      id: "d1",
      name: "Holding",
      functionCode: 3,
      slaveId: 1,
      startAddress: 0,
      quantity: 2,
      dataType: "u16",
      byteOrder: "abcd",
      intervalMs: 1000,
      enabled: true,
      aliases: { 0: "Temperature" },
      points: [],
    },
  ],
};

const workspace: WorkspaceSnapshot = { sessions: [session], version: 1, demo: true };

function snapshot(timestamp: number, first: number): DefinitionSnapshot {
  return {
    sessionId: "s1",
    definitionId: "d1",
    timestamp,
    latencyMs: 4,
    values: [
      { address: 0, words: [first], formatted: String(first), numeric: first },
      { address: 1, words: [42], formatted: "42", numeric: 42 },
    ],
  };
}

describe("sessions store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("selects the first master session on boot", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    expect(store.demo).toBe(true);
    expect(store.activeSessionId).toBe("s1");
    expect(store.activeDefinitionId).toBe("d1");
    expect(store.activeDefinition?.name).toBe("Holding");
  });

  it("falls back to the first definition when the active one disappears", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    store.setActiveDefinition("missing");
    store.applyWorkspace(workspace);
    expect(store.activeDefinitionId).toBe("d1");
    expect(store.activeDefinition?.name).toBe("Holding");
  });

  it("stores snapshots and bounded history", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    store.applySnapshot(snapshot(1, 10));
    store.applySnapshot(snapshot(2, 20));
    expect(store.activeSnapshot?.values[0]?.numeric).toBe(20);
    expect(store.history["d1:0"]).toEqual([10, 20]);
    expect(store.lastUpdate).toBe(2);
  });

  it("keeps aligned chart timelines", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    store.applySnapshot(snapshot(1, 10));
    store.applySnapshot(snapshot(2, 20));
    const timeline = store.timelines["d1"];
    expect(timeline?.timestamps).toEqual([1, 2]);
    expect(timeline?.series[0]).toEqual([10, 20]);
    expect(timeline?.series[1]).toEqual([42, 42]);
  });

  it("keeps failed reads out of history and trend data", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    store.applySnapshot(snapshot(1, 10));
    const failed = snapshot(2, 0);
    store.applySnapshot({
      ...failed,
      error: "Modbus 异常 0x02",
      values: failed.values.map((entry) => ({
        ...entry,
        words: [],
        formatted: "—",
        numeric: null,
        stale: true,
        error: "Modbus 异常 0x02",
      })),
    });
    expect(store.history["d1:0"]).toEqual([10]);
    expect(store.timelines["d1"]?.series[0]).toEqual([10, null]);
  });

  it("applies status events immutably", () => {
    const store = useSessionsStore();
    store.applyWorkspace(workspace);
    store.applyStatus({
      sessionId: "s1",
      status: "error",
      polling: false,
      stats: { tx: 9, rx: 8, errors: 1, latencyMs: 7 },
    });
    expect(store.activeSession?.status).toBe("error");
    expect(store.activeSession?.polling).toBe(false);
    expect(store.activeSession?.stats.errors).toBe(1);
  });
});
