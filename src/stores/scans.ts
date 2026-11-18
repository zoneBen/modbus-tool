import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { ipc } from "@/lib/ipc";
import type {
  AddressScanInput,
  BaudScanInput,
  NetworkScanInput,
  ScanDone,
  ScanKind,
  ScanProgress,
  ScanResult,
  SlaveIdScanInput,
} from "@/types/models";

export type ScanTaskStatus = "running" | "done" | "error";

export interface ScanTask {
  id: string;
  backendId: string | null;
  name: string;
  kind: ScanKind;
  description: string;
  createdAt: number;
  status: ScanTaskStatus;
  progress: ScanProgress | null;
  results: ScanResult[];
  error: string | null;
}

interface OrphanEvents {
  progress?: ScanProgress;
  results: ScanResult[];
  done?: ScanDone;
}

function localTaskId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `scan-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export const useScansStore = defineStore("scans", () => {
  const tasks = ref<ScanTask[]>([]);
  const activeTaskId = ref<string | null>(null);
  const orphans = new Map<string, OrphanEvents>();
  const removedBackendIds = new Set<string>();

  const activeTask = computed<ScanTask | null>(
    () => tasks.value.find((task) => task.id === activeTaskId.value) ?? null,
  );
  const runningCount = computed(
    () => tasks.value.filter((task) => task.status === "running").length,
  );
  const running = computed(() => runningCount.value > 0);

  function taskById(taskId: string): ScanTask | undefined {
    return tasks.value.find((task) => task.id === taskId);
  }

  function taskByBackendId(scanId: string): ScanTask | undefined {
    return tasks.value.find((task) => task.backendId === scanId);
  }

  function selectTask(taskId: string): void {
    if (taskById(taskId)) activeTaskId.value = taskId;
  }

  function percent(task: ScanTask): number {
    if (!task.progress || task.progress.total <= 0) return 0;
    return Math.min(100, Math.round((task.progress.done / task.progress.total) * 100));
  }

  function foundResults(task: ScanTask): ScanResult[] {
    return task.results.filter((result) => ["ok", "exception", "open"].includes(result.status));
  }

  function orphanFor(scanId: string): OrphanEvents {
    let entry = orphans.get(scanId);
    if (!entry) {
      entry = { results: [] };
      orphans.set(scanId, entry);
    }
    return entry;
  }

  function applyOrphanEvents(scanId: string): void {
    const entry = orphans.get(scanId);
    if (!entry) return;
    if (entry.progress) applyProgress(entry.progress);
    for (const result of entry.results) applyResult(result);
    if (entry.done) applyDone(entry.done);
    orphans.delete(scanId);
  }

  function bindBackend(taskId: string, backendId: string): void {
    const task = taskById(taskId);
    if (!task) return;
    task.backendId = backendId;
    applyOrphanEvents(backendId);
  }

  async function createTask(
    kind: ScanKind,
    name: string,
    description: string,
    starter: () => Promise<string>,
  ): Promise<string> {
    const id = localTaskId();
    const task: ScanTask = {
      id,
      backendId: null,
      name,
      kind,
      description,
      createdAt: Date.now(),
      status: "running",
      progress: null,
      results: [],
      error: null,
    };
    tasks.value = [...tasks.value, task];
    activeTaskId.value = id;
    try {
      const backendId = await starter();
      bindBackend(id, backendId);
      return id;
    } catch (error: unknown) {
      const current = taskById(id);
      if (current) {
        current.status = "error";
        current.error = error instanceof Error ? error.message : String(error);
      }
      throw error;
    }
  }

  function applyProgress(update: ScanProgress): void {
    if (removedBackendIds.has(update.scanId)) return;
    const task = taskByBackendId(update.scanId);
    if (!task) {
      orphanFor(update.scanId).progress = update;
      return;
    }
    task.progress = update;
    if (update.error) task.error = update.error;
    if (update.finished) task.status = update.error ? "error" : "done";
  }

  function applyResult(result: ScanResult): void {
    if (removedBackendIds.has(result.scanId)) return;
    const task = taskByBackendId(result.scanId);
    if (!task) {
      orphanFor(result.scanId).results.push(result);
      return;
    }
    task.results = [...task.results, result];
  }

  function applyDone(done: ScanDone): void {
    if (removedBackendIds.has(done.scanId)) return;
    const task = taskByBackendId(done.scanId);
    if (!task) {
      orphanFor(done.scanId).done = done;
      return;
    }
    task.status = done.error ? "error" : "done";
    if (done.error) task.error = done.error;
    if (task.progress) {
      task.progress = {
        ...task.progress,
        found: done.found,
        finished: true,
        ...(done.error ? { error: done.error } : {}),
      };
    }
  }

  async function startSlaveIds(
    input: SlaveIdScanInput,
    name: string,
    description: string,
  ): Promise<string> {
    return createTask("slaveId", name, description, () => ipc.scanSlaveIds(input));
  }

  async function startAddresses(
    input: AddressScanInput,
    name: string,
    description: string,
  ): Promise<string> {
    return createTask("address", name, description, () => ipc.scanAddresses(input));
  }

  async function startBaudRates(
    input: BaudScanInput,
    name: string,
    description: string,
  ): Promise<string> {
    return createTask("baud", name, description, () => ipc.scanBaudRates(input));
  }

  async function startNetwork(
    input: NetworkScanInput,
    name: string,
    description: string,
  ): Promise<string> {
    return createTask("network", name, description, () => ipc.scanNetwork(input));
  }

  async function cancel(taskId: string): Promise<void> {
    const task = taskById(taskId);
    if (!task || task.status !== "running") return;
    if (task.backendId) await ipc.cancelScan(task.backendId);
  }

  async function removeTask(taskId: string): Promise<void> {
    const task = taskById(taskId);
    if (!task) return;
    if (task.backendId) {
      removedBackendIds.add(task.backendId);
      orphans.delete(task.backendId);
    }
    if (task.status === "running" && task.backendId) {
      try {
        await ipc.cancelScan(task.backendId);
      } catch {
        // Removing the local task is still possible if cancellation races.
      }
    }
    tasks.value = tasks.value.filter((entry) => entry.id !== taskId);
    if (activeTaskId.value === taskId) {
      activeTaskId.value = tasks.value[0]?.id ?? null;
    }
  }

  function clearResults(taskId: string): void {
    const task = taskById(taskId);
    if (task) task.results = [];
  }

  function clearFinished(): void {
    tasks.value = tasks.value.filter((task) => task.status === "running");
    if (!tasks.value.some((task) => task.id === activeTaskId.value)) {
      activeTaskId.value = tasks.value[0]?.id ?? null;
    }
  }

  return {
    tasks,
    activeTaskId,
    activeTask,
    running,
    runningCount,
    selectTask,
    percent,
    foundResults,
    startSlaveIds,
    startAddresses,
    startBaudRates,
    startNetwork,
    cancel,
    removeTask,
    clearResults,
    clearFinished,
    applyProgress,
    applyResult,
    applyDone,
  };
});
