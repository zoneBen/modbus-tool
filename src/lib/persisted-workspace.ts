import { toDefinitionInput } from "@/lib/definition";
import type { DefinitionInput, NewSessionInput, Session } from "@/types/models";

const STORAGE_KEY = "mt.workspace.v1";

export interface PersistedSession {
  name: string;
  role: Session["role"];
  transport: Session["transport"];
  framing: Session["framing"];
  endpoint: string;
  host?: string;
  port?: number;
  serialPort?: string;
  baudRate?: number;
  dataBits?: number;
  parity?: Session["parity"];
  stopBits?: number;
  timeoutMs?: number;
  retries?: number;
  interFrameDelayMs?: number;
  autoReconnect?: boolean;
  definitions: DefinitionInput[];
}

export interface PersistedWorkspace {
  version: 1;
  savedAt: number;
  sessions: PersistedSession[];
}

function toPersistedSession(session: Session): PersistedSession {
  return {
    name: session.name,
    role: session.role,
    transport: session.transport,
    framing: session.framing,
    endpoint: session.endpoint,
    host: session.host,
    port: session.port,
    serialPort: session.serialPort,
    baudRate: session.baudRate,
    dataBits: session.dataBits,
    parity: session.parity,
    stopBits: session.stopBits,
    timeoutMs: session.timeoutMs,
    retries: session.retries,
    interFrameDelayMs: session.interFrameDelayMs,
    autoReconnect: session.autoReconnect,
    definitions: session.definitions.map(toDefinitionInput),
  };
}

export function workspaceConfigSignature(sessions: readonly Session[]): string {
  return JSON.stringify(sessions.map(toPersistedSession));
}

export function savePersistedWorkspace(sessions: readonly Session[]): void {
  if (typeof localStorage === "undefined") return;
  if (!sessions.length) {
    localStorage.removeItem(STORAGE_KEY);
    return;
  }
  const payload: PersistedWorkspace = {
    version: 1,
    savedAt: Date.now(),
    sessions: sessions.map(toPersistedSession),
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

export function loadPersistedWorkspace(): PersistedWorkspace | null {
  if (typeof localStorage === "undefined") return null;
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<PersistedWorkspace>;
    if (parsed.version !== 1 || !Array.isArray(parsed.sessions)) return null;
    return {
      version: 1,
      savedAt: Number(parsed.savedAt) || 0,
      sessions: parsed.sessions as PersistedSession[],
    };
  } catch {
    return null;
  }
}

export function persistedSessionInput(session: PersistedSession): NewSessionInput {
  return {
    name: session.name,
    role: session.role,
    transport: session.transport,
    framing: session.framing,
    endpoint: session.endpoint,
    host: session.host,
    port: session.port,
    serialPort: session.serialPort,
    baudRate: session.baudRate,
    dataBits: session.dataBits,
    parity: session.parity,
    stopBits: session.stopBits,
    timeoutMs: session.timeoutMs,
    retries: session.retries,
    interFrameDelayMs: session.interFrameDelayMs,
    autoReconnect: session.autoReconnect,
    slaveId: session.role === "slave" ? (session.definitions[0]?.slaveId ?? 1) : undefined,
    autoConnect: false,
  };
}

export function clearPersistedWorkspace(): void {
  if (typeof localStorage === "undefined") return;
  localStorage.removeItem(STORAGE_KEY);
}
