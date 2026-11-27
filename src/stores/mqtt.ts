import { defineStore } from "pinia";
import { reactive, ref, watch } from "vue";
import { ipc } from "@/lib/ipc";
import type { MqttMessage, MqttProtocol, MqttState, MqttStatus } from "@/types/tools";

const STORAGE_KEY = "mt.mqtt.v1";

export interface MqttBrokerConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  protocol: MqttProtocol;
  clientId: string;
  username: string;
  tls: boolean;
  caPath: string;
  keepAlive: number;
  cleanSession: boolean;
}

export interface MqttSubscription {
  topic: string;
  qos: number;
}

export interface MqttLogEntry {
  id: string;
  direction: "in" | "out";
  topic: string;
  payloadText: string;
  payloadHex: string;
  payloadBytes: number;
  qos: number;
  retain: boolean;
  timestampMs: number;
}

interface PersistedMqtt {
  version: 1;
  brokers: MqttBrokerConfig[];
  activeId: string | null;
  subscriptions: Record<string, MqttSubscription[]>;
}

export type MqttConfigSnapshot = PersistedMqtt;

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  return `broker-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function defaultBroker(index: number): MqttBrokerConfig {
  return {
    id: newId(),
    name: `Broker ${index}`,
    host: "127.0.0.1",
    port: 1883,
    protocol: "3.1.1",
    clientId: "",
    username: "",
    tls: false,
    caPath: "",
    keepAlive: 60,
    cleanSession: true,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function sanitizeBroker(value: unknown): MqttBrokerConfig | null {
  if (!isRecord(value)) return null;
  if (typeof value.id !== "string" || typeof value.host !== "string") return null;
  return {
    id: value.id,
    name: typeof value.name === "string" ? value.name : "Broker",
    host: value.host,
    port: typeof value.port === "number" ? Math.min(65_535, Math.max(1, value.port)) : 1883,
    protocol: value.protocol === "5" ? "5" : "3.1.1",
    clientId: typeof value.clientId === "string" ? value.clientId : "",
    username: typeof value.username === "string" ? value.username : "",
    tls: value.tls === true,
    caPath: typeof value.caPath === "string" ? value.caPath : "",
    keepAlive: typeof value.keepAlive === "number" ? value.keepAlive : 60,
    cleanSession: value.cleanSession !== false,
  };
}

function load(): PersistedMqtt {
  if (typeof localStorage === "undefined") {
    return { version: 1, brokers: [defaultBroker(1)], activeId: null, subscriptions: {} };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      const broker = defaultBroker(1);
      return { version: 1, brokers: [broker], activeId: broker.id, subscriptions: {} };
    }
    const parsed = JSON.parse(raw) as Partial<PersistedMqtt>;
    const brokers = Array.isArray(parsed.brokers)
      ? parsed.brokers.map(sanitizeBroker).filter((broker): broker is MqttBrokerConfig => !!broker)
      : [];
    const activeId =
      typeof parsed.activeId === "string" && brokers.some((broker) => broker.id === parsed.activeId)
        ? parsed.activeId
        : (brokers[0]?.id ?? null);
    const subscriptions: Record<string, MqttSubscription[]> = {};
    if (isRecord(parsed.subscriptions)) {
      for (const [key, value] of Object.entries(parsed.subscriptions)) {
        if (!Array.isArray(value)) continue;
        subscriptions[key] = value
          .filter(
            (entry): entry is MqttSubscription =>
              isRecord(entry) && typeof entry.topic === "string" && entry.topic.length > 0,
          )
          .map((entry) => ({
            topic: entry.topic,
            qos: typeof entry.qos === "number" ? Math.min(2, Math.max(0, entry.qos)) : 0,
          }));
      }
    }
    if (!brokers.length) {
      const broker = defaultBroker(1);
      brokers.push(broker);
      return { version: 1, brokers, activeId: broker.id, subscriptions };
    }
    return { version: 1, brokers, activeId, subscriptions };
  } catch {
    const broker = defaultBroker(1);
    return { version: 1, brokers: [broker], activeId: broker.id, subscriptions: {} };
  }
}

export const useMqttStore = defineStore("mqtt", () => {
  const initial = load();
  const brokers = ref<MqttBrokerConfig[]>(initial.brokers);
  const activeId = ref<string | null>(initial.activeId);
  const subscriptions = ref<Record<string, MqttSubscription[]>>(initial.subscriptions);
  const statuses = reactive<Record<string, MqttStatus>>({});
  const details = reactive<Record<string, string | null>>({});
  const messages = reactive<Record<string, MqttLogEntry[]>>({});
  const passwords = reactive<Record<string, string>>({});
  let initialized = false;
  let sequence = 0;

  function persist(): void {
    if (typeof localStorage === "undefined") return;
    const payload: PersistedMqtt = {
      version: 1,
      brokers: brokers.value,
      activeId: activeId.value,
      subscriptions: subscriptions.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  }

  watch([brokers, activeId, subscriptions], persist, { deep: true });

  function setStatus(brokerId: string, status: MqttStatus, detail: string | null = null): void {
    statuses[brokerId] = status;
    details[brokerId] = detail;
  }

  function logEntries(brokerId: string): MqttLogEntry[] {
    return messages[brokerId] ?? [];
  }

  function appendMessage(brokerId: string, entry: MqttLogEntry): void {
    const list = messages[brokerId] ?? [];
    list.unshift(entry);
    messages[brokerId] = list;
  }

  function applyState(state: MqttState): void {
    setStatus(state.brokerId, state.status, state.detail);
    if (state.status === "connected") {
      const entries = subscriptions.value[state.brokerId] ?? [];
      for (const subscription of entries) {
        void ipc
          .toolMqttSubscribe({
            brokerId: state.brokerId,
            topic: subscription.topic,
            qos: subscription.qos,
          })
          .catch(() => undefined);
      }
    }
  }

  function applyMessage(message: MqttMessage): void {
    sequence += 1;
    appendMessage(message.brokerId, {
      id: `in-${message.timestampMs}-${sequence}`,
      direction: "in",
      topic: message.topic,
      payloadText: message.payloadText,
      payloadHex: message.payloadHex,
      payloadBytes: message.payloadBytes,
      qos: message.qos,
      retain: message.retain,
      timestampMs: message.timestampMs,
    });
  }

  async function init(): Promise<void> {
    if (initialized) return;
    initialized = true;
    await ipc.on("mqtt:state", applyState);
    await ipc.on("mqtt:message", applyMessage);
  }

  function addBroker(): MqttBrokerConfig {
    const broker = defaultBroker(brokers.value.length + 1);
    brokers.value = [...brokers.value, broker];
    activeId.value = broker.id;
    return broker;
  }

  async function removeBroker(brokerId: string): Promise<void> {
    await stop(brokerId).catch(() => undefined);
    brokers.value = brokers.value.filter((broker) => broker.id !== brokerId);
    delete statuses[brokerId];
    delete details[brokerId];
    delete messages[brokerId];
    delete passwords[brokerId];
    const next = { ...subscriptions.value };
    delete next[brokerId];
    subscriptions.value = next;
    if (activeId.value === brokerId) {
      activeId.value = brokers.value[0]?.id ?? null;
    }
  }

  function updateBroker(brokerId: string, patch: Partial<MqttBrokerConfig>): void {
    brokers.value = brokers.value.map((broker) =>
      broker.id === brokerId ? { ...broker, ...patch, id: broker.id } : broker,
    );
  }

  function getBroker(brokerId: string): MqttBrokerConfig | undefined {
    return brokers.value.find((broker) => broker.id === brokerId);
  }

  function isRunning(brokerId: string): boolean {
    const status = statuses[brokerId];
    return status === "connecting" || status === "connected" || status === "reconnecting";
  }

  async function start(brokerId: string): Promise<void> {
    const broker = getBroker(brokerId);
    if (!broker) throw { code: "invalidBroker", detail: "Broker not found" };
    setStatus(brokerId, "connecting");
    try {
      await ipc.toolMqttStart({
        brokerId: broker.id,
        host: broker.host,
        port: broker.port,
        protocol: broker.protocol,
        clientId: broker.clientId,
        username: broker.username,
        password: passwords[brokerId] || undefined,
        tls: broker.tls,
        caPath: broker.caPath || undefined,
        keepAlive: broker.keepAlive,
        cleanSession: broker.cleanSession,
      });
    } catch (cause: unknown) {
      setStatus(brokerId, "error", null);
      throw cause;
    }
  }

  async function stop(brokerId: string): Promise<void> {
    await ipc.toolMqttStop(brokerId);
    setStatus(brokerId, "disconnected");
  }

  async function publish(
    brokerId: string,
    topic: string,
    payload: string,
    payloadMode: "text" | "hex",
    qos: number,
    retain: boolean,
  ): Promise<void> {
    await ipc.toolMqttPublish({ brokerId, topic, payload, payloadMode, qos, retain });
    const bytes = new TextEncoder().encode(payload);
    sequence += 1;
    appendMessage(brokerId, {
      id: `out-${Date.now()}-${sequence}`,
      direction: "out",
      topic,
      payloadText: payload,
      payloadHex:
        payloadMode === "hex"
          ? payload
          : Array.from(bytes)
              .map((byte) => byte.toString(16).padStart(2, "0").toUpperCase())
              .join(" "),
      payloadBytes:
        payloadMode === "hex" ? Math.floor(payload.replace(/\s+/g, "").length / 2) : bytes.length,
      qos,
      retain,
      timestampMs: Date.now(),
    });
  }

  async function subscribe(brokerId: string, topic: string, qos: number): Promise<void> {
    if (isRunning(brokerId)) {
      await ipc.toolMqttSubscribe({ brokerId, topic, qos });
    }
    const entries = subscriptions.value[brokerId] ?? [];
    if (!entries.some((entry) => entry.topic === topic)) {
      subscriptions.value = {
        ...subscriptions.value,
        [brokerId]: [...entries, { topic, qos }],
      };
    }
  }

  async function unsubscribe(brokerId: string, topic: string): Promise<void> {
    if (isRunning(brokerId)) {
      await ipc.toolMqttUnsubscribe(brokerId, topic);
    }
    subscriptions.value = {
      ...subscriptions.value,
      [brokerId]: (subscriptions.value[brokerId] ?? []).filter((entry) => entry.topic !== topic),
    };
  }

  function clearMessages(brokerId: string): void {
    messages[brokerId] = [];
  }

  function exportConfig(): MqttConfigSnapshot {
    return {
      version: 1,
      brokers: brokers.value.map((broker) => ({ ...broker })),
      activeId: activeId.value,
      subscriptions: Object.fromEntries(
        Object.entries(subscriptions.value).map(([key, value]) => [
          key,
          value.map((subscription) => ({ ...subscription })),
        ]),
      ),
    };
  }

  async function replaceConfig(value: unknown): Promise<void> {
    if (!isRecord(value)) return;
    const running = brokers.value.map((broker) => broker.id);
    await Promise.all(running.map((brokerId) => stop(brokerId).catch(() => undefined)));
    const nextBrokers = Array.isArray(value.brokers)
      ? value.brokers
          .map(sanitizeBroker)
          .filter((broker): broker is MqttBrokerConfig => Boolean(broker))
      : [];
    if (!nextBrokers.length) {
      const broker = defaultBroker(1);
      nextBrokers.push(broker);
    }
    const nextActive =
      typeof value.activeId === "string" &&
      nextBrokers.some((broker) => broker.id === value.activeId)
        ? value.activeId
        : nextBrokers[0].id;
    const nextSubscriptions: Record<string, MqttSubscription[]> = {};
    if (isRecord(value.subscriptions)) {
      for (const [key, entries] of Object.entries(value.subscriptions)) {
        if (!Array.isArray(entries)) continue;
        nextSubscriptions[key] = entries
          .filter(
            (entry): entry is MqttSubscription =>
              isRecord(entry) && typeof entry.topic === "string" && entry.topic.length > 0,
          )
          .map((entry) => ({
            topic: entry.topic,
            qos: typeof entry.qos === "number" ? Math.min(2, Math.max(0, entry.qos)) : 0,
          }));
      }
    }
    brokers.value = nextBrokers;
    activeId.value = nextActive;
    subscriptions.value = nextSubscriptions;
    for (const key of Object.keys(messages)) delete messages[key];
    for (const key of Object.keys(statuses)) delete statuses[key];
    for (const key of Object.keys(details)) delete details[key];
    for (const key of Object.keys(passwords)) delete passwords[key];
  }

  return {
    brokers,
    activeId,
    subscriptions,
    statuses,
    details,
    messages,
    passwords,
    init,
    addBroker,
    removeBroker,
    updateBroker,
    getBroker,
    isRunning,
    start,
    stop,
    publish,
    subscribe,
    unsubscribe,
    clearMessages,
    logEntries,
    exportConfig,
    replaceConfig,
  };
});
