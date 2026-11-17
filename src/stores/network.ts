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

const STORAGE_KEY = "mt.network.config.v1";
const MAX_ENTRIES = 2000;

interface NetworkConfig {
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

const MODES: NetworkMode[] = ["tcp-server", "tcp-client", "udp-server", "udp-client", "tcp-relay"];

const DEFAULT_CONFIG: NetworkConfig = {
  mode: "tcp-server",
  localHost: "0.0.0.0",
  localPort: 8080,
  remoteHost: "127.0.0.1",
  remotePort: 502,
};

function loadConfig(): NetworkConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_CONFIG };
    const parsed = JSON.parse(raw) as Partial<NetworkConfig>;
    const mode = MODES.includes(parsed.mode ?? "tcp-server")
      ? (parsed.mode as NetworkMode)
      : DEFAULT_CONFIG.mode;
    return {
      mode,
      localHost: typeof parsed.localHost === "string" ? parsed.localHost : DEFAULT_CONFIG.localHost,
      localPort:
        typeof parsed.localPort === "number" && parsed.localPort >= 0 && parsed.localPort <= 65535
          ? parsed.localPort
          : DEFAULT_CONFIG.localPort,
      remoteHost:
        typeof parsed.remoteHost === "string" ? parsed.remoteHost : DEFAULT_CONFIG.remoteHost,
      remotePort:
        typeof parsed.remotePort === "number" &&
        parsed.remotePort >= 0 &&
        parsed.remotePort <= 65535
          ? parsed.remotePort
          : DEFAULT_CONFIG.remotePort,
    };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

export const useNetworkStore = defineStore("network", () => {
  const initial = loadConfig();
  const mode = ref<NetworkMode>(initial.mode);
  const localHost = ref(initial.localHost);
  const localPort = ref(initial.localPort);
  const remoteHost = ref(initial.remoteHost);
  const remotePort = ref(initial.remotePort);

  const active = ref(false);
  const busy = ref(false);
  const endpoint = ref<string | null>(null);
  const clients = ref(0);
  const error = ref("");
  const entries = ref<NetworkEntry[]>([]);
  let entryId = 0;

  const isServer = computed(() => mode.value === "tcp-server" || mode.value === "udp-server");
  const isRelay = computed(() => mode.value === "tcp-relay");
  const isClient = computed(() => mode.value === "tcp-client" || mode.value === "udp-client");
  const rxBytes = computed(() =>
    entries.value
      .filter((entry) => entry.direction === "rx")
      .reduce((total, entry) => total + entry.data.length, 0),
  );
  const txBytes = computed(() =>
    entries.value
      .filter((entry) => entry.direction === "tx")
      .reduce((total, entry) => total + entry.data.length, 0),
  );

  function config(): NetworkConfig {
    return {
      mode: mode.value,
      localHost: localHost.value.trim(),
      localPort: localPort.value,
      remoteHost: remoteHost.value.trim(),
      remotePort: remotePort.value,
    };
  }

  function input(): NetworkStartInput {
    const value = config();
    return {
      mode: value.mode,
      localHost: value.localHost,
      localPort: value.localPort,
      remoteHost: value.remoteHost || undefined,
      remotePort: value.remotePort || undefined,
    };
  }

  function validate(): string | null {
    if (isServer.value && (localPort.value < 1 || localPort.value > 65535)) {
      return "listenPortRange";
    }
    if ((isClient.value || isRelay.value) && !remoteHost.value.trim()) {
      return "remoteHostRequired";
    }
    if ((isClient.value || isRelay.value) && (remotePort.value < 1 || remotePort.value > 65535)) {
      return "remotePortRange";
    }
    if (isRelay.value && (localPort.value < 1 || localPort.value > 65535)) {
      return "listenPortRange";
    }
    return null;
  }

  function persist(): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(config()));
    } catch {
      // Persistence is best effort; the assistant still works without it.
    }
  }

  watch([mode, localHost, localPort, remoteHost, remotePort], persist, { flush: "post" });

  async function start(): Promise<void> {
    if (busy.value) return;
    const problem = validate();
    if (problem) throw new Error(problem);
    busy.value = true;
    error.value = "";
    try {
      const state = await ipc.startNetwork(input());
      applyState(state);
      if (!state.active) throw new Error(state.error ?? "networkStartFailed");
    } finally {
      busy.value = false;
    }
  }

  async function stop(): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    try {
      await ipc.stopNetwork();
      active.value = false;
      clients.value = 0;
    } finally {
      busy.value = false;
    }
  }

  async function send(payload: RawSendInput): Promise<number> {
    if (!active.value) throw new Error("networkNotRunning");
    return ipc.sendNetwork(payload);
  }

  function applyState(state: NetworkState): void {
    active.value = state.active;
    endpoint.value = state.endpoint;
    clients.value = state.clients;
    if (state.error) error.value = state.error;
    else if (state.active) error.value = "";
  }

  function applyData(data: NetworkData): void {
    if (!data.data.length) return;
    const entry: NetworkEntry = {
      id: ++entryId,
      direction: data.direction,
      data: [...data.data],
      connection: data.connection,
      timestamp: data.timestamp,
    };
    const next = [...entries.value, entry];
    entries.value = next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
  }

  function clear(): void {
    entries.value = [];
  }

  function resetError(): void {
    error.value = "";
  }

  return {
    mode,
    localHost,
    localPort,
    remoteHost,
    remotePort,
    active,
    busy,
    endpoint,
    clients,
    error,
    entries,
    isServer,
    isClient,
    isRelay,
    rxBytes,
    txBytes,
    start,
    stop,
    send,
    applyState,
    applyData,
    clear,
    resetError,
  };
});
