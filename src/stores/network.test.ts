import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { defaultNetworkSession, useNetworkStore } from "@/stores/network";

const STORAGE_KEY = "mt.network.sessions.v1";

describe("network store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("creates multiple independent network sessions", () => {
    const network = useNetworkStore();
    const first = network.createSession(defaultNetworkSession("tcp-server", "Server"));
    const second = network.createSession(defaultNetworkSession("tcp-client", "Client"));

    expect(network.sessions).toHaveLength(2);
    expect(network.activeSessionId).toBe(second.id);
    network.selectSession(first.id);
    expect(network.activeSession?.name).toBe("Server");
  });

  it("validates the listening port before starting a server", async () => {
    const network = useNetworkStore();
    const session = network.createSession(defaultNetworkSession("tcp-server", "Server"));
    network.updateConfig(session.id, { localPort: 0 });

    await expect(network.start(session.id)).rejects.toThrow("listenPortRange");
    expect(network.activeSession?.active).toBe(false);
  });

  it("only requires a target for clients", async () => {
    const network = useNetworkStore();
    const session = network.createSession(defaultNetworkSession("tcp-client", "Client"));
    network.updateConfig(session.id, { remoteHost: " " });

    await expect(network.start(session.id)).rejects.toThrow("remoteHostRequired");
  });

  it("starts, logs and stops one session without affecting another", async () => {
    const network = useNetworkStore();
    const first = network.createSession(defaultNetworkSession("tcp-server", "Server"));
    const second = network.createSession(defaultNetworkSession("udp-client", "Client"));
    network.updateConfig(first.id, { localPort: 18081 });
    network.updateConfig(second.id, { remoteHost: "127.0.0.1", remotePort: 9000 });

    await network.start(first.id);
    await network.start(second.id);
    expect(network.activeCount).toBe(2);
    expect(network.sessions.find((session) => session.id === first.id)?.active).toBe(true);
    expect(network.sessions.find((session) => session.id === second.id)?.active).toBe(true);

    network.applyData({
      sessionId: first.id,
      direction: "rx",
      data: [1, 2, 3],
      connection: "127.0.0.1:18081",
      timestamp: 1,
    });
    expect(network.sessions.find((session) => session.id === first.id)?.entries).toHaveLength(1);
    expect(network.sessions.find((session) => session.id === second.id)?.entries).toHaveLength(0);

    await network.stop(first.id);
    expect(network.activeCount).toBe(1);
  });

  it("keeps network session parameters after reopening", async () => {
    const network = useNetworkStore();
    const session = network.createSession(defaultNetworkSession("udp-client", "Probe"));
    network.updateConfig(session.id, {
      remoteHost: "10.0.0.9",
      remotePort: 9000,
      name: "Probe link",
    });
    await nextTick();

    const raw = localStorage.getItem(STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw ?? "{}") as {
      version: number;
      sessions: Array<Record<string, unknown>>;
    };
    expect(parsed.version).toBe(1);
    expect(parsed.sessions[0]?.name).toBe("Probe link");
    expect(parsed.sessions[0]?.remoteHost).toBe("10.0.0.9");
    expect(parsed.sessions[0]?.remotePort).toBe(9000);

    setActivePinia(createPinia());
    const restored = useNetworkStore();
    expect(restored.sessions).toHaveLength(1);
    expect(restored.activeSession?.name).toBe("Probe link");
    expect(restored.activeSession?.active).toBe(false);
  });
});
