import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { ipc } from "@/lib/ipc";
import type {
  NetworkData,
  NetworkMode,
  NetworkStartInput,
  NetworkState,
  RawSendInput,
} from "@/types/models";

const STORAGE_KEY = "mt.network.sessions.v1";
const LEGACY_KEY = "mt.network.config.v1";
const MAX_ENTRIES = 2000;

const MODES: NetworkMode[] = ["tcp-server", "tcp-client", "udp-server", "udp-client", "tcp-relay"];

export interface NetworkSessionConfig {
  id: string;
  name: string;
  mode: NetworkMode;
  localHost: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
}

export interface NetworkEntry {
  id: number;
  direction: "tx" | "rx";
  data: number[];
  connection: string;
  timestamp: number;
}

export interface NetworkSession extends NetworkSessionConfig {
  active: boolean;
  busy: boolean;
  endpoint: string | null;
  clients: number;
  error: string;
  entries: NetworkEntry[];
}

type PersistedNetworkSession = NetworkSessionConfig;

interface PersistedNetworkWorkspace {
  version: 1;
  activeId: string | null;
  sessions: PersistedNetworkSession[];
}

let entrySequence = 0;

function newSessionId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `net-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function defaultLocalPort(mode: NetworkMode): number {
  return mode === "tcp-server" || mode === "udp-server" || mode === "tcp-relay" ? 8080 : 0;
}

function defaultRemoteHost(mode: NetworkMode): string {
  return mode === "tcp-client" || mode === "udp-client" || mode === "tcp-relay" ? "127.0.0.1" : "";
}

function defaultRemotePort(mode: NetworkMode): number {
  return mode === "tcp-client" || mode === "udp-client" || mode === "tcp-relay" ? 502 : 0;
}

export function defaultNetworkSession(mode: NetworkMode, name: string): NetworkSession {
  return {
    id: newSessionId(),
    name,
    mode,
    localHost: "0.0.0.0",
    localPort: defaultLocalPort(mode),
    remoteHost: defaultRemoteHost(mode),
    remotePort: defaultRemotePort(mode),
    active: false,
    busy: false,
    endpoint: null,
    clients: 0,
    error: "",
    entries: [],
  };
}

function parsePort(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 65535
    ? value
    : fallback;
}

function parseSession(value: unknown): NetworkSession | null {
  if (!value || typeof value !== "object") return null;
  const record = value as Partial<PersistedNetworkSession>;
  const mode = MODES.includes(record.mode ?? "tcp-server")
    ? (record.mode as NetworkMode)
    : "tcp-server";
  return {
    id: typeof record.id === "string" && record.id ? record.id : newSessionId(),
    name: typeof record.name === "string" && record.name.trim() ? record.name : "Network",
    mode,
    localHost: typeof record.localHost === "string" ? record.localHost : "0.0.0.0",
    localPort: parsePort(record.localPort, defaultLocalPort(mode)),
    remoteHost: typeof record.remoteHost === "string" ? record.remoteHost : defaultRemoteHost(mode),
    remotePort: parsePort(record.remotePort, defaultRemotePort(mode)),
    active: false,
    busy: false,
    endpoint: null,
    clients: 0,
    error: "",
    entries: [],
  };
}

function loadWorkspace(): { sessions: NetworkSession[]; activeId: string | null } {
  if (typeof localStorage === "undefined") return { sessions: [], activeId: null };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<PersistedNetworkWorkspace>;
      if (parsed.version === 1 && Array.isArray(parsed.sessions)) {
        const sessions = parsed.sessions
          .map(parseSession)
          .filter((session): session is NetworkSession => session !== null);
        const activeId =
          typeof parsed.activeId === "string" &&
          sessions.some((session) => session.id === parsed.activeId)
            ? parsed.activeId
            : (sessions[0]?.id ?? null);
        return { sessions, activeId };
      }
    }
    const legacy = localStorage.getItem(LEGACY_KEY);
    if (legacy) {
      const parsed = JSON.parse(legacy) as Record<string, unknown>;
      const session = parseSession({
        id: newSessionId(),
        name: "Network 01",
        mode: parsed.mode,
        localHost: parsed.localHost,
        localPort: parsed.localPort,
        remoteHost: parsed.remoteHost,
        remotePort: parsed.remotePort,
      });
      if (session) return { sessions: [session], activeId: session.id };
    }
  } catch {
    // A malformed persisted workspace falls back to an empty assistant.
  }
  return { sessions: [], activeId: null };
}

function persistedConfig(session: NetworkSession): PersistedNetworkSession {
  return {
    id: session.id,
    name: session.name,
    mode: session.mode,
    localHost: session.localHost,
    localPort: session.localPort,
    remoteHost: session.remoteHost,
    remotePort: session.remotePort,
  };
}

function startInput(session: NetworkSession): NetworkStartInput {
  const server = session.mode === "tcp-server" || session.mode === "udp-server";
  const relay = session.mode === "tcp-relay";
  return {
    mode: session.mode,
    localHost: server || relay ? session.localHost.trim() : "",
    localPort: server || relay ? session.localPort : 0,
    remoteHost: server ? undefined : session.remoteHost.trim() || undefined,
    remotePort: server ? undefined : session.remotePort,
  };
}

function validateSession(session: NetworkSession): string | null {
  const server = session.mode === "tcp-server" || session.mode === "udp-server";
  const relay = session.mode === "tcp-relay";
  if (server && (session.localPort < 1 || session.localPort > 65535)) {
    return "listenPortRange";
  }
  if (!server && !session.remoteHost.trim()) {
    return "remoteHostRequired";
  }
  if (!server && (session.remotePort < 1 || session.remotePort > 65535)) {
    return "remotePortRange";
  }
  if (relay && (session.localPort < 1 || session.localPort > 65535)) {
    return "listenPortRange";
  }
  return null;
}

export const useNetworkStore = defineStore("network", () => {
  const initial = loadWorkspace();
  const sessions = ref<NetworkSession[]>(initial.sessions);
  const activeSessionId = ref<string | null>(initial.activeId);

  const activeSession = computed<NetworkSession | null>(
    () => sessions.value.find((session) => session.id === activeSessionId.value) ?? null,
  );
  const activeCount = computed(() => sessions.value.filter((session) => session.active).length);
  const configSignature = computed(() => JSON.stringify(sessions.value.map(persistedConfig)));

  function persist(): void {
    if (typeof localStorage === "undefined") return;
    const payload: PersistedNetworkWorkspace = {
      version: 1,
      activeId: activeSessionId.value,
      sessions: sessions.value.map(persistedConfig),
    };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
    } catch {
      // Persistence is best effort; the assistant still works without it.
    }
  }

  watch(configSignature, persist, { flush: "post" });
  watch(activeSessionId, persist, { flush: "post" });

  function sessionById(sessionId: string): NetworkSession | undefined {
    return sessions.value.find((session) => session.id === sessionId);
  }

  function selectSession(sessionId: string): void {
    if (sessionById(sessionId)) activeSessionId.value = sessionId;
  }

  function createSession(session: NetworkSession): NetworkSession {
    sessions.value = [...sessions.value, session];
    activeSessionId.value = session.id;
    persist();
    return session;
  }

  function updateConfig(sessionId: string, patch: Partial<NetworkSessionConfig>): void {
    const session = sessionById(sessionId);
    if (!session) return;
    Object.assign(session, patch);
  }

  async function deleteSession(sessionId: string): Promise<void> {
    const session = sessionById(sessionId);
    if (!session) return;
    if (session.active) {
      await stop(sessionId);
    }
    sessions.value = sessions.value.filter((entry) => entry.id !== sessionId);
    if (activeSessionId.value === sessionId) {
      activeSessionId.value = sessions.value[0]?.id ?? null;
    }
    persist();
  }

  async function start(sessionId: string): Promise<void> {
    const session = sessionById(sessionId);
    if (!session || session.busy || session.active) return;
    const problem = validateSession(session);
    if (problem) throw new Error(problem);
    session.busy = true;
    session.error = "";
    try {
      const state = await ipc.startNetwork(sessionId, startInput(session));
      applyState(state);
      if (!state.active) throw new Error(state.error ?? "networkStartFailed");
    } catch (error) {
      if (error instanceof Error && session.error === "") {
        session.error = error.message;
      }
      throw error;
    } finally {
      session.busy = false;
    }
  }

  async function stop(sessionId: string): Promise<void> {
    const session = sessionById(sessionId);
    if (!session || session.busy) return;
    session.busy = true;
    try {
      await ipc.stopNetwork(sessionId);
      session.active = false;
      session.clients = 0;
    } finally {
      session.busy = false;
    }
  }

  async function send(sessionId: string, payload: RawSendInput): Promise<number> {
    const session = sessionById(sessionId);
    if (!session?.active) throw new Error("networkNotRunning");
    return ipc.sendNetwork(sessionId, payload);
  }

  function applyState(state: NetworkState): void {
    const session = sessionById(state.sessionId);
    if (!session) return;
    session.active = state.active;
    session.endpoint = state.endpoint;
    session.clients = state.clients;
    if (state.error) session.error = state.error;
    else if (state.active) session.error = "";
  }

  function applyData(data: NetworkData): void {
    if (!data.data.length) return;
    const session = sessionById(data.sessionId);
    if (!session) return;
    const entry: NetworkEntry = {
      id: ++entrySequence,
      direction: data.direction,
      data: [...data.data],
      connection: data.connection,
      timestamp: data.timestamp,
    };
    const next = [...session.entries, entry];
    session.entries = next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
  }

  function clear(sessionId: string): void {
    const session = sessionById(sessionId);
    if (session) session.entries = [];
  }

  function resetError(sessionId: string): void {
    const session = sessionById(sessionId);
    if (session) session.error = "";
  }

  function rxBytes(session: NetworkSession): number {
    return session.entries
      .filter((entry) => entry.direction === "rx")
      .reduce((total, entry) => total + entry.data.length, 0);
  }

  function txBytes(session: NetworkSession): number {
    return session.entries
      .filter((entry) => entry.direction === "tx")
      .reduce((total, entry) => total + entry.data.length, 0);
  }

  return {
    sessions,
    activeSessionId,
    activeSession,
    activeCount,
    selectSession,
    createSession,
    updateConfig,
    deleteSession,
    start,
    stop,
    send,
    applyState,
    applyData,
    clear,
    resetError,
    rxBytes,
    txBytes,
  };
});
