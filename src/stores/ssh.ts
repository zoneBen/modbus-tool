import { defineStore } from "pinia";
import { reactive, ref, watch } from "vue";
import { ipc } from "@/lib/ipc";
import { toToolError } from "@/types/tools";
import type {
  SshCwdEvent,
  SshSessionState,
  SshSessionStatus,
  SshTransferEvent,
} from "@/types/tools";

const STORAGE_KEY = "mt.ssh.v1";

export interface SshProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth: "password" | "key";
  keyPath: string;
  cols: number;
  rows: number;
  followCwd: boolean;
}

interface PersistedSsh {
  version: 1;
  profiles: SshProfile[];
  activeId: string | null;
}

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  return `ssh-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function defaultSshProfile(index: number): SshProfile {
  return {
    id: newId(),
    name: `SSH ${index}`,
    host: "",
    port: 22,
    username: "root",
    auth: "password",
    keyPath: "",
    cols: 120,
    rows: 32,
    followCwd: false,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function sanitizeProfile(value: unknown): SshProfile | null {
  if (!isRecord(value) || typeof value.id !== "string") return null;
  return {
    id: value.id,
    name: typeof value.name === "string" ? value.name : "SSH",
    host: typeof value.host === "string" ? value.host : "",
    port: typeof value.port === "number" ? Math.min(65_535, Math.max(1, value.port)) : 22,
    username: typeof value.username === "string" ? value.username : "root",
    auth: value.auth === "key" ? "key" : "password",
    keyPath: typeof value.keyPath === "string" ? value.keyPath : "",
    cols: typeof value.cols === "number" ? Math.min(500, Math.max(20, value.cols)) : 120,
    rows: typeof value.rows === "number" ? Math.min(300, Math.max(4, value.rows)) : 32,
    followCwd: value.followCwd === true,
  };
}

function load(): PersistedSsh {
  if (typeof localStorage === "undefined") return { version: 1, profiles: [], activeId: null };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { version: 1, profiles: [], activeId: null };
    const parsed = JSON.parse(raw) as Partial<PersistedSsh>;
    const profiles = Array.isArray(parsed.profiles)
      ? parsed.profiles.map(sanitizeProfile).filter((item): item is SshProfile => !!item)
      : [];
    const activeId =
      typeof parsed.activeId === "string" && profiles.some((item) => item.id === parsed.activeId)
        ? parsed.activeId
        : (profiles[0]?.id ?? null);
    return { version: 1, profiles, activeId };
  } catch {
    return { version: 1, profiles: [], activeId: null };
  }
}

export const useSshStore = defineStore("ssh", () => {
  const initial = load();
  const profiles = ref<SshProfile[]>(initial.profiles);
  const activeId = ref<string | null>(initial.activeId);
  const statuses = reactive<Record<string, SshSessionStatus>>({});
  const details = reactive<Record<string, string | null>>({});
  const sessionIds = reactive<Record<string, string | null>>({});
  const cwds = reactive<Record<string, string>>({});
  const passwords = reactive<Record<string, string>>({});
  const passphrases = reactive<Record<string, string>>({});
  const transfers = reactive<Record<string, SshTransferEvent | null>>({});
  const connectedAt = reactive<Record<string, number>>({});
  let initialized = false;

  function persist(): void {
    if (typeof localStorage === "undefined") return;
    const payload: PersistedSsh = {
      version: 1,
      profiles: profiles.value,
      activeId: activeId.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  }

  watch([profiles, activeId], persist, { deep: true });

  function setStatus(
    profileId: string,
    status: SshSessionStatus,
    detail: string | null = null,
  ): void {
    statuses[profileId] = status;
    details[profileId] = detail;
  }

  function profileBySession(sessionId: string): SshProfile | null {
    return profiles.value.find((profile) => sessionIds[profile.id] === sessionId) ?? null;
  }

  function applyState(state: SshSessionState): void {
    const profile = profileBySession(state.sessionId);
    if (!profile) {
      if (state.status === "error") {
        const connecting = profiles.value.find((item) => statuses[item.id] === "connecting");
        if (connecting) {
          setStatus(connecting.id, "error", state.detail ?? null);
        }
      }
      return;
    }
    setStatus(profile.id, state.status, state.detail ?? null);
    if (state.status === "connected") {
      sessionIds[profile.id] = state.sessionId;
      connectedAt[profile.id] = state.connectedAtMs;
    }
    if (state.cwd) cwds[profile.id] = state.cwd;
  }

  function applyCwd(event: SshCwdEvent): void {
    const profile = profileBySession(event.sessionId);
    if (profile) cwds[profile.id] = event.path;
  }

  function applyTransfer(event: SshTransferEvent): void {
    const profile = profileBySession(event.sessionId);
    if (!profile) return;
    transfers[profile.id] = event;
    if (event.status === "done" || event.status === "error") {
      window.setTimeout(() => {
        if (transfers[profile.id]?.timestampMs === event.timestampMs) {
          transfers[profile.id] = null;
        }
      }, 2400);
    }
  }

  async function syncSessions(): Promise<void> {
    const states = await ipc.toolSshList();
    for (const state of states) {
      const profile =
        profiles.value.find((item) => sessionIds[item.id] === state.sessionId) ??
        profiles.value.find(
          (item) =>
            !sessionIds[item.id] &&
            item.host === state.host &&
            item.port === state.port &&
            item.username === state.username,
        ) ??
        (state.name
          ? profiles.value.find((item) => item.name === state.name && !sessionIds[item.id])
          : undefined);
      if (!profile) continue;
      sessionIds[profile.id] = state.sessionId;
      setStatus(profile.id, state.status, state.detail ?? null);
    }
  }

  async function init(): Promise<void> {
    if (initialized) return;
    initialized = true;
    await ipc.on("ssh:state", applyState);
    await ipc.on("ssh:cwd", applyCwd);
    await ipc.on("ssh:transfer", applyTransfer);
    await syncSessions().catch(() => undefined);
  }

  function addProfile(): SshProfile {
    const profile = defaultSshProfile(profiles.value.length + 1);
    profiles.value = [...profiles.value, profile];
    activeId.value = profile.id;
    setStatus(profile.id, "disconnected");
    return profile;
  }

  function updateProfile(profileId: string, patch: Partial<SshProfile>): void {
    profiles.value = profiles.value.map((profile) =>
      profile.id === profileId ? { ...profile, ...patch, id: profile.id } : profile,
    );
  }

  function getProfile(profileId: string | null): SshProfile | undefined {
    return profileId ? profiles.value.find((profile) => profile.id === profileId) : undefined;
  }

  async function removeProfile(profileId: string): Promise<void> {
    await disconnect(profileId).catch(() => undefined);
    profiles.value = profiles.value.filter((profile) => profile.id !== profileId);
    delete statuses[profileId];
    delete details[profileId];
    delete sessionIds[profileId];
    delete cwds[profileId];
    delete passwords[profileId];
    delete passphrases[profileId];
    delete transfers[profileId];
    delete connectedAt[profileId];
    if (activeId.value === profileId) activeId.value = profiles.value[0]?.id ?? null;
  }

  function isRunning(profileId: string): boolean {
    const status = statuses[profileId];
    return status === "connecting" || status === "connected";
  }

  async function connect(profileId: string): Promise<SshSessionState> {
    const profile = getProfile(profileId);
    if (!profile) throw { code: "invalidSession", detail: "SSH profile not found" };
    setStatus(profileId, "connecting");
    details[profileId] = null;
    try {
      const state = await ipc.toolSshConnect({
        name: profile.name,
        host: profile.host.trim(),
        port: profile.port,
        username: profile.username.trim(),
        auth: profile.auth,
        password: profile.auth === "password" ? passwords[profileId] || undefined : undefined,
        keyPath: profile.auth === "key" ? profile.keyPath.trim() || undefined : undefined,
        keyPassphrase: profile.auth === "key" ? passphrases[profileId] || undefined : undefined,
        cols: profile.cols,
        rows: profile.rows,
        timeoutMs: 15_000,
        followCwd: profile.followCwd,
      });
      sessionIds[profileId] = state.sessionId;
      connectedAt[profileId] = state.connectedAtMs;
      if (state.cwd) cwds[profileId] = state.cwd;
      setStatus(profileId, state.status, state.detail ?? null);
      return state;
    } catch (cause: unknown) {
      const error = toToolError(cause);
      setStatus(profileId, "error", `${error.code}: ${error.detail}`);
      throw cause;
    }
  }

  async function disconnect(profileId: string): Promise<boolean> {
    const sessionId = sessionIds[profileId];
    if (!sessionId) {
      setStatus(profileId, "disconnected");
      return false;
    }
    const result = await ipc.toolSshDisconnect(sessionId);
    setStatus(profileId, "disconnected");
    return result;
  }

  async function setCwdTracking(profileId: string, enabled: boolean): Promise<void> {
    const sessionId = sessionIds[profileId];
    if (!sessionId) return;
    await ipc.toolSshSetCwdTracking(sessionId, enabled);
  }

  return {
    profiles,
    activeId,
    statuses,
    details,
    sessionIds,
    cwds,
    passwords,
    passphrases,
    transfers,
    connectedAt,
    init,
    syncSessions,
    addProfile,
    updateProfile,
    getProfile,
    removeProfile,
    isRunning,
    connect,
    disconnect,
    setCwdTracking,
    applyState,
    applyCwd,
    applyTransfer,
    setStatus,
    profileBySession,
  };
});
