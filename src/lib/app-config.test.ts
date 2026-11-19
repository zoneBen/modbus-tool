import { describe, expect, it } from "vitest";
import { APP_CONFIG_KIND, APP_CONFIG_VERSION, buildAppConfig, parseAppConfig } from "./app-config";
import type { PersistedSession } from "./persisted-workspace";

const session: PersistedSession = {
  name: "PLC-01",
  role: "master",
  transport: "tcp",
  framing: "mbap",
  endpoint: "127.0.0.1:502",
  host: "127.0.0.1",
  port: 502,
  timeoutMs: 1000,
  retries: 1,
  autoReconnect: true,
  definitions: [
    {
      name: "Holding",
      functionCode: 3,
      slaveId: 1,
      startAddress: 0,
      quantity: 8,
      dataType: "u16",
      byteOrder: "abcd",
      intervalMs: 1000,
      enabled: true,
      aliases: { 0: "Temperature" },
      points: [],
    },
  ],
};

const networkSession = {
  id: "net-1",
  name: "TCP client",
  mode: "tcp-client" as const,
  localHost: "0.0.0.0",
  localPort: 0,
  remoteHost: "127.0.0.1",
  remotePort: 502,
};

describe("app config", () => {
  it("round-trips settings, sessions and network sessions", () => {
    const config = buildAppConfig(
      { locale: "en-US", addressBase: 1 },
      [session],
      [networkSession],
      {
        version: 1,
        networkTool: "arp",
        devTool: "hash",
      },
    );
    const parsed = parseAppConfig(JSON.stringify(config));
    expect(parsed.kind).toBe(APP_CONFIG_KIND);
    expect(parsed.version).toBe(APP_CONFIG_VERSION);
    expect(parsed.settings).toEqual({ locale: "en-US", addressBase: 1 });
    expect(parsed.sessions).toHaveLength(1);
    expect(parsed.sessions[0]?.definitions[0]?.aliases).toEqual({ 0: "Temperature" });
    expect(parsed.networkSessions[0]?.mode).toBe("tcp-client");
    expect(parsed.tools?.networkTool).toBe("arp");
    expect(parsed.tools?.devTool).toBe("hash");
  });

  it("rejects malformed files with stable error codes", () => {
    expect(() => parseAppConfig("{")).toThrow("invalidJson");
    expect(() => parseAppConfig("{}")).toThrow("invalidConfigKind");
    expect(() => parseAppConfig(JSON.stringify({ kind: APP_CONFIG_KIND, version: 99 }))).toThrow(
      "unsupportedConfigVersion",
    );
    expect(() =>
      parseAppConfig(JSON.stringify({ kind: APP_CONFIG_KIND, version: APP_CONFIG_VERSION })),
    ).toThrow("invalidConfigShape");
  });

  it("drops invalid sessions while keeping valid ones", () => {
    const parsed = parseAppConfig(
      JSON.stringify({
        kind: APP_CONFIG_KIND,
        version: APP_CONFIG_VERSION,
        sessions: [session, { name: "broken", role: "master" }],
        networkSessions: [{ id: "net-1" }],
      }),
    );
    expect(parsed.sessions).toHaveLength(1);
    expect(parsed.networkSessions).toHaveLength(1);
    expect(parsed.tools).toBeUndefined();
  });
});
