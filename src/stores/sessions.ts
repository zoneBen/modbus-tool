import { defineStore } from "pinia";
import { i18n } from "@/i18n";
import { computed, ref, watch } from "vue";
import { toast } from "vue-sonner";
import {
  loadPersistedWorkspace,
  persistedSessionInput,
  savePersistedWorkspace,
  toPersistedSession,
  workspaceConfigSignature,
} from "@/lib/persisted-workspace";
import type { PersistedSession } from "@/lib/persisted-workspace";
import { ipc } from "@/lib/ipc";
import type {
  CustomWriteInput,
  Definition,
  DefinitionInput,
  DefinitionSnapshot,
  DefinitionTimeline,
  NewSessionInput,
  Session,
  SessionStatusEvent,
  SlaveValueInput,
  UpdateSessionInput,
  WorkspaceSnapshot,
} from "@/types/models";
import { historyKey, snapshotKey } from "@/lib/format";

const HISTORY_LIMIT = 40;
const TIMELINE_LIMIT = 1200;

export const useSessionsStore = defineStore("sessions", () => {
  const sessions = ref<Session[]>([]);
  const activeSessionId = ref<string | null>(null);
  const activeDefinitionId = ref<string | null>(null);
  const selectedAddress = ref<number | null>(null);
  const chartFocusAddress = ref<number | null>(null);
  const snapshots = ref<Record<string, DefinitionSnapshot>>({});
  const history = ref<Record<string, number[]>>({});
  const timelines = ref<Record<string, DefinitionTimeline>>({});
  const booted = ref(false);
  const bootError = ref<string | null>(null);
  const demo = ref(false);
  const lastUpdate = ref<number | null>(null);
  const restoring = ref(false);
  let persistTimer: ReturnType<typeof setTimeout> | null = null;

  function persistNow(): void {
    if (!booted.value || demo.value || restoring.value) return;
    savePersistedWorkspace(sessions.value);
  }

  function schedulePersist(): void {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      persistTimer = null;
      persistNow();
    }, 180);
  }

  watch(
    () => workspaceConfigSignature(sessions.value),
    () => schedulePersist(),
    { flush: "post" },
  );

  if (typeof window !== "undefined") {
    window.addEventListener("beforeunload", () => {
      if (persistTimer) {
        clearTimeout(persistTimer);
        persistTimer = null;
      }
      persistNow();
    });
  }

  const activeSession = computed<Session | null>(
    () => sessions.value.find((session) => session.id === activeSessionId.value) ?? null,
  );

  const activeDefinitions = computed<Definition[]>(() => activeSession.value?.definitions ?? []);

  const activeDefinition = computed<Definition | null>(
    () =>
      activeDefinitions.value.find((definition) => definition.id === activeDefinitionId.value) ??
      null,
  );

  const activeSnapshot = computed<DefinitionSnapshot | null>(() => {
    const session = activeSession.value;
    const definition = activeDefinition.value;
    if (!session || !definition) return null;
    return snapshots.value[snapshotKey(session.id, definition.id)] ?? null;
  });

  const connectedCount = computed(
    () => sessions.value.filter((session) => session.status === "connected").length,
  );

  const masterCount = computed(
    () => sessions.value.filter((session) => session.role === "master").length,
  );
  const slaveCount = computed(
    () => sessions.value.filter((session) => session.role === "slave").length,
  );

  function sessionName(sessionId: string): string {
    return (
      sessions.value.find((session) => session.id === sessionId)?.name ??
      i18n.global.t("errors.unknownSession")
    );
  }

  function sessionAccent(sessionId: string): Session["accent"] {
    return sessions.value.find((session) => session.id === sessionId)?.accent ?? "signal";
  }

  function applyWorkspace(workspace: WorkspaceSnapshot): void {
    sessions.value = workspace.sessions;
    demo.value = workspace.demo;
    let current = sessions.value.find((session) => session.id === activeSessionId.value);
    if (!current) {
      current = sessions.value.find((session) => session.role === "master") ?? sessions.value[0];
      activeSessionId.value = current?.id ?? null;
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }
    const definition = current?.definitions.find(
      (candidate) => candidate.id === activeDefinitionId.value,
    );
    if (!definition) {
      activeDefinitionId.value = current?.definitions[0]?.id ?? null;
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }
  }

  function refreshSessions(updater: (session: Session) => Session): void {
    sessions.value = sessions.value.map(updater);
  }

  function applyStatus(event: SessionStatusEvent): void {
    refreshSessions((session) =>
      session.id === event.sessionId
        ? {
            ...session,
            status: event.status,
            polling: event.polling,
            stats: event.stats,
          }
        : session,
    );
  }

  function applySnapshot(snapshot: DefinitionSnapshot): void {
    const key = snapshotKey(snapshot.sessionId, snapshot.definitionId);
    snapshots.value = { ...snapshots.value, [key]: snapshot };
    const nextHistory = { ...history.value };
    for (const value of snapshot.values) {
      if (value.numeric === null || !Number.isFinite(value.numeric)) continue;
      const hKey = historyKey(snapshot.definitionId, value.address);
      const previous = nextHistory[hKey] ?? [];
      nextHistory[hKey] = [...previous.slice(-(HISTORY_LIMIT - 1)), value.numeric];
    }
    history.value = nextHistory;

    const previousTimeline = timelines.value[snapshot.definitionId];
    const previousTimestamps = previousTimeline?.timestamps ?? [];
    const nextTimestamps = [...previousTimestamps, snapshot.timestamp].slice(-TIMELINE_LIMIT);
    const addresses = new Set<number>([
      ...Object.keys(previousTimeline?.series ?? {}).map(Number),
      ...snapshot.values.map((value) => value.address),
    ]);
    const nextSeries: Record<number, Array<number | null>> = {};
    for (const address of addresses) {
      const previous = previousTimeline?.series[address] ?? [];
      const value = snapshot.values.find((entry) => entry.address === address)?.numeric ?? null;
      let series = [...previous, value];
      if (series.length < nextTimestamps.length) {
        series = [
          ...new Array<number | null>(nextTimestamps.length - series.length).fill(null),
          ...series,
        ];
      }
      nextSeries[address] = series.slice(-TIMELINE_LIMIT);
    }
    timelines.value = {
      ...timelines.value,
      [snapshot.definitionId]: {
        timestamps: nextTimestamps,
        series: nextSeries,
      },
    };

    lastUpdate.value = snapshot.timestamp;
  }

  async function createPersistedSessions(
    persistedSessions: readonly PersistedSession[],
  ): Promise<string[]> {
    const errors: string[] = [];
    for (const persisted of persistedSessions) {
      try {
        const created = await ipc.createSession(persistedSessionInput(persisted));
        if (persisted.role === "slave") {
          const synthetic = created.definitions[0];
          const savedDefinition = persisted.definitions[0];
          if (synthetic && savedDefinition) {
            await ipc.updateDefinition(created.id, synthetic.id, savedDefinition);
          }
        } else {
          for (const definition of persisted.definitions) {
            try {
              await ipc.createDefinition(created.id, definition);
            } catch (cause: unknown) {
              errors.push(
                `${persisted.name} / ${definition.name}: ${
                  cause instanceof Error ? cause.message : String(cause)
                }`,
              );
            }
          }
        }
      } catch (cause: unknown) {
        errors.push(`${persisted.name}: ${cause instanceof Error ? cause.message : String(cause)}`);
      }
    }
    return errors;
  }

  async function restorePersistedWorkspace(): Promise<string[]> {
    const saved = loadPersistedWorkspace();
    if (!saved?.sessions.length) return [];
    restoring.value = true;
    try {
      const errors = await createPersistedSessions(saved.sessions);
      const workspace = await ipc.boot();
      applyWorkspace(workspace);
      return errors;
    } finally {
      restoring.value = false;
    }
  }

  function persistedSessions(): PersistedSession[] {
    return sessions.value.map(toPersistedSession);
  }

  async function replaceWorkspace(
    persistedSessionsToImport: readonly PersistedSession[],
  ): Promise<string[]> {
    const errors: string[] = [];
    restoring.value = true;
    try {
      const current = [...sessions.value];
      for (const session of current) {
        try {
          await ipc.deleteSession(session.id);
        } catch (cause: unknown) {
          errors.push(cause instanceof Error ? cause.message : String(cause));
        }
      }
      sessions.value = [];
      activeSessionId.value = null;
      activeDefinitionId.value = null;
      selectedAddress.value = null;
      chartFocusAddress.value = null;
      snapshots.value = {};
      history.value = {};
      timelines.value = {};
      lastUpdate.value = null;
      errors.push(...(await createPersistedSessions(persistedSessionsToImport)));
      const workspace = await ipc.boot();
      applyWorkspace(workspace);
      savePersistedWorkspace(sessions.value);
      return errors;
    } finally {
      restoring.value = false;
    }
  }

  async function boot(): Promise<void> {
    if (booted.value) return;
    try {
      const workspace = await ipc.boot();
      if (!workspace.demo && workspace.sessions.length === 0) {
        const errors = await restorePersistedWorkspace();
        if (errors.length) {
          bootError.value = errors.join("; ");
          toast.error(i18n.global.t("restore.partialFailed"), { description: errors[0] });
        } else if (sessions.value.length) {
          toast.success(i18n.global.t("restore.restored", { count: sessions.value.length }), {
            description: i18n.global.t("restore.restoredHint"),
          });
        }
      } else {
        applyWorkspace(workspace);
      }
      booted.value = true;
    } catch (error: unknown) {
      bootError.value = error instanceof Error ? error.message : String(error);
    }
  }

  function setActiveSession(sessionId: string): void {
    const session = sessions.value.find((candidate) => candidate.id === sessionId);
    if (!session) return;
    if (activeSessionId.value !== sessionId) {
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }
    activeSessionId.value = sessionId;
    const currentDefinition = session.definitions.find(
      (definition) => definition.id === activeDefinitionId.value,
    );
    activeDefinitionId.value = currentDefinition?.id ?? session.definitions[0]?.id ?? null;
  }

  function setActiveDefinition(definitionId: string): void {
    if (activeDefinitionId.value !== definitionId) {
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }
    activeDefinitionId.value = definitionId;
  }

  function selectAddress(address: number | null): void {
    selectedAddress.value = address;
  }

  function focusChart(address: number | null): void {
    chartFocusAddress.value = address;
    if (address !== null) selectedAddress.value = address;
  }

  async function togglePolling(): Promise<void> {
    const session = activeSession.value;
    if (!session) return;
    const enabled = !session.polling;
    applyStatus({
      sessionId: session.id,
      status: session.status,
      polling: enabled,
      stats: session.stats,
    });
    await ipc.setPolling(session.id, enabled);
  }

  async function readOnce(): Promise<void> {
    const session = activeSession.value;
    const definition = activeDefinition.value;
    if (!session || !definition) return;
    await ipc.readOnce(session.id, definition.id);
  }

  async function toggleDefinition(definitionId: string, enabled: boolean): Promise<void> {
    const session = activeSession.value;
    if (!session) return;
    sessions.value = sessions.value.map((candidate) =>
      candidate.id === session.id
        ? {
            ...candidate,
            definitions: candidate.definitions.map((definition) =>
              definition.id === definitionId ? { ...definition, enabled } : definition,
            ),
          }
        : candidate,
    );
    await ipc.setDefinitionEnabled(session.id, definitionId, enabled);
  }

  async function createDefinition(input: DefinitionInput): Promise<Definition> {
    const session = activeSession.value;
    if (!session) throw new Error(i18n.global.t("errors.noSession"));
    const definition = await ipc.createDefinition(session.id, input);
    sessions.value = sessions.value.map((candidate) => {
      if (candidate.id !== session.id) return candidate;
      if (candidate.definitions.some((entry) => entry.id === definition.id)) {
        return candidate;
      }
      return { ...candidate, definitions: [...candidate.definitions, definition] };
    });
    activeDefinitionId.value = definition.id;
    return definition;
  }

  async function updateDefinition(
    definitionId: string,
    input: DefinitionInput,
  ): Promise<Definition> {
    const session = activeSession.value;
    if (!session) throw new Error(i18n.global.t("errors.noSession"));
    const definition = await ipc.updateDefinition(session.id, definitionId, input);
    sessions.value = sessions.value.map((candidate) =>
      candidate.id === session.id
        ? {
            ...candidate,
            definitions: candidate.definitions.map((entry) =>
              entry.id === definitionId ? definition : entry,
            ),
          }
        : candidate,
    );
    return definition;
  }

  async function deleteDefinition(definitionId: string): Promise<void> {
    const session = activeSession.value;
    if (!session) throw new Error(i18n.global.t("errors.noSession"));
    await ipc.deleteDefinition(session.id, definitionId);
    const remaining = session.definitions.filter((definition) => definition.id !== definitionId);
    sessions.value = sessions.value.map((candidate) =>
      candidate.id === session.id ? { ...candidate, definitions: remaining } : candidate,
    );
    if (activeDefinitionId.value === definitionId) {
      activeDefinitionId.value = remaining[0]?.id ?? null;
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }

    const nextSnapshots = { ...snapshots.value };
    delete nextSnapshots[snapshotKey(session.id, definitionId)];
    snapshots.value = nextSnapshots;

    const nextTimelines = { ...timelines.value };
    delete nextTimelines[definitionId];
    timelines.value = nextTimelines;

    const nextHistory = { ...history.value };
    for (const key of Object.keys(nextHistory)) {
      if (key.startsWith(`${definitionId}:`)) delete nextHistory[key];
    }
    history.value = nextHistory;
  }

  async function createSession(input: NewSessionInput): Promise<Session> {
    const session = await ipc.createSession(input);
    if (!sessions.value.some((candidate) => candidate.id === session.id)) {
      sessions.value = [...sessions.value, session];
    }
    activeSessionId.value = session.id;
    activeDefinitionId.value = session.definitions[0]?.id ?? null;
    selectedAddress.value = null;
    chartFocusAddress.value = null;
    return session;
  }

  async function deleteSession(sessionId: string): Promise<void> {
    const removed = sessions.value.find((session) => session.id === sessionId);
    await ipc.deleteSession(sessionId);
    sessions.value = sessions.value.filter((session) => session.id !== sessionId);
    if (activeSessionId.value === sessionId) {
      const next = sessions.value.find((session) => session.role === "master") ?? sessions.value[0];
      activeSessionId.value = next?.id ?? null;
      activeDefinitionId.value = next?.definitions[0]?.id ?? null;
      selectedAddress.value = null;
      chartFocusAddress.value = null;
    }
    if (!removed) return;
    const nextSnapshots = { ...snapshots.value };
    for (const key of Object.keys(nextSnapshots)) {
      if (key.startsWith(`${sessionId}:`)) delete nextSnapshots[key];
    }
    snapshots.value = nextSnapshots;
    const nextTimelines = { ...timelines.value };
    const nextHistory = { ...history.value };
    for (const definition of removed.definitions) {
      delete nextTimelines[definition.id];
      for (const key of Object.keys(nextHistory)) {
        if (key.startsWith(`${definition.id}:`)) delete nextHistory[key];
      }
    }
    timelines.value = nextTimelines;
    history.value = nextHistory;
  }

  async function connectSession(sessionId: string): Promise<void> {
    refreshSessions((session) =>
      session.id === sessionId ? { ...session, status: "connecting" } : session,
    );
    await ipc.connectSession(sessionId);
  }

  async function disconnectSession(sessionId: string): Promise<void> {
    await ipc.disconnectSession(sessionId);
    refreshSessions((session) =>
      session.id === sessionId ? { ...session, status: "disconnected", polling: false } : session,
    );
  }

  async function updateSession(sessionId: string, input: UpdateSessionInput): Promise<void> {
    await ipc.updateSession(sessionId, input);
    refreshSessions((session) =>
      session.id === sessionId
        ? {
            ...session,
            name: input.name.trim() || session.name,
            transport: input.transport,
            framing: input.framing,
            endpoint: input.endpoint,
          }
        : session,
    );
  }

  async function writeSlaveValue(input: SlaveValueInput): Promise<number> {
    const session = activeSession.value;
    if (!session) throw new Error(i18n.global.t("errors.noSession"));
    if (session.role !== "slave") throw new Error(i18n.global.t("errors.slaveValueOnly"));
    return ipc.writeSlaveValue(session.id, input);
  }

  async function writeRegister(address: number, value: string): Promise<void> {
    const session = activeSession.value;
    const definition = activeDefinition.value;
    if (!session || !definition) throw new Error(i18n.global.t("errors.noDefinition"));
    await ipc.writeRegister(session.id, definition.id, address, value);
  }

  async function writeCustom(input: CustomWriteInput): Promise<void> {
    const session = activeSession.value;
    const definition = activeDefinition.value;
    if (!session) throw new Error(i18n.global.t("errors.noSession"));
    await ipc.writeCustom(session.id, definition?.id ?? null, input);
  }

  return {
    sessions,
    activeSessionId,
    activeDefinitionId,
    selectedAddress,
    chartFocusAddress,
    snapshots,
    history,
    timelines,
    booted,
    bootError,
    demo,
    lastUpdate,
    activeSession,
    activeDefinitions,
    activeDefinition,
    activeSnapshot,
    connectedCount,
    masterCount,
    slaveCount,
    sessionName,
    sessionAccent,
    applyWorkspace,
    applyStatus,
    applySnapshot,
    boot,
    persistedSessions,
    replaceWorkspace,
    setActiveSession,
    setActiveDefinition,
    selectAddress,
    focusChart,
    togglePolling,
    readOnce,
    toggleDefinition,
    createDefinition,
    updateDefinition,
    deleteDefinition,
    createSession,
    deleteSession,
    connectSession,
    disconnectSession,
    updateSession,
    writeSlaveValue,
    writeRegister,
    writeCustom,
  };
});
