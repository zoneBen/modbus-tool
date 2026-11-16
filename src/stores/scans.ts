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

export const useScansStore = defineStore("scans", () => {
  const scanId = ref<string | null>(null);
  const kind = ref<ScanKind | null>(null);
  const progress = ref<ScanProgress | null>(null);
  const results = ref<ScanResult[]>([]);
  const running = ref(false);
  const error = ref<string | null>(null);
  const pendingKind = ref<ScanKind | null>(null);

  const percent = computed(() => {
    const current = progress.value;
    if (!current || current.total <= 0) return 0;
    return Math.min(100, Math.round((current.done / current.total) * 100));
  });

  const foundResults = computed(() =>
    results.value.filter((result) => ["ok", "exception", "open"].includes(result.status)),
  );

  function reset(): void {
    scanId.value = null;
    kind.value = null;
    progress.value = null;
    results.value = [];
    running.value = false;
    error.value = null;
    pendingKind.value = null;
  }

  function begin(id: string, scanKind: ScanKind): void {
    scanId.value = id;
    kind.value = scanKind;
    results.value = [];
    progress.value = null;
    error.value = null;
    running.value = true;
    pendingKind.value = null;
  }

  async function startScan(scanKind: ScanKind, starter: () => Promise<string>): Promise<string> {
    reset();
    kind.value = scanKind;
    pendingKind.value = scanKind;
    running.value = true;
    try {
      const id = await starter();
      if (scanId.value === null) begin(id, scanKind);
      pendingKind.value = null;
      return id;
    } catch (error: unknown) {
      reset();
      throw error;
    }
  }

  function applyProgress(update: ScanProgress): void {
    if (scanId.value === null && pendingKind.value === update.kind) {
      scanId.value = update.scanId;
      kind.value = update.kind;
      results.value = [];
      running.value = true;
    }
    if (update.scanId !== scanId.value) return;
    progress.value = update;
    if (update.error) error.value = update.error;
    if (update.finished) running.value = false;
  }

  function applyResult(result: ScanResult): void {
    if (scanId.value === null && pendingKind.value === result.kind) {
      scanId.value = result.scanId;
      kind.value = result.kind;
      running.value = true;
    }
    if (result.scanId !== scanId.value) return;
    results.value = [...results.value, result];
  }

  function applyDone(done: ScanDone): void {
    if (done.scanId !== scanId.value) return;
    running.value = false;
    pendingKind.value = null;
    if (done.error) error.value = done.error;
    if (progress.value) {
      progress.value = {
        ...progress.value,
        done: progress.value.done,
        total: progress.value.total,
        found: done.found,
        finished: true,
        ...(done.error ? { error: done.error } : {}),
      };
    }
  }

  async function startSlaveIds(input: SlaveIdScanInput): Promise<string> {
    return startScan("slaveId", () => ipc.scanSlaveIds(input));
  }

  async function startAddresses(input: AddressScanInput): Promise<string> {
    return startScan("address", () => ipc.scanAddresses(input));
  }

  async function startBaudRates(input: BaudScanInput): Promise<string> {
    return startScan("baud", () => ipc.scanBaudRates(input));
  }

  async function startNetwork(input: NetworkScanInput): Promise<string> {
    return startScan("network", () => ipc.scanNetwork(input));
  }

  async function cancel(): Promise<void> {
    const id = scanId.value;
    if (!id) return;
    await ipc.cancelScan(id);
    running.value = false;
  }

  return {
    scanId,
    kind,
    progress,
    results,
    running,
    error,
    percent,
    foundResults,
    startSlaveIds,
    startAddresses,
    startBaudRates,
    startNetwork,
    cancel,
    reset,
    applyProgress,
    applyResult,
    applyDone,
  };
});
