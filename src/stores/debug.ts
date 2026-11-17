import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "@/lib/ipc";
import type { DebugData, DebugState, RawSendInput } from "@/types/models";

const MAX_ENTRIES = 1200;

export interface DebugEntry {
  id: number;
  sessionId: string;
  direction: "tx" | "rx";
  data: number[];
  timestamp: number;
}

export const useDebugStore = defineStore("debug", () => {
  const sessionId = ref<string | null>(null);
  const active = ref(false);
  const error = ref("");
  const busy = ref(false);
  const entries = ref<DebugEntry[]>([]);
  let entryId = 0;
  let generation = 0;

  async function start(id: string): Promise<void> {
    if (busy.value) return;
    const current = ++generation;
    busy.value = true;
    entries.value = [];
    error.value = "";
    sessionId.value = id;
    try {
      await ipc.startDebug(id);
      if (current !== generation) return;
      active.value = true;
    } catch (cause: unknown) {
      if (current === generation) {
        sessionId.value = null;
        active.value = false;
      }
      throw cause;
    } finally {
      if (current === generation) busy.value = false;
    }
  }

  async function stop(): Promise<void> {
    generation += 1;
    busy.value = false;
    const id = sessionId.value;
    active.value = false;
    if (!id) return;
    try {
      await ipc.stopDebug(id);
    } finally {
      sessionId.value = null;
    }
  }

  async function send(input: RawSendInput): Promise<number> {
    if (!sessionId.value) throw new Error("调试会话未开启");
    return ipc.sendDebug(sessionId.value, input);
  }

  function applyData(data: DebugData): void {
    if (data.sessionId !== sessionId.value || !data.data.length) return;
    const entry: DebugEntry = {
      id: ++entryId,
      sessionId: data.sessionId,
      direction: data.direction,
      data: [...data.data],
      timestamp: data.timestamp,
    };
    const next = [...entries.value, entry];
    entries.value = next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
  }

  function applyState(state: DebugState): void {
    if (sessionId.value && state.sessionId !== sessionId.value) return;
    if (state.active) {
      sessionId.value = state.sessionId;
      active.value = true;
      error.value = "";
      return;
    }
    active.value = false;
    if (state.error) error.value = state.error;
  }

  function clear(): void {
    entries.value = [];
  }

  return {
    sessionId,
    active,
    error,
    busy,
    entries,
    start,
    stop,
    send,
    applyData,
    applyState,
    clear,
  };
});
