import { beforeEach, describe, expect, it } from "vitest";
import {
  clearPersistedWorkspace,
  loadPersistedWorkspace,
  persistedSessionInput,
  savePersistedWorkspace,
  workspaceConfigSignature,
} from "@/lib/persisted-workspace";
import type { Session } from "@/types/models";

function sampleSession(): Session {
  return {
    id: "session-1",
    name: "PLC",
    role: "master",
    transport: "tcp",
    framing: "mbap",
    endpoint: "127.0.0.1:502",
    status: "connected",
    polling: true,
    accent: "signal",
    stats: { tx: 4, rx: 4, errors: 0, latencyMs: 3 },
    startedAt: 1,
    host: "127.0.0.1",
    port: 502,
    timeoutMs: 1000,
    retries: 1,
    interFrameDelayMs: 0,
    autoReconnect: true,
    definitions: [
      {
        id: "definition-1",
        name: "Registers",
        functionCode: 3,
        slaveId: 1,
        startAddress: 0,
        quantity: 4,
        dataType: "u16",
        byteOrder: "abcd",
        intervalMs: 500,
        enabled: true,
        aliases: { 0: "Temperature" },
        points: [
          {
            address: 0,
            dataType: "u16",
            byteOrder: "abcd",
            meaning: "Temperature",
            unit: "C",
            scale: 0.1,
            offset: -5,
            alerts: [{ id: "rule-1", operator: "gt", value: 80, delayMs: 500, enabled: true }],
          },
          {
            address: 2,
            dataType: "u16",
            byteOrder: "abcd",
            meaning: "Level",
            unit: "%",
            scale: 1,
            offset: 0,
            alerts: [],
            simulator: { mode: "increment", refreshMs: 250, min: 0, max: 100, enabled: true },
          },
        ],
      },
    ],
  };
}

describe("persisted workspace", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("round-trips points, alerts and simulator configuration", () => {
    savePersistedWorkspace([sampleSession()]);
    const restored = loadPersistedWorkspace();

    expect(restored?.sessions).toHaveLength(1);
    const definition = restored?.sessions[0]?.definitions[0];
    expect(definition?.aliases[0]).toBe("Temperature");
    expect(definition?.points[0]?.scale).toBe(0.1);
    expect(definition?.points[0]?.offset).toBe(-5);
    expect(definition?.points[0]?.alerts[0]?.delayMs).toBe(500);
    expect(definition?.points[1]?.simulator?.mode).toBe("increment");
    expect(definition?.points[1]?.simulator?.refreshMs).toBe(250);
  });

  it("restores sessions disconnected", () => {
    savePersistedWorkspace([sampleSession()]);
    const loaded = loadPersistedWorkspace()?.sessions[0];
    if (!loaded) throw new Error("workspace did not round-trip");
    const input = persistedSessionInput(loaded);

    expect(input.autoConnect).toBe(false);
  });

  it("changes the config signature when point rules change", () => {
    const original = sampleSession();
    const updated = sampleSession();
    const point = updated.definitions[0]?.points[0];
    if (point) point.scale = 2;

    expect(workspaceConfigSignature([updated])).not.toBe(workspaceConfigSignature([original]));
  });

  it("clears the saved workspace", () => {
    savePersistedWorkspace([sampleSession()]);
    clearPersistedWorkspace();
    expect(loadPersistedWorkspace()).toBeNull();
  });
});
