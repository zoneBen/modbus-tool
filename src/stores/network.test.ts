import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { useNetworkStore } from "@/stores/network";

const STORAGE_KEY = "mt.network.config.v1";

describe("network store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("validates the listening port before starting a server", async () => {
    const network = useNetworkStore();
    network.mode = "tcp-server";
    network.localPort = 0;

    await expect(network.start()).rejects.toThrow("listenPortRange");
    expect(network.active).toBe(false);
  });

  it("requires a remote endpoint for clients", async () => {
    const network = useNetworkStore();
    network.mode = "tcp-client";
    network.remoteHost = " ";
    network.remotePort = 502;

    await expect(network.start()).rejects.toThrow("remoteHostRequired");
  });

  it("starts and stops through the demo bridge", async () => {
    const network = useNetworkStore();
    network.mode = "tcp-server";
    network.localPort = 18080;

    await network.start();
    expect(network.active).toBe(true);
    expect(network.clients).toBe(1);

    await network.stop();
    expect(network.active).toBe(false);
  });

  it("keeps the last parameters after reopening", async () => {
    const network = useNetworkStore();
    network.mode = "udp-client";
    network.remoteHost = "10.0.0.9";
    network.remotePort = 9000;
    network.localHost = "127.0.0.1";
    network.localPort = 19000;
    await nextTick();

    const raw = localStorage.getItem(STORAGE_KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw ?? "{}") as Record<string, unknown>;
    expect(parsed.mode).toBe("udp-client");
    expect(parsed.remoteHost).toBe("10.0.0.9");
    expect(parsed.remotePort).toBe(9000);
  });

  it("records received bytes and enforces the entry limit", () => {
    const network = useNetworkStore();
    network.applyData({
      direction: "rx",
      data: [1, 2, 3],
      connection: "127.0.0.1:502",
      timestamp: 1,
    });
    network.applyData({
      direction: "tx",
      data: [4],
      connection: "127.0.0.1:502",
      timestamp: 2,
    });

    expect(network.entries).toHaveLength(2);
    expect(network.rxBytes).toBe(3);
    expect(network.txBytes).toBe(1);
    network.clear();
    expect(network.entries).toHaveLength(0);
  });
});
