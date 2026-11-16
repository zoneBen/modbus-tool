import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { ipc } from "@/lib/ipc";
import type {
  Definition,
  DefinitionInput,
  DefinitionSnapshot,
  DefinitionTimeline,
  NewSessionInput,
  Session,
  SessionStatusEvent,
  WorkspaceSnapshot,
} from "@/types/models";
import { historyKey, snapshotKey } from "@/lib/format";

const HISTORY_LIMIT = 40;
const TIMELINE_LIMIT = 1200;

export const useSessionsStore = defineStore("sessions", () => {
  const sessions = ref<Session[]>([]);
  const activeSessionId = ref<string | null>(null);
  const activeDefinitionId = ref<string | null>(null);
  const snapshots = ref<Record<string, DefinitionSnapshot>>({});
  const history = ref<Record<string, number[]>>({});
  const timelines = ref<Record<string, DefinitionTimeline>>({});
  const booted = ref(false);
  const bootError = ref<string | null>(null);
  const demo = ref(false);
  const lastUpdate = ref<number | null>(null);

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
    return sessions.value.find((session) => session.id === sessionId)?.name ?? "未知会话";
  }

  function sessionAccent(sessionId: string): Session["accent"] {
    return sessions.value.find((session) => session.id === sessionId)?.accent ?? "signal";
  }

  function applyWorkspace(workspace: WorkspaceSnapshot): void {
    sessions.value = workspace.sessions;
    demo.value = workspace.demo;
    if (!sessions.value.some((session) => session.id === activeSessionId.value)) {
      const first =
        sessions.value.find((session) => session.role === "master") ?? sessions.value[0];
      activeSessionId.value = first?.id ?? null;
      activeDefinitionId.value = first?.definitions[0]?.id ?? null;
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

  async function boot(): Promise<void> {
    if (booted.value) return;
    try {
      const workspace = await ipc.boot();
      applyWorkspace(workspace);
      booted.value = true;
    } catch (error: unknown) {
      bootError.value = error instanceof Error ? error.message : String(error);
    }
  }

  function setActiveSession(sessionId: string): void {
    const session = sessions.value.find((candidate) => candidate.id === sessionId);
    if (!session) return;
    activeSessionId.value = sessionId;
    const currentDefinition = session.definitions.find(
      (definition) => definition.id === activeDefinitionId.value,
    );
    activeDefinitionId.value = currentDefinition?.id ?? session.definitions[0]?.id ?? null;
  }

  function setActiveDefinition(definitionId: string): void {
    activeDefinitionId.value = definitionId;
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
    if (!session) throw new Error("未选择会话");
    const definition = await ipc.createDefinition(session.id, input);
    sessions.value = sessions.value.map((candidate) =>
      candidate.id === session.id
        ? { ...candidate, definitions: [...candidate.definitions, definition] }
        : candidate,
    );
    activeDefinitionId.value = definition.id;
    return definition;
  }

  async function updateDefinition(
    definitionId: string,
    input: DefinitionInput,
  ): Promise<Definition> {
    const session = activeSession.value;
    if (!session) throw new Error("未选择会话");
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
    if (!session) throw new Error("未选择会话");
    await ipc.deleteDefinition(session.id, definitionId);
    const remaining = session.definitions.filter((definition) => definition.id !== definitionId);
    sessions.value = sessions.value.map((candidate) =>
      candidate.id === session.id ? { ...candidate, definitions: remaining } : candidate,
    );
    if (activeDefinitionId.value === definitionId) {
      activeDefinitionId.value = remaining[0]?.id ?? null;
    }
  }

  async function createSession(input: NewSessionInput): Promise<Session> {
    const session = await ipc.createSession(input);
    if (!sessions.value.some((candidate) => candidate.id === session.id)) {
      sessions.value = [...sessions.value, session];
    }
    activeSessionId.value = session.id;
    activeDefinitionId.value = session.definitions[0]?.id ?? null;
    return session;
  }

  async function writeRegister(address: number, value: string): Promise<void> {
    const session = activeSession.value;
    const definition = activeDefinition.value;
    if (!session || !definition) throw new Error("未选择定义");
    await ipc.writeRegister(session.id, definition.id, address, value);
  }

  return {
    sessions,
    activeSessionId,
    activeDefinitionId,
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
    setActiveSession,
    setActiveDefinition,
    togglePolling,
    readOnce,
    toggleDefinition,
    createDefinition,
    updateDefinition,
    deleteDefinition,
    createSession,
    writeRegister,
  };
});
