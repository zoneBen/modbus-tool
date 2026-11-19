import type { AppSettingsSnapshot } from "@/stores/settings";
import type { NetworkSessionConfig } from "@/stores/network";
import type { PersistedSession } from "@/lib/persisted-workspace";
import type { ToolboxConfig } from "@/stores/toolbox";
import type { DefinitionInput } from "@/types/models";

export const APP_CONFIG_KIND = "modbus-tool-config";
export const APP_CONFIG_VERSION = 1;

export interface AppConfigFile {
  kind: typeof APP_CONFIG_KIND;
  version: typeof APP_CONFIG_VERSION;
  exportedAt: string;
  settings: Partial<AppSettingsSnapshot>;
  sessions: PersistedSession[];
  networkSessions: NetworkSessionConfig[];
  tools?: ToolboxConfig;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function normalizeSession(value: unknown): PersistedSession | null {
  if (!isRecord(value)) return null;
  if (
    typeof value.name !== "string" ||
    (value.role !== "master" && value.role !== "slave") ||
    (value.transport !== "serial" && value.transport !== "tcp" && value.transport !== "udp") ||
    (value.framing !== "rtu" && value.framing !== "ascii" && value.framing !== "mbap")
  ) {
    return null;
  }
  const definitions = Array.isArray(value.definitions)
    ? (value.definitions.filter(isRecord) as unknown as DefinitionInput[])
    : [];
  return {
    ...(value as unknown as PersistedSession),
    endpoint: typeof value.endpoint === "string" ? value.endpoint : "",
    definitions,
  };
}

export function parseAppConfig(text: string): AppConfigFile {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("invalidJson");
  }
  if (!isRecord(parsed) || parsed.kind !== APP_CONFIG_KIND) {
    throw new Error("invalidConfigKind");
  }
  if (parsed.version !== APP_CONFIG_VERSION) {
    throw new Error("unsupportedConfigVersion");
  }
  if (!Array.isArray(parsed.sessions) || !Array.isArray(parsed.networkSessions)) {
    throw new Error("invalidConfigShape");
  }
  const sessions = parsed.sessions
    .map(normalizeSession)
    .filter((session): session is PersistedSession => session !== null);
  const networkSessions = parsed.networkSessions.filter(
    isRecord,
  ) as unknown as NetworkSessionConfig[];
  return {
    kind: APP_CONFIG_KIND,
    version: APP_CONFIG_VERSION,
    exportedAt: typeof parsed.exportedAt === "string" ? parsed.exportedAt : "",
    settings: isRecord(parsed.settings) ? (parsed.settings as Partial<AppSettingsSnapshot>) : {},
    sessions,
    networkSessions,
    tools: isRecord(parsed.tools) ? (parsed.tools as unknown as ToolboxConfig) : undefined,
  };
}

export function buildAppConfig(
  settings: Partial<AppSettingsSnapshot>,
  sessions: PersistedSession[],
  networkSessions: NetworkSessionConfig[],
  tools?: ToolboxConfig,
): AppConfigFile {
  return {
    kind: APP_CONFIG_KIND,
    version: APP_CONFIG_VERSION,
    exportedAt: new Date().toISOString(),
    settings,
    sessions,
    networkSessions,
    tools,
  };
}
