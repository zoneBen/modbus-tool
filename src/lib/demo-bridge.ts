import type { AppEventMap, AppEventName, IpcApi } from "@/types/commands";
import type {
  AddressScanInput,
  BaudScanInput,
  ByteOrder,
  CustomWriteInput,
  DataType,
  Definition,
  DefinitionInput,
  DefinitionSnapshot,
  Frame,
  NetworkScanInput,
  NetworkStartInput,
  NetworkState,
  NewSessionInput,
  RawSendInput,
  RegisterPoint,
  RegisterValue,
  ScanKind,
  ScanProgress,
  ScanResult,
  SerialPortInfo,
  Session,
  SessionAccent,
  SessionStatusEvent,
  SlaveIdScanInput,
  SlaveValueInput,
  UpdateSessionInput,
  WorkspaceSnapshot,
} from "@/types/models";
import {
  applyByteOrder,
  buildDecodedFrame,
  floatToWords,
  mbapFrame,
  readRequestPdu,
  readResponsePdu,
  rtuFrame,
  uint16Words,
  wordsToDataBytes,
  writeSingleRequestPdu,
} from "./demo-codec";
import type {
  ArpCapabilities,
  ArpEntry,
  ArpScanInput,
  ArpScanResult,
  ContinuousPingInput,
  DnsQueryInput,
  FileServerStartInput,
  FileServerStatus,
  HashInput,
  SearchInput,
  DnsResult,
  HttpRequestInput,
  HttpResponse,
  MqttBrokerInput,
  MqttPublishInput,
  MqttSubscribeInput,
  PingInput,
  PingHostResult,
  TelnetResult,
  TelnetTestInput,
  TracerouteInput,
} from "@/types/tools";
import { resolvePoints } from "./definition";
import { formatFunctionCode, formatNumber, snapshotKey } from "./format";
import { encodeRawPayload } from "./raw-send";

const TICK_MS = 720;

function demoWait(ms = 220): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
const MAX_FRAME_HISTORY = 400;

type AnyHandler = (payload: unknown) => void;

function clone<T>(value: T): T {
  try {
    return structuredClone(value);
  } catch {
    // Vue reactive proxies are not structured-cloneable; JSON keeps the demo
    // bridge usable when dialogs pass reactive mapping objects through IPC.
    return JSON.parse(JSON.stringify(value)) as T;
  }
}

function spanOf(dataType: DataType): number {
  switch (dataType) {
    case "i32":
    case "u32":
    case "f32":
      return 2;
    case "i64":
    case "u64":
    case "f64":
      return 4;
    default:
      return 1;
  }
}

function valueCount(definition: Definition): number {
  return Math.max(1, Math.floor(definition.quantity / spanOf(definition.dataType)));
}

function parseDemoRegister(text: string): number {
  const cleaned = text.trim().replace(/_/g, "");
  const value = cleaned.toLowerCase().startsWith("0x")
    ? Number.parseInt(cleaned.slice(2), 16)
    : Number(cleaned);
  if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new Error(`无法解析寄存器值「${text}」，请输入 0–65535 或 0x0000–0xFFFF`);
  }
  return value;
}

function encodeSlaveWords(input: SlaveValueInput): number[] {
  const order = input.byteOrder;
  const text = input.value.trim();
  if (!text) throw new Error("请输入写入值");
  const numeric = Number(text.replace(/,/g, ""));
  const bytesToWords = (bytes: number[]): number[] => {
    const padded = bytes.length % 2 === 0 ? bytes : [...bytes, 0];
    const words: number[] = [];
    for (let index = 0; index < padded.length; index += 2) {
      words.push(((padded[index] ?? 0) << 8) | (padded[index + 1] ?? 0));
    }
    return applyByteOrder(words, order);
  };
  const integer = (min: number, max: number): number => {
    if (!Number.isInteger(numeric) || numeric < min || numeric > max) {
      throw new Error(`值需为 ${min}–${max} 的整数`);
    }
    return numeric;
  };

  switch (input.dataType) {
    case "bool":
      return [parseDemoCoil(text) ? 1 : 0];
    case "u8":
      return bytesToWords([integer(0, 0xff)]);
    case "i8":
      return bytesToWords([integer(-128, 127) & 0xff]);
    case "u16":
    case "i16":
      return uint16Words(
        integer(input.dataType === "u16" ? 0 : -32768, input.dataType === "u16" ? 0xffff : 32767),
        order,
      );
    case "u32":
    case "i32": {
      const value = integer(
        input.dataType === "u32" ? 0 : -2147483648,
        input.dataType === "u32" ? 4294967295 : 2147483647,
      );
      const buffer = new ArrayBuffer(4);
      const view = new DataView(buffer);
      if (input.dataType === "u32") view.setUint32(0, value, false);
      else view.setInt32(0, value, false);
      return bytesToWords(Array.from(new Uint8Array(buffer)));
    }
    case "f32":
      if (!Number.isFinite(numeric)) throw new Error("请输入有效数字");
      return floatToWords(numeric, order);
    case "u64":
    case "i64": {
      const big = BigInt(integer(Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER));
      const buffer = new ArrayBuffer(8);
      const view = new DataView(buffer);
      if (input.dataType === "u64") view.setBigUint64(0, big, false);
      else view.setBigInt64(0, big, false);
      return bytesToWords(Array.from(new Uint8Array(buffer)));
    }
    case "f64": {
      if (!Number.isFinite(numeric)) throw new Error("请输入有效数字");
      const buffer = new ArrayBuffer(8);
      new DataView(buffer).setFloat64(0, numeric, false);
      return bytesToWords(Array.from(new Uint8Array(buffer)));
    }
    case "hex": {
      const hex = text.replace(/^0x/i, "").replace(/[^0-9a-f]/gi, "");
      if (!hex.length) throw new Error("请输入十六进制字节");
      const padded = hex.length % 2 === 0 ? hex : `0${hex}`;
      const bytes = Array.from({ length: padded.length / 2 }, (_, index) =>
        Number.parseInt(padded.slice(index * 2, index * 2 + 2), 16),
      );
      return bytesToWords(bytes);
    }
    case "bin": {
      const bits = text.replace(/[^01]/g, "");
      if (!bits.length) throw new Error("请输入二进制位");
      const bytes: number[] = [];
      for (let index = 0; index < bits.length; index += 8) {
        const chunk = bits.slice(index, index + 8).padEnd(8, "0");
        bytes.push(Number.parseInt(chunk, 2));
      }
      return bytesToWords(bytes);
    }
    case "ascii":
      return bytesToWords(Array.from(new TextEncoder().encode(text)));
    default:
      throw new Error("不支持的数据类型");
  }
}

function parseDemoCoil(text: string): boolean {
  switch (text.trim().toLowerCase()) {
    case "1":
    case "on":
    case "true":
      return true;
    case "0":
    case "off":
    case "false":
      return false;
    default:
      throw new Error(`无法解析线圈值「${text}」，请输入 0/1 或 ON/OFF`);
  }
}

function makeDefinition(
  id: string,
  name: string,
  startAddress: number,
  quantity: number,
  dataType: DataType,
  byteOrder: ByteOrder,
  intervalMs: number,
  aliases: Record<number, string>,
  slaveId = 1,
): Definition {
  const definition: Definition = {
    id,
    name,
    functionCode: 3,
    slaveId,
    startAddress,
    quantity,
    dataType,
    byteOrder,
    intervalMs,
    enabled: true,
    aliases,
    points: [],
  };
  return { ...definition, points: resolvePoints(definition) };
}

function createDemoSessions(): Session[] {
  const now = Date.now();
  return [
    {
      id: "plc-01",
      name: "PLC-01 · 产线 A",
      role: "master",
      transport: "tcp",
      framing: "mbap",
      endpoint: "192.168.1.10:502",
      status: "connected",
      polling: true,
      accent: "signal",
      stats: { tx: 12684, rx: 12681, errors: 3, latencyMs: 8.4 },
      startedAt: now - 1000 * 60 * 128,
      definitions: [
        makeDefinition("d-plc-main", "保持寄存器 · 主表", 0, 16, "u16", "abcd", 760, {
          0: "电机温度 ×10",
          1: "出口压力 ×100",
          2: "流量设定",
          3: "运行状态",
          4: "报警字",
          5: "产量计数",
          6: "母线电压 ×10",
          7: "负载率 %",
        }),
        makeDefinition("d-plc-float", "工艺参数 · 浮点", 100, 8, "f32", "cdab", 1100, {
          100: "介质温度 ℃",
          102: "系统压力 bar",
          104: "瞬时流量 L/min",
          106: "目标值",
        }),
      ],
    },
    {
      id: "rtu-485",
      name: "RTU-485 · 变频器",
      role: "master",
      transport: "serial",
      framing: "rtu",
      endpoint: "COM3 · 9600 8N1",
      status: "connected",
      polling: true,
      accent: "rx",
      stats: { tx: 4281, rx: 4280, errors: 1, latencyMs: 21.7 },
      startedAt: now - 1000 * 60 * 54,
      definitions: [
        makeDefinition(
          "d-rtu-speed",
          "输出频率 · 变频器",
          0,
          6,
          "u16",
          "badc",
          1480,
          {
            0: "输出频率 ×100",
            1: "输出电流 ×10",
            2: "直流母线",
            3: "运行命令",
            4: "故障码",
            5: "累计运行 h",
          },
          2,
        ),
        makeDefinition(
          "d-rtu-temp",
          "模块温度",
          20,
          4,
          "u16",
          "abcd",
          2100,
          { 20: "IGBT 温度", 21: "环境温度", 22: "风扇转速", 23: "状态" },
          2,
        ),
      ],
    },
    {
      id: "sim-01",
      name: "模拟器 · 从站",
      role: "slave",
      transport: "tcp",
      framing: "mbap",
      endpoint: "0.0.0.0:5020",
      status: "connected",
      polling: false,
      accent: "sim",
      stats: { tx: 0, rx: 1842, errors: 0, latencyMs: 0 },
      startedAt: now - 1000 * 60 * 18,
      definitions: [
        makeDefinition(
          "d-sim-map",
          "保持寄存器映射",
          0,
          16,
          "u16",
          "abcd",
          900,
          {
            0: "设定温度 ×10",
            1: "设定压力 ×100",
            2: "运行模式",
            3: "心跳计数",
            4: "报警字",
            5: "产品计数",
          },
          10,
        ),
      ],
    },
  ];
}

interface GeneratedValue {
  words: number[];
  formatted: string;
  numeric: number | null;
}

const FLOAT_BASES = [23.46, 4.82, 126.5, 24.0, 0.75, 1.4, 36.8, 8.15];
const INTEGER_BASES = [1180, 348, 760, 1, 0, 48213, 2284, 67, 3120, 445, 918, 12];
const SIM_BASES = [2350, 482, 2, 1841, 0, 17620, 3010, 12];

function generateValue(
  definition: Definition,
  index: number,
  tick: number,
  seed: number,
): GeneratedValue {
  const address = definition.startAddress + index * spanOf(definition.dataType);
  const phase = tick * 0.55 + index * 0.9 + seed;

  if (definition.dataType === "f32" || definition.dataType === "f64") {
    const base = FLOAT_BASES[(index + address) % FLOAT_BASES.length] ?? 10;
    const value = base + Math.sin(phase) * base * 0.035 + (Math.random() - 0.5) * 0.018;
    return {
      words: floatToWords(value, definition.byteOrder),
      formatted: value.toFixed(Math.abs(value) >= 100 ? 1 : 2),
      numeric: value,
    };
  }

  let base = INTEGER_BASES[index % INTEGER_BASES.length] ?? 100 + ((address * 37) % 900);
  if (definition.id === "d-sim-map") base = SIM_BASES[index % SIM_BASES.length] ?? 0;
  if (definition.id === "d-rtu-temp") base = [72, 31, 1840, 1][index % 4] ?? 0;

  const drift = Math.sin(phase) * Math.max(4, base * 0.018);
  const counterBump = index === 5 ? tick % 1000 : 0;
  const value = Math.max(0, Math.round(base + drift + counterBump));
  return {
    words: uint16Words(value, definition.byteOrder),
    formatted: formatNumber(value),
    numeric: value,
  };
}

export function createDemoBridge(): IpcApi {
  const sessions = createDemoSessions();
  const listeners = new Map<AppEventName, Set<AnyHandler>>();
  const latestSnapshots = new Map<string, DefinitionSnapshot>();
  const lastRun = new Map<string, number>();
  const transactions = new Map<string, number>();
  let frameSeq = 4200;
  let tick = 0;
  let timer: ReturnType<typeof setInterval> | null = null;
  const scanTimers = new Map<string, ReturnType<typeof setInterval>>();
  const scanMeta = new Map<
    string,
    { kind: ScanKind; total: number; done: number; found: number }
  >();
  let scanSequence = 0;
  let debugSessionId: string | null = null;
  let debugTimer: ReturnType<typeof setInterval> | null = null;
  let debugHeartbeat = 0;
  const demoNetworks = new Map<string, NetworkState>();
  const networkEchoTimers = new Map<string, ReturnType<typeof setTimeout>>();

  function emit<K extends AppEventName>(event: K, payload: AppEventMap[K]): void {
    listeners.get(event)?.forEach((handler) => handler(payload));
  }

  function sessionById(sessionId: string): Session {
    const session = sessions.find((candidate) => candidate.id === sessionId);
    if (!session) throw new Error(`会话不存在: ${sessionId}`);
    return session;
  }

  function emitDebug(sessionId: string, direction: "tx" | "rx", data: number[]): void {
    emit("debug:data", { sessionId, direction, data, timestamp: Date.now() });
  }

  function stopDebugLoop(notify: boolean): void {
    if (debugTimer) {
      clearInterval(debugTimer);
      debugTimer = null;
    }
    const sessionId = debugSessionId;
    debugSessionId = null;
    if (notify && sessionId) {
      emit("debug:state", { sessionId, active: false });
    }
  }

  function statusPayload(session: Session): SessionStatusEvent {
    return {
      sessionId: session.id,
      status: session.status,
      polling: session.polling,
      stats: { ...session.stats },
    };
  }

  function nextTransaction(sessionId: string): number {
    const value = ((transactions.get(sessionId) ?? 0) + 1) & 0xffff;
    transactions.set(sessionId, value);
    return value;
  }

  function mappingValue(
    definition: Definition,
    point: RegisterPoint,
    index: number,
    currentTick: number,
  ): RegisterValue {
    const generated = generateValue(
      {
        ...definition,
        startAddress: point.address,
        dataType: point.dataType,
        byteOrder: point.byteOrder,
      },
      0,
      currentTick,
      point.address * 0.13 + definition.slaveId,
    );
    const simulator = point.simulator;
    if (!simulator) {
      return {
        address: point.address,
        words: generated.words,
        formatted: generated.formatted,
        numeric: generated.numeric,
      };
    }
    const span = simulator.max - simulator.min;
    let numeric = simulator.min;
    if (simulator.enabled) {
      if (simulator.mode === "increment") {
        const steps = Math.max(1, Math.floor(span) + 1);
        numeric = simulator.min + ((Math.floor(currentTick / 3) + index) % steps);
      } else if (simulator.mode === "decrement") {
        const steps = Math.max(1, Math.floor(span) + 1);
        numeric = simulator.max - ((Math.floor(currentTick / 3) + index) % steps);
      } else if (simulator.mode === "random") {
        numeric = simulator.min + Math.random() * span;
      }
    }
    numeric = Math.min(simulator.max, Math.max(simulator.min, numeric));
    if (point.dataType === "bool") {
      const on = numeric >= 0.5;
      return {
        address: point.address,
        words: [on ? 1 : 0],
        formatted: on ? "ON" : "OFF",
        numeric: on ? 1 : 0,
      };
    }
    if (point.dataType === "f32") {
      return {
        address: point.address,
        words: floatToWords(numeric, point.byteOrder),
        formatted: numeric.toFixed(2),
        numeric,
      };
    }
    if (point.dataType === "f64" || point.dataType === "i64" || point.dataType === "u64") {
      return {
        address: point.address,
        words: generated.words,
        formatted: formatNumber(Math.round(numeric)),
        numeric,
      };
    }
    const rounded = Math.round(numeric);
    return {
      address: point.address,
      words: uint16Words(((rounded % 0x10000) + 0x10000) % 0x10000, point.byteOrder),
      formatted: formatNumber(rounded),
      numeric,
    };
  }

  function snapshotFor(
    session: Session,
    definition: Definition,
    currentTick = tick,
  ): DefinitionSnapshot {
    if (session.role === "slave" && definition.points.length) {
      return {
        sessionId: session.id,
        definitionId: definition.id,
        timestamp: Date.now(),
        values: definition.points.map((point, index) =>
          mappingValue(definition, point, index, currentTick),
        ),
        latencyMs: 0,
      };
    }
    const count = Math.min(valueCount(definition), 32);
    const seed = definition.startAddress * 0.13 + definition.slaveId;
    const values: RegisterValue[] = Array.from({ length: count }, (_, index) => {
      const generated = generateValue(definition, index, currentTick, seed);
      return {
        address: definition.startAddress + index * spanOf(definition.dataType),
        words: generated.words,
        formatted: generated.formatted,
        numeric: generated.numeric,
      };
    });
    const jitter =
      definition.id === "d-sim-map"
        ? 0
        : Math.max(1, session.stats.latencyMs + (Math.random() - 0.5) * 3.4);
    return {
      sessionId: session.id,
      definitionId: definition.id,
      timestamp: Date.now(),
      values,
      latencyMs: Number(jitter.toFixed(1)),
    };
  }

  function storeSnapshot(snapshot: DefinitionSnapshot): void {
    latestSnapshots.set(snapshotKey(snapshot.sessionId, snapshot.definitionId), snapshot);
  }

  function emitSnapshot(
    session: Session,
    definition: Definition,
    currentTick = tick,
  ): DefinitionSnapshot {
    const snapshot = snapshotFor(session, definition, currentTick);
    storeSnapshot(snapshot);
    emit("session:data", snapshot);
    return snapshot;
  }

  function makeFrame(
    session: Session,
    direction: "tx" | "rx",
    raw: number[],
    summary: string,
    latencyMs?: number,
  ): Frame {
    frameSeq += 1;
    return {
      seq: frameSeq,
      sessionId: session.id,
      timestamp: Date.now() + (direction === "rx" ? Math.round(latencyMs ?? 0) : 0),
      direction,
      raw,
      decoded: buildDecodedFrame(session.framing, raw),
      summary,
      latencyMs,
      status: "ok",
    };
  }

  function buildReadFrames(
    session: Session,
    definition: Definition,
    snapshot: DefinitionSnapshot,
  ): Frame[] {
    const transactionId = nextTransaction(session.id);
    const requestPdu = readRequestPdu(
      definition.startAddress,
      definition.quantity,
      definition.functionCode,
    );
    const dataBytes = snapshot.values.flatMap((value) => wordsToDataBytes(value.words));
    const responsePdu = readResponsePdu(definition.functionCode, dataBytes);
    const requestRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, definition.slaveId, requestPdu)
        : rtuFrame(definition.slaveId, requestPdu);
    const responseRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, definition.slaveId, responsePdu)
        : rtuFrame(definition.slaveId, responsePdu);
    const fc = formatFunctionCode(definition.functionCode);
    return [
      makeFrame(
        session,
        "tx",
        requestRaw,
        `${fc} · 从站 ${definition.slaveId} · ${definition.startAddress} × ${definition.quantity}`,
      ),
      makeFrame(
        session,
        "rx",
        responseRaw,
        `${fc} · ${snapshot.values.length} 值 · ${snapshot.values.length * spanOf(definition.dataType)} 寄存器`,
        snapshot.latencyMs,
      ),
    ];
  }

  function buildWriteFrames(
    session: Session,
    definition: Definition,
    address: number,
    words: number[],
  ): Frame[] {
    const transactionId = nextTransaction(session.id);
    const span = spanOf(definition.dataType);
    const requestPdu =
      span === 1
        ? writeSingleRequestPdu(address, words[0] ?? 0)
        : [
            0x10,
            (address >> 8) & 0xff,
            address & 0xff,
            0,
            span,
            span * 2,
            ...wordsToDataBytes(words),
          ];
    const responsePdu =
      span === 1 ? requestPdu : [0x10, (address >> 8) & 0xff, address & 0xff, 0, span];
    const requestRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, definition.slaveId, requestPdu)
        : rtuFrame(definition.slaveId, requestPdu);
    const responseRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, definition.slaveId, responsePdu)
        : rtuFrame(definition.slaveId, responsePdu);
    const label = span === 1 ? "FC06" : "FC16";
    return [
      makeFrame(session, "tx", requestRaw, `${label} · 写入 ${address} · ${words.length} 寄存器`),
      makeFrame(session, "rx", responseRaw, `${label} · 写入确认`, 2 + Math.random() * 4),
    ];
  }

  function runSessionTick(session: Session, currentTick: number): void {
    const now = Date.now();
    const frames: Frame[] = [];
    let ran = 0;
    for (const definition of session.definitions) {
      if (!definition.enabled) continue;
      const last = lastRun.get(definition.id) ?? 0;
      if (now - last < definition.intervalMs) continue;
      lastRun.set(definition.id, now);
      const snapshot = emitSnapshot(session, definition, currentTick);
      frames.push(...buildReadFrames(session, definition, snapshot));
      ran += 1;
    }
    if (!frames.length) return;
    session.stats.tx += ran;
    session.stats.rx += ran;
    session.stats.latencyMs = Number(
      Math.max(1, session.stats.latencyMs + (Math.random() - 0.5) * 0.8).toFixed(1),
    );
    emit("session:frames", { sessionId: session.id, frames: frames.slice(-MAX_FRAME_HISTORY) });
  }

  function onTick(): void {
    tick += 1;
    for (const session of sessions) {
      if (session.role === "slave") {
        if (tick % 2 === 0) {
          const definition = session.definitions[0];
          if (definition) emitSnapshot(session, definition, tick + 5);
        }
        continue;
      }
      if (session.status === "connected" && session.polling) {
        runSessionTick(session, tick);
      }
    }
    if (tick % 4 === 0) {
      for (const session of sessions) emit("session:status", statusPayload(session));
    }
  }

  function start(): void {
    if (timer) return;
    timer = setInterval(onTick, TICK_MS);
    for (const session of sessions) {
      if (session.role === "master" && session.status === "connected") {
        setTimeout(() => runSessionTick(session, tick), 120);
      }
    }
  }

  async function boot(): Promise<WorkspaceSnapshot> {
    start();
    return { sessions: clone(sessions), version: 1, demo: true };
  }

  async function saveTextFile(
    suggestedName: string,
    contents: string,
    format: "csv" | "json" = "csv",
  ): Promise<boolean> {
    const blob = new Blob([contents], {
      type: format === "json" ? "application/json;charset=utf-8" : "text/csv;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = suggestedName;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return true;
  }

  async function openTextFile(): Promise<string | null> {
    return new Promise((resolve) => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json,application/json";
      input.style.display = "none";
      input.addEventListener(
        "change",
        () => {
          const file = input.files?.[0];
          if (!file) {
            input.remove();
            resolve(null);
            return;
          }
          const reader = new FileReader();
          reader.onload = () => {
            input.remove();
            resolve(typeof reader.result === "string" ? reader.result : null);
          };
          reader.onerror = () => {
            input.remove();
            resolve(null);
          };
          reader.readAsText(file);
        },
        { once: true },
      );
      document.body.appendChild(input);
      input.click();
    });
  }

  async function listSerialPorts(): Promise<SerialPortInfo[]> {
    return [
      { path: "COM3", label: "USB-SERIAL CH340 (COM3)", manufacturer: "wch.cn" },
      { path: "COM4", label: "FTDI FT232R USB UART (COM4)", manufacturer: "FTDI" },
      { path: "COM7", label: "Prolific USB-to-Serial (COM7)", manufacturer: "Prolific" },
    ];
  }

  async function createSession(input: NewSessionInput): Promise<Session> {
    const role = input.role;
    const accent: SessionAccent =
      role === "slave" ? "sim" : input.transport === "serial" ? "rx" : "signal";
    const id = `demo-${Date.now().toString(36)}`;
    const definition =
      role === "slave"
        ? makeDefinition(
            `${id}-map`,
            "保持寄存器映射",
            0,
            0,
            "u16",
            "abcd",
            900,
            {},
            input.slaveId ?? 10,
          )
        : null;
    const connected = role !== "slave" && input.autoConnect !== false;
    const session: Session = {
      id,
      name: input.name || (role === "slave" ? "模拟器 · 新从站" : "新会话"),
      role,
      transport: input.transport,
      framing: input.framing,
      endpoint: input.endpoint,
      status: connected ? "connected" : "disconnected",
      polling: role === "master" && connected,
      accent,
      stats: { tx: 0, rx: 0, errors: 0, latencyMs: input.transport === "serial" ? 18 : 6 },
      startedAt: Date.now(),
      definitions: definition ? [definition] : [],
      host: input.host,
      port: input.port,
      serialPort: input.serialPort,
      baudRate: input.baudRate,
      dataBits: input.dataBits,
      parity: input.parity,
      stopBits: input.stopBits,
      timeoutMs: input.timeoutMs,
      retries: input.retries,
      interFrameDelayMs: input.interFrameDelayMs,
      autoReconnect: input.autoReconnect,
    };
    sessions.push(session);
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
    emit("session:status", statusPayload(session));
    if (definition && session.status === "connected") {
      const snapshot = snapshotFor(session, definition);
      storeSnapshot(snapshot);
      emit("session:data", snapshot);
    }
    setTimeout(() => {
      if (session.role === "master" && session.polling) runSessionTick(session, tick);
    }, 180);
    return clone(session);
  }

  async function deleteSession(sessionId: string): Promise<void> {
    const index = sessions.findIndex((session) => session.id === sessionId);
    if (index < 0) throw new Error("会话不存在");
    if (debugSessionId === sessionId) stopDebugLoop(true);
    sessions.splice(index, 1);
    for (const key of [...latestSnapshots.keys()]) {
      if (key.startsWith(`${sessionId}:`)) latestSnapshots.delete(key);
    }
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
  }

  async function connectSession(sessionId: string): Promise<void> {
    if (debugSessionId === sessionId) throw new Error("调试模式进行中，请先关闭调试");
    const session = sessionById(sessionId);
    const definition = session.definitions[0];
    if (session.role === "slave" && (!definition || !definition.points.length)) {
      throw new Error("请先配置至少一个模拟映射，再启动从站监听");
    }
    session.status = "connected";
    session.polling = session.role === "master";
    emit("session:status", statusPayload(session));
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
    if (session.role === "master") {
      setTimeout(() => runSessionTick(session, tick), 120);
    } else if (definition) {
      emitSnapshot(session, definition);
    }
  }

  async function disconnectSession(sessionId: string): Promise<void> {
    const session = sessionById(sessionId);
    if (debugSessionId === sessionId) stopDebugLoop(true);
    session.status = "disconnected";
    session.polling = false;
    emit("session:status", statusPayload(session));
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
  }

  async function updateSession(sessionId: string, input: UpdateSessionInput): Promise<void> {
    if (debugSessionId === sessionId) throw new Error("调试模式进行中，请先关闭调试");
    const session = sessionById(sessionId);
    session.name = input.name.trim() || session.name;
    session.transport = input.transport;
    session.framing = input.framing;
    session.endpoint = input.endpoint;
    session.host = input.host;
    session.port = input.port;
    session.serialPort = input.serialPort;
    session.baudRate = input.baudRate;
    session.dataBits = input.dataBits;
    session.parity = input.parity;
    session.stopBits = input.stopBits;
    session.timeoutMs = input.timeoutMs;
    session.retries = input.retries;
    session.interFrameDelayMs = input.interFrameDelayMs;
    session.autoReconnect = input.autoReconnect;
    emit("session:status", statusPayload(session));
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
  }

  async function writeSlaveValue(sessionId: string, input: SlaveValueInput): Promise<number> {
    const session = sessionById(sessionId);
    if (session.role !== "slave") throw new Error("仅从站模拟会话支持修改模拟值");
    const definition = session.definitions[0];
    if (!definition) throw new Error("从站映射定义缺失");
    const words = encodeSlaveWords(input);
    if (!words.length) throw new Error("写入值为空");
    if (input.address + words.length > 0x10000) {
      throw new Error("写入地址范围超出 65535");
    }
    const snapshot = latestSnapshots.get(snapshotKey(sessionId, definition.id));
    if (!snapshot) return words.length;
    const next = {
      ...snapshot,
      timestamp: Date.now(),
      values: snapshot.values.map((entry) => {
        const offset = entry.address - input.address;
        if (offset < 0 || offset >= words.length) return entry;
        const word = words[offset] ?? 0;
        return {
          ...entry,
          words: [word],
          formatted: formatNumber(word),
          numeric: word,
          stale: false,
          error: undefined,
        };
      }),
    };
    storeSnapshot(next);
    emit("session:data", next);
    return words.length;
  }

  async function setPolling(sessionId: string, enabled: boolean): Promise<void> {
    if (debugSessionId === sessionId && enabled) {
      throw new Error("调试模式进行中，请先关闭调试");
    }
    const session = sessionById(sessionId);
    session.polling = enabled;
    emit("session:status", statusPayload(session));
    if (enabled && session.role === "master") {
      setTimeout(() => runSessionTick(session, tick), 80);
    }
  }

  async function readOnce(sessionId: string, definitionId: string): Promise<void> {
    const session = sessionById(sessionId);
    const definition = session.definitions.find((candidate) => candidate.id === definitionId);
    if (!definition) throw new Error("定义不存在");
    const snapshot = emitSnapshot(session, definition);
    const frames = buildReadFrames(session, definition, snapshot);
    session.stats.tx += 1;
    session.stats.rx += 1;
    emit("session:frames", { sessionId, frames });
    emit("session:status", statusPayload(session));
  }

  async function setDefinitionEnabled(
    sessionId: string,
    definitionId: string,
    enabled: boolean,
  ): Promise<void> {
    const session = sessionById(sessionId);
    const definition = session.definitions.find((candidate) => candidate.id === definitionId);
    if (!definition) throw new Error("定义不存在");
    definition.enabled = enabled;
    if (enabled && session.role === "master" && session.polling) {
      setTimeout(() => runSessionTick(session, tick), 60);
    }
  }

  function emitScanProgress(
    scanId: string,
    kind: ScanKind,
    done: number,
    total: number,
    found: number,
    phase: string,
    finished: boolean,
    error?: string,
  ): void {
    const progress: ScanProgress = {
      scanId,
      kind,
      done,
      total,
      found,
      phase,
      finished,
      ...(error ? { error } : {}),
    };
    emit("scan:progress", progress);
  }

  function finishDemoScan(scanId: string, error?: string): void {
    const interval = scanTimers.get(scanId);
    if (interval) clearInterval(interval);
    scanTimers.delete(scanId);
    const meta = scanMeta.get(scanId);
    scanMeta.delete(scanId);
    if (!meta) return;
    emitScanProgress(
      scanId,
      meta.kind,
      meta.done,
      meta.total,
      meta.found,
      error ? "已取消" : "完成",
      true,
      error,
    );
    emit("scan:done", {
      scanId,
      kind: meta.kind,
      found: meta.found,
      ...(error ? { error } : {}),
    });
  }

  function startDemoScan(
    kind: ScanKind,
    total: number,
    resultAt: (step: number) => Partial<ScanResult> | null,
    stepMs = 90,
  ): Promise<string> {
    scanSequence += 1;
    const scanId = `demo-scan-${scanSequence}`;
    let step = 0;
    let found = 0;
    emitScanProgress(scanId, kind, 0, total, 0, "准备中", false);
    scanMeta.set(scanId, { kind, total, done: 0, found: 0 });
    const interval = setInterval(() => {
      step += 1;
      const partial = resultAt(step);
      if (partial) {
        const status = partial.status ?? "ok";
        if (["ok", "exception", "open"].includes(status)) found += 1;
        const result: ScanResult = {
          scanId,
          kind,
          label: partial.label ?? `#${step}`,
          status,
          detail: partial.detail ?? "演示响应",
          latencyMs: partial.latencyMs ?? Number((3 + Math.random() * 12).toFixed(1)),
          slaveId: partial.slaveId,
          address: partial.address,
          baudRate: partial.baudRate,
          endpoint: partial.endpoint,
          port: partial.port,
          service: partial.service,
          banner: partial.banner,
        };
        emit("scan:result", result);
      }
      scanMeta.set(scanId, { kind, total, done: step, found });
      emitScanProgress(scanId, kind, step, total, found, "扫描中", false);
      if (step >= total) finishDemoScan(scanId);
    }, stepMs);
    scanTimers.set(scanId, interval);
    return Promise.resolve(scanId);
  }

  async function scanSlaveIds(input: SlaveIdScanInput): Promise<string> {
    const total = Math.max(1, input.idEnd - input.idStart + 1);
    const foundIds = new Set<number>([input.idStart, input.idStart + 9, input.idStart + 41]);
    return startDemoScan("slaveId", Math.min(total, 64), (step) => {
      const id = input.idStart + step - 1;
      if (id > input.idEnd) return null;
      if (foundIds.has(id)) {
        return {
          label: `ID ${id}`,
          status: "ok",
          detail: "演示从站响应",
          slaveId: id,
          address: input.address,
        };
      }
      return step % 3 === 0
        ? {
            label: `ID ${id}`,
            status: "timeout",
            detail: "演示超时",
            slaveId: id,
            address: input.address,
          }
        : null;
    });
  }

  async function scanAddresses(input: AddressScanInput): Promise<string> {
    const span = input.endAddress - input.startAddress + 1;
    const blockSize = Math.max(1, input.blockSize);
    const total = Math.max(1, Math.ceil(span / blockSize));
    return startDemoScan(
      "address",
      Math.min(total, 32),
      (step) => {
        if (step % 4 !== 1) return null;
        const start = input.startAddress + (step - 1) * blockSize;
        const end = Math.min(start + blockSize - 1, input.endAddress);
        return {
          label: `${start.toString().padStart(5, "0")}–${end.toString().padStart(5, "0")}`,
          status: "ok",
          detail: `可读 ${end - start + 1} · 非零 ${Math.max(1, (step * 7) % 5)}`,
          address: start,
        };
      },
      110,
    );
  }

  async function scanBaudRates(input: BaudScanInput): Promise<string> {
    const rates = [...new Set(input.baudRates)].slice(0, 20);
    return startDemoScan(
      "baud",
      Math.max(rates.length, 1),
      (step) => {
        const baudRate = rates[step - 1];
        if (baudRate === undefined) return null;
        const ok = baudRate === 9600 || baudRate === 19200;
        return {
          label: `${baudRate} bps`,
          status: ok ? "ok" : "timeout",
          detail: ok ? "演示从站响应" : "无响应",
          slaveId: input.slaveId,
          baudRate,
        };
      },
      240,
    );
  }

  async function scanNetwork(input: NetworkScanInput): Promise<string> {
    const total = 128;
    const ports = input.ports?.length ? [...new Set(input.ports)] : [input.port ?? 502];
    const foundHosts = ["192.168.1.10", "192.168.1.20", "192.168.1.100"];
    const foundSteps = [12, 28, 95];
    return startDemoScan(
      "network",
      total,
      (step) => {
        const hostIndex = foundSteps.indexOf(step);
        if (hostIndex < 0) return null;
        const host = foundHosts[hostIndex] ?? "192.168.1.10";
        const port = ports[hostIndex % ports.length] ?? 502;
        const modbus = input.probeModbus && port === 502;
        return {
          label: `${host}:${port}`,
          status: modbus ? "ok" : "open",
          detail: modbus ? "Modbus 从站响应" : "TCP 端口开放",
          endpoint: `${host}:${port}`,
          port,
          service:
            port === 502 ? "Modbus" : port === 80 ? "HTTP" : port === 443 ? "HTTPS" : undefined,
          banner: input.grabBanner && port !== 502 ? "SSH-2.0-demo" : undefined,
        };
      },
      45,
    );
  }

  async function toolDnsQuery(input: DnsQueryInput): Promise<DnsResult> {
    await demoWait(260);
    if (!input.name.trim()) {
      throw { code: "invalidName", detail: "DNS name is required" };
    }
    const status = input.name.toLowerCase().includes("nxdomain") ? "nxdomain" : "ok";
    const answers =
      status === "nxdomain"
        ? []
        : [
            {
              name: input.name.endsWith(".") ? input.name : `${input.name}.`,
              recordType: input.recordType,
              ttl: 300,
              value:
                input.recordType === "MX"
                  ? "10 mail.demo.local."
                  : input.recordType === "TXT"
                    ? '"v=modbus-tool-demo"'
                    : input.recordType === "AAAA"
                      ? "2001:db8::10"
                      : input.recordType === "PTR"
                        ? "demo.local."
                        : "192.0.2.10",
            },
          ];
    return {
      question: input.name,
      recordType: input.recordType,
      server: input.server?.trim() || "system",
      status,
      elapsedMs: 18.6,
      answers,
    };
  }

  async function toolTelnetTest(input: TelnetTestInput): Promise<TelnetResult> {
    await demoWait(260);
    if (!input.host.trim()) {
      throw { code: "invalidHost", detail: "Host is required" };
    }
    const text = input.port === 23 ? "220 demo telnet ready\r\n" : "SSH-2.0-modbus-tool-demo\r\n";
    return {
      host: input.host,
      port: input.port,
      connected: true,
      latencyMs: 7.8,
      bytesRead: new TextEncoder().encode(text).length,
      text,
      hex: Array.from(new TextEncoder().encode(text))
        .map((byte) => byte.toString(16).padStart(2, "0").toUpperCase())
        .join(" "),
    };
  }

  async function toolArpCapabilities(): Promise<ArpCapabilities> {
    await demoWait(160);
    return {
      native: true,
      npcapAvailable: true,
      elevated: true,
      adapters: [
        {
          name: "{demo-ethernet}",
          description: "Demo Ethernet",
          ip: "192.168.1.10",
          cidr: "192.168.1.0/24",
          mac: "02:00:00:00:00:01",
          npcapName: "\\Device\\NPF_{demo-ethernet}",
        },
      ],
      npcapInterfaces: [
        {
          name: "\\Device\\NPF_{demo-ethernet}",
          description: "Demo Ethernet (Npcap)",
        },
      ],
    };
  }

  async function toolArpScan(input: ArpScanInput): Promise<ArpScanResult> {
    await demoWait(420);
    const base = input.cidr.split("/")[0]?.split(".").slice(0, 3).join(".") ?? "192.168.1";
    const entries: ArpEntry[] = [
      {
        ip: `${base}.1`,
        mac: "02:00:00:00:00:01",
        interface: "Demo Ethernet",
        source: "native" as const,
        replies: 1,
        conflict: false,
      },
      {
        ip: `${base}.20`,
        mac: "02:00:00:00:00:20",
        interface: "Demo Ethernet",
        source: "native" as const,
        replies: 1,
        conflict: false,
      },
    ];
    if (input.enhanced) {
      entries.push({
        ip: `${base}.20`,
        mac: "02:00:00:00:00:21",
        interface: "Demo Ethernet",
        source: "npcap" as const,
        replies: 2,
        conflict: true,
      });
    }
    return {
      cidr: input.cidr,
      elapsedMs: 318.4,
      enhanced: input.enhanced,
      entries,
      conflicts: input.enhanced ? 1 : 0,
    };
  }

  async function toolHashStart(input: HashInput): Promise<string> {
    await demoWait(120);
    if (!input.path.trim()) {
      throw { code: "invalidPath", detail: "Select a file to hash" };
    }
    toolTaskSequence += 1;
    const taskId = `demo-tool-${toolTaskSequence}`;
    toolKinds.set(taskId, "hash");
    let done = 0;
    const total = 8 * 1024 * 1024;
    toolTimers.set(
      taskId,
      setInterval(() => {
        done = Math.min(total, done + 2 * 1024 * 1024);
        emit("tool:event", { event: "hashProgress", taskId, done, total });
        if (done >= total) {
          clearInterval(toolTimers.get(taskId));
          toolTimers.delete(taskId);
          toolKinds.delete(taskId);
          const algorithms = input.algorithms.length ? input.algorithms : ["sha256"];
          emit("tool:event", {
            event: "hashDone",
            taskId,
            result: {
              path: input.path,
              size: total,
              elapsedMs: 186.3,
              values: algorithms.map((algorithm, index) => ({
                algorithm: algorithm.toUpperCase(),
                digest: `${algorithm}-demo-${String(index + 1).repeat(48)}`.slice(0, 64),
              })),
            },
            error: undefined,
          });
        }
      }, 260),
    );
    return taskId;
  }

  async function toolSearchStart(input: SearchInput): Promise<string> {
    await demoWait(120);
    if (!input.folder.trim()) {
      throw { code: "invalidFolder", detail: "Select an existing folder" };
    }
    if (!input.query.trim()) {
      throw { code: "noQuery", detail: "Enter search text" };
    }
    toolTaskSequence += 1;
    const taskId = `demo-tool-${toolTaskSequence}`;
    toolKinds.set(taskId, "search");
    let step = 0;
    const totalFiles = 4;
    const hits = [
      {
        path: `${input.folder}\\src\\main.ts`,
        line: 12,
        column: 8,
        preview: `…const ${input.query} = "demo"…`,
        matched: input.query,
      },
      {
        path: `${input.folder}\\README.md`,
        line: 4,
        column: 1,
        preview: `…${input.query} usage…`,
        matched: input.query,
      },
      {
        path: `${input.folder}\\src\\utils.ts`,
        line: 29,
        column: 14,
        preview: `…return ${input.query}…`,
        matched: input.query,
      },
    ];
    toolTimers.set(
      taskId,
      setInterval(() => {
        step += 1;
        if (step === 2) {
          emit("tool:event", { event: "searchHits", taskId, hits });
        }
        emit("tool:event", {
          event: "searchProgress",
          taskId,
          files: step,
          matches: step >= 2 ? hits.length : 0,
        });
        if (step >= totalFiles) {
          clearInterval(toolTimers.get(taskId));
          toolTimers.delete(taskId);
          toolKinds.delete(taskId);
          emit("tool:event", {
            event: "searchDone",
            taskId,
            files: totalFiles,
            matches: hits.length,
            skipped: 1,
            truncated: false,
          });
        }
      }, 260),
    );
    return taskId;
  }

  async function pickFile(): Promise<string | null> {
    await demoWait(80);
    return "C:\\demo\\modbus-tool-demo.json";
  }

  async function pickDirectory(): Promise<string | null> {
    await demoWait(80);
    return "C:\\demo\\workspace";
  }

  async function readTextFile(_path: string): Promise<string> {
    await demoWait(80);
    return "name: modbus-tool\nversion: 0.6.0\nports:\n  - 502\n  - 1502\n";
  }

  let toolTaskSequence = 0;
  const toolTimers = new Map<string, ReturnType<typeof setInterval>>();
  const toolKinds = new Map<string, "ping" | "traceroute" | "latency" | "hash" | "search">();
  const latencyStats = new Map<string, { sent: number; received: number; rtts: number[] }>();

  async function toolPingStart(input: PingInput): Promise<string> {
    await demoWait(120);
    const targets = input.targets
      .split(/[\s,;]+/)
      .map((target) => target.trim())
      .filter(Boolean)
      .slice(0, 6);
    if (!targets.length) {
      throw { code: "noTargets", detail: "Enter at least one host or CIDR" };
    }
    toolTaskSequence += 1;
    const taskId = `demo-tool-${toolTaskSequence}`;
    toolKinds.set(taskId, "ping");
    const total = targets.length;
    let step = 0;
    emit("tool:event", { event: "pingProgress", taskId, done: 0, total });
    toolTimers.set(
      taskId,
      setInterval(() => {
        const target = targets[step];
        step += 1;
        if (target) {
          const received = step === 2 ? 0 : Math.min(input.count, 2 + (step % 3));
          const result: PingHostResult = {
            host: target,
            address: target.includes("/") ? "192.0.2.10" : target,
            sent: input.count,
            received,
            lossPercent: Math.round(((input.count - received) / input.count) * 1000) / 10,
            minMs: received ? 1.2 + step : undefined,
            avgMs: received ? 2.4 + step : undefined,
            maxMs: received ? 4.1 + step : undefined,
            status: received ? "ok" : "timeout",
            error: received ? undefined : { code: "timeout", detail: "ICMP timeout" },
          };
          emit("tool:event", { event: "pingResult", taskId, result });
        }
        emit("tool:event", {
          event: "pingProgress",
          taskId,
          done: Math.min(step, total),
          total,
        });
        if (step >= total) {
          clearInterval(toolTimers.get(taskId));
          toolTimers.delete(taskId);
          toolKinds.delete(taskId);
          emit("tool:event", {
            event: "pingDone",
            taskId,
            completed: total,
            failed: 1,
            error: undefined,
          });
        }
      }, 260),
    );
    return taskId;
  }

  async function toolTracerouteStart(input: TracerouteInput): Promise<string> {
    await demoWait(120);
    toolTaskSequence += 1;
    const taskId = `demo-tool-${toolTaskSequence}`;
    toolKinds.set(taskId, "traceroute");
    let hop = 0;
    toolTimers.set(
      taskId,
      setInterval(() => {
        hop += 1;
        const reached = hop >= 5;
        emit("tool:event", {
          event: "tracerouteHop",
          taskId,
          hop: {
            hop,
            address: reached ? "192.0.2.10" : `10.0.0.${hop}`,
            hostname: reached ? input.target : `hop-${hop}.demo.local`,
            rttsMs: [1.1 + hop, null, 2.2 + hop],
            status: reached ? "reached" : "ttlExpired",
            reached,
          },
        });
        if (reached) {
          clearInterval(toolTimers.get(taskId));
          toolTimers.delete(taskId);
          toolKinds.delete(taskId);
          emit("tool:event", { event: "tracerouteDone", taskId, reached: true });
        }
      }, 320),
    );
    return taskId;
  }

  async function toolLatencyStart(input: ContinuousPingInput): Promise<string> {
    await demoWait(120);
    if (!input.host.trim()) {
      throw { code: "invalidHost", detail: "Target host is required" };
    }
    toolTaskSequence += 1;
    const taskId = `demo-tool-${toolTaskSequence}`;
    toolKinds.set(taskId, "latency");
    let seq = 0;
    let sent = 0;
    let received = 0;
    const rtts: number[] = [];
    toolTimers.set(
      taskId,
      setInterval(
        () => {
          seq += 1;
          sent += 1;
          const lost = seq % 11 === 0 || seq % 17 === 0;
          const base = 12 + Math.sin(seq / 3) * 4;
          const rtt = lost ? undefined : Math.round((base + Math.random() * 6) * 10) / 10;
          if (rtt !== undefined) {
            received += 1;
            rtts.push(rtt);
          }
          const stats = latencyStats.get(taskId);
          if (stats) {
            stats.sent = sent;
            stats.received = received;
          }
          emit("tool:event", {
            event: "latencySample",
            taskId,
            sample: {
              taskId,
              timestampMs: Date.now(),
              address: input.host.includes(".") ? input.host : "192.0.2.10",
              status: lost ? "timeout" : "ok",
              rttMs: rtt,
              seq,
            },
          });
        },
        Math.max(200, input.intervalMs),
      ),
    );
    latencyStats.set(taskId, { sent, received, rtts });
    return taskId;
  }

  async function toolTaskCancel(taskId: string): Promise<boolean> {
    const timer = toolTimers.get(taskId);
    if (!timer) return false;
    clearInterval(timer);
    toolTimers.delete(taskId);
    const kind = toolKinds.get(taskId);
    toolKinds.delete(taskId);
    if (kind === "traceroute") {
      emit("tool:event", {
        event: "tracerouteDone",
        taskId,
        reached: false,
        error: { code: "cancelled", detail: "Task cancelled" },
      });
    } else if (kind === "latency") {
      const stats = latencyStats.get(taskId) ?? { sent: 0, received: 0, rtts: [] };
      latencyStats.delete(taskId);
      emit("tool:event", {
        event: "latencyDone",
        taskId,
        sent: stats.sent,
        received: stats.received,
        lost: stats.sent - stats.received,
        minMs: stats.rtts.length ? Math.min(...stats.rtts) : undefined,
        avgMs: stats.rtts.length
          ? Math.round(
              (stats.rtts.reduce((sum, value) => sum + value, 0) / stats.rtts.length) * 10,
            ) / 10
          : undefined,
        maxMs: stats.rtts.length ? Math.max(...stats.rtts) : undefined,
        error: { code: "cancelled", detail: "Task cancelled" },
      });
    } else if (kind === "hash") {
      emit("tool:event", {
        event: "hashDone",
        taskId,
        error: { code: "cancelled", detail: "Task cancelled" },
      });
    } else if (kind === "search") {
      emit("tool:event", {
        event: "searchDone",
        taskId,
        files: 0,
        matches: 0,
        skipped: 0,
        truncated: false,
        error: { code: "cancelled", detail: "Task cancelled" },
      });
    } else {
      emit("tool:event", {
        event: "pingDone",
        taskId,
        completed: 0,
        failed: 0,
        error: { code: "cancelled", detail: "Task cancelled" },
      });
    }
    return true;
  }

  async function toolHttpRequest(input: HttpRequestInput): Promise<HttpResponse> {
    await demoWait(320);
    if (!/^https?:\/\//i.test(input.url.trim())) {
      throw { code: "invalidUrl", detail: "Only http and https URLs are supported" };
    }
    const ok = !input.url.includes("/404");
    const body =
      input.method === "HEAD"
        ? ""
        : JSON.stringify(
            {
              demo: true,
              method: input.method,
              url: input.url,
              bodyMode: input.bodyMode,
              receivedBody: input.body ?? null,
              formFields: input.formFields,
              uploadedFile: input.uploadPath ?? null,
            },
            null,
            2,
          );
    return {
      status: ok ? 200 : 404,
      statusText: ok ? "OK" : "Not Found",
      finalUrl: input.url,
      elapsedMs: 32.4,
      headers: [
        { name: "content-type", value: "application/json; charset=utf-8" },
        { name: "x-modbus-tool-demo", value: "1" },
      ],
      body,
      bodyBytes: new TextEncoder().encode(body).length,
      truncated: false,
      contentType: "application/json; charset=utf-8",
      binary: false,
    };
  }

  async function cancelScan(scanId: string): Promise<void> {
    const interval = scanTimers.get(scanId);
    if (interval) clearInterval(interval);
    scanTimers.delete(scanId);
    finishDemoScan(scanId, "扫描已取消");
  }

  async function createDefinition(sessionId: string, input: DefinitionInput): Promise<Definition> {
    const session = sessionById(sessionId);
    const draft: Definition = {
      ...input,
      id: `demo-def-${Date.now().toString(36)}`,
      dataType: input.functionCode === 1 || input.functionCode === 2 ? "bool" : input.dataType,
      points: input.points ?? [],
    };
    const definition: Definition = {
      ...draft,
      points: draft.points.length ? draft.points : resolvePoints(draft),
    };
    session.definitions.push(definition);
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
    return clone(definition);
  }

  async function updateDefinition(
    sessionId: string,
    definitionId: string,
    input: DefinitionInput,
  ): Promise<Definition> {
    const session = sessionById(sessionId);
    const index = session.definitions.findIndex((definition) => definition.id === definitionId);
    if (index < 0) throw new Error("定义不存在");
    const draft: Definition = {
      ...input,
      id: definitionId,
      dataType: input.functionCode === 1 || input.functionCode === 2 ? "bool" : input.dataType,
      points: input.points ?? [],
    };
    const definition: Definition = {
      ...draft,
      points: draft.points.length ? draft.points : resolvePoints(draft),
    };
    session.definitions[index] = definition;
    lastRun.delete(definitionId);
    if (session.role === "slave" && session.status === "connected") {
      if (!definition.points.length) {
        session.status = "disconnected";
        session.polling = false;
      } else {
        emitSnapshot(session, definition);
      }
      emit("session:status", statusPayload(session));
    }
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
    return clone(definition);
  }

  async function deleteDefinition(sessionId: string, definitionId: string): Promise<void> {
    const session = sessionById(sessionId);
    const index = session.definitions.findIndex((definition) => definition.id === definitionId);
    if (index < 0) throw new Error("定义不存在");
    session.definitions.splice(index, 1);
    lastRun.delete(definitionId);
    latestSnapshots.delete(snapshotKey(sessionId, definitionId));
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
  }

  async function writeRegister(
    sessionId: string,
    definitionId: string,
    address: number,
    value: string,
  ): Promise<void> {
    const session = sessionById(sessionId);
    const definition = session.definitions.find((candidate) => candidate.id === definitionId);
    if (!definition) throw new Error("定义不存在");

    let words: number[];
    let formatted: string;
    let numeric: number | null;
    if (definition.dataType === "f32" || definition.dataType === "f64") {
      const parsed = Number(value);
      if (!Number.isFinite(parsed)) throw new Error("请输入有效数字");
      words = floatToWords(parsed, definition.byteOrder);
      formatted = parsed.toFixed(2);
      numeric = parsed;
    } else {
      const parsed = Number(value.replace(/,/g, ""));
      if (!Number.isFinite(parsed)) throw new Error("请输入有效数字");
      const rounded = Math.max(0, Math.min(0xffff, Math.round(parsed)));
      words = uint16Words(rounded, definition.byteOrder);
      formatted = formatNumber(rounded);
      numeric = rounded;
    }

    const key = snapshotKey(sessionId, definitionId);
    const snapshot = latestSnapshots.get(key) ?? snapshotFor(session, definition);
    const target = snapshot.values.find((entry) => entry.address === address);
    if (!target) throw new Error("地址不在当前定义范围内");
    target.words = words;
    target.formatted = formatted;
    target.numeric = numeric;
    snapshot.timestamp = Date.now();
    storeSnapshot(snapshot);
    emit("session:data", snapshot);

    const frames = buildWriteFrames(session, definition, address, words);
    session.stats.tx += 1;
    session.stats.rx += 1;
    emit("session:frames", { sessionId, frames });
    emit("session:status", statusPayload(session));
  }

  async function startDebug(sessionId: string): Promise<void> {
    const session = sessionById(sessionId);
    if (session.role !== "master") throw new Error("从站模拟会话不支持原始调试");
    if (session.status !== "connected") throw new Error("会话未连接，请先启动链路");
    if (session.polling) throw new Error("请先停止轮询后再进入调试模式");
    if (debugSessionId) throw new Error("调试模式已开启");

    debugSessionId = sessionId;
    debugHeartbeat = 0;
    emit("debug:state", { sessionId, active: true });
    debugTimer = setInterval(() => {
      if (!debugSessionId) return;
      debugHeartbeat += 1;
      const text = `DEVICE READY ${debugHeartbeat.toString().padStart(4, "0")}\r\n`;
      const data = Array.from(new TextEncoder().encode(text));
      emitDebug(debugSessionId, "rx", data);
      session.stats.rx += 1;
      emit("session:status", statusPayload(session));
    }, 1600);
  }

  async function stopDebug(sessionId: string): Promise<void> {
    if (!debugSessionId) return;
    if (debugSessionId !== sessionId) throw new Error("调试会话不匹配");
    stopDebugLoop(true);
  }

  async function sendDebug(sessionId: string, input: RawSendInput): Promise<number> {
    if (debugSessionId !== sessionId) throw new Error("调试模式未开启");
    const session = sessionById(sessionId);
    const payload = Array.from(
      encodeRawPayload(input.data, input.mode, {
        appendCr: input.appendCr,
        appendLf: input.appendLf,
      }),
    );
    emitDebug(sessionId, "tx", payload);
    session.stats.tx += 1;
    emit("session:status", statusPayload(session));
    setTimeout(() => {
      if (debugSessionId !== sessionId) return;
      emitDebug(sessionId, "rx", payload);
      session.stats.rx += 1;
      emit("session:status", statusPayload(session));
    }, 120);
    return payload.length;
  }

  async function startNetwork(sessionId: string, input: NetworkStartInput): Promise<NetworkState> {
    const serverMode = input.mode === "tcp-server" || input.mode === "udp-server";
    const relayMode = input.mode === "tcp-relay";
    if (serverMode && (input.localPort < 1 || input.localPort > 65535)) {
      throw new Error("监听端口需在 1–65535 之间");
    }
    if (!serverMode && !input.remoteHost?.trim()) {
      throw new Error("请输入远端主机地址");
    }
    if (!serverMode && (!input.remotePort || input.remotePort < 1)) {
      throw new Error("远端端口需在 1–65535 之间");
    }
    if (relayMode && (input.localPort < 1 || input.localPort > 65535)) {
      throw new Error("本地监听端口需在 1–65535 之间");
    }
    const localHost = input.localHost.trim() || "0.0.0.0";
    const endpoint = serverMode
      ? `${localHost}:${input.localPort}`
      : relayMode
        ? `${localHost}:${input.localPort} → ${input.remoteHost}:${input.remotePort}`
        : `${input.remoteHost}:${input.remotePort}`;
    const state: NetworkState = {
      sessionId,
      active: true,
      mode: input.mode,
      endpoint,
      clients: 1,
    };
    demoNetworks.set(sessionId, state);
    emit("network:state", state);
    return state;
  }

  async function stopNetwork(sessionId: string): Promise<void> {
    const timer = networkEchoTimers.get(sessionId);
    if (timer) {
      clearTimeout(timer);
      networkEchoTimers.delete(sessionId);
    }
    const previous = demoNetworks.get(sessionId);
    const state: NetworkState = {
      sessionId,
      active: false,
      mode: previous?.mode ?? null,
      endpoint: previous?.endpoint ?? null,
      clients: 0,
    };
    demoNetworks.set(sessionId, state);
    emit("network:state", state);
  }

  async function sendNetwork(sessionId: string, input: RawSendInput): Promise<number> {
    const state = demoNetworks.get(sessionId);
    if (!state?.active) throw new Error("网络助手尚未启动");
    const payload = Array.from(
      encodeRawPayload(input.data, input.mode, {
        appendCr: input.appendCr,
        appendLf: input.appendLf,
      }),
    );
    emit("network:data", {
      sessionId,
      direction: "tx",
      data: payload,
      connection: state.endpoint ?? "demo",
      timestamp: Date.now(),
    });
    const previous = networkEchoTimers.get(sessionId);
    if (previous) clearTimeout(previous);
    networkEchoTimers.set(
      sessionId,
      setTimeout(() => {
        emit("network:data", {
          sessionId,
          direction: "rx",
          data: payload,
          connection: state.endpoint ?? "demo",
          timestamp: Date.now(),
        });
        networkEchoTimers.delete(sessionId);
      }, 180),
    );
    return payload.length;
  }

  async function writeCustom(
    sessionId: string,
    definitionId: string | null,
    input: CustomWriteInput,
  ): Promise<void> {
    const session = sessionById(sessionId);
    const definition = definitionId
      ? session.definitions.find((candidate) => candidate.id === definitionId)
      : undefined;
    if (input.slaveId < 1 || input.slaveId > 247) {
      throw new Error("从站 ID 需在 1–247 之间");
    }
    if (definition && input.slaveId !== definition.slaveId) {
      throw new Error("Modbus 异常 0x0B：网关无法连接目标从站");
    }
    const rawValues = input.values.map((value) => value.trim()).filter(Boolean);
    if (!rawValues.length) throw new Error("请输入写入值");

    const quantity = rawValues.length;
    if (quantity > 1 && input.address + quantity > 0x10000) {
      throw new Error("写入地址范围超出 65535");
    }
    const addressHigh = (input.address >> 8) & 0xff;
    const addressLow = input.address & 0xff;
    const quantityHigh = (quantity >> 8) & 0xff;
    const quantityLow = quantity & 0xff;
    const writes: Array<{ address: number; value: number; bit: boolean }> = [];
    let requestPdu: number[];
    let responsePdu: number[];
    let label: string;

    switch (input.functionCode) {
      case 5: {
        if (quantity !== 1) throw new Error("FC05 只能写入一个线圈");
        const on = parseDemoCoil(rawValues[0] ?? "0");
        requestPdu = [0x05, addressHigh, addressLow, on ? 0xff : 0x00, 0x00];
        responsePdu = requestPdu;
        writes.push({ address: input.address, value: on ? 1 : 0, bit: true });
        label = "FC05";
        break;
      }
      case 6: {
        if (quantity !== 1) throw new Error("FC06 只能写入一个寄存器");
        const value = parseDemoRegister(rawValues[0] ?? "0");
        requestPdu = [0x06, addressHigh, addressLow, (value >> 8) & 0xff, value & 0xff];
        responsePdu = requestPdu;
        writes.push({ address: input.address, value, bit: false });
        label = "FC06";
        break;
      }
      case 15: {
        if (quantity > 1968) throw new Error("FC0F 一次最多写入 1968 个线圈");
        const coils = rawValues.map(parseDemoCoil);
        const byteCount = Math.ceil(quantity / 8);
        const packed = Array.from({ length: byteCount }, () => 0);
        coils.forEach((on, index) => {
          if (on) packed[Math.floor(index / 8)] |= 1 << (index % 8);
        });
        requestPdu = [
          0x0f,
          addressHigh,
          addressLow,
          0x00,
          quantityHigh,
          quantityLow,
          byteCount,
          ...packed,
        ];
        responsePdu = [0x0f, addressHigh, addressLow, 0x00, quantityHigh, quantityLow];
        coils.forEach((on, index) => {
          writes.push({ address: input.address + index, value: on ? 1 : 0, bit: true });
        });
        label = "FC0F";
        break;
      }
      case 16: {
        if (quantity > 123) throw new Error("FC10 一次最多写入 123 个寄存器");
        const registers = rawValues.map(parseDemoRegister);
        requestPdu = [
          0x10,
          addressHigh,
          addressLow,
          0x00,
          quantityHigh,
          quantityLow,
          quantity * 2,
          ...wordsToDataBytes(registers),
        ];
        responsePdu = [0x10, addressHigh, addressLow, 0x00, quantityHigh, quantityLow];
        registers.forEach((value, index) => {
          writes.push({ address: input.address + index, value, bit: false });
        });
        label = "FC10";
        break;
      }
      default:
        throw new Error("写入功能码仅支持 FC05 / FC06 / FC0F / FC10");
    }

    const transactionId = nextTransaction(session.id);
    const requestRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, input.slaveId, requestPdu)
        : rtuFrame(input.slaveId, requestPdu);
    const responseRaw =
      session.framing === "mbap"
        ? mbapFrame(transactionId, input.slaveId, responsePdu)
        : rtuFrame(input.slaveId, responsePdu);
    const frames: Frame[] = [
      makeFrame(
        session,
        "tx",
        requestRaw,
        `${label} · 从站 ${input.slaveId} · 写入 ${input.address} × ${quantity}`,
      ),
      makeFrame(session, "rx", responseRaw, `${label} · 写入确认`, 2 + Math.random() * 4),
    ];
    session.stats.tx += 1;
    session.stats.rx += 1;
    emit("session:frames", { sessionId, frames });
    emit("session:status", statusPayload(session));

    if (!definition) return;
    const key = snapshotKey(sessionId, definition.id);
    const snapshot = latestSnapshots.get(key);
    if (!snapshot) return;
    const next = {
      ...snapshot,
      timestamp: Date.now(),
      error: undefined,
      values: snapshot.values.map((entry) => {
        const written = writes.find((item) => item.address === entry.address);
        if (!written) return entry;
        return {
          ...entry,
          words: [written.value],
          formatted: written.bit ? (written.value ? "ON" : "OFF") : formatNumber(written.value),
          numeric: written.value,
          stale: false,
          error: undefined,
        };
      }),
    };
    storeSnapshot(next);
    emit("session:data", next);
  }

  let demoFileServer: FileServerStatus = {
    running: false,
    root: null,
    host: null,
    port: null,
    urls: [],
    startedAt: null,
    uploads: 0,
    downloads: 0,
    files: [],
    requests: [],
  };

  async function toolFileserverStart(input: FileServerStartInput): Promise<FileServerStatus> {
    await demoWait(240);
    if (!input.root.trim()) {
      throw { code: "invalidRoot", detail: "Select a folder to share" };
    }
    const host = input.host || "127.0.0.1";
    const port = input.port || 80;
    const urls =
      host === "0.0.0.0"
        ? [`http://127.0.0.1:${port}`, `http://192.168.1.10:${port}`]
        : [`http://${host}:${port}`];
    demoFileServer = {
      running: true,
      root: input.root,
      host,
      port,
      urls,
      startedAt: Date.now(),
      uploads: 2,
      downloads: 5,
      files: [
        { name: "firmware.bin", size: 2_457_600, modifiedMs: Date.now() - 3_600_000 },
        { name: "notes.txt", size: 1_824, modifiedMs: Date.now() - 900_000 },
      ],
      requests: [
        {
          timestampMs: Date.now() - 42_000,
          remote: "192.168.1.24",
          method: "GET",
          path: "/download/firmware.bin",
          status: 200,
          bytes: 2_457_600,
        },
        {
          timestampMs: Date.now() - 120_000,
          remote: "192.168.1.31",
          method: "POST",
          path: "/upload",
          status: 303,
          bytes: 18_432,
        },
      ],
    };
    return clone(demoFileServer);
  }

  async function toolFileserverStop(): Promise<FileServerStatus> {
    await demoWait(180);
    demoFileServer = {
      ...demoFileServer,
      running: false,
      urls: [],
    };
    return clone(demoFileServer);
  }

  async function toolFileserverStatus(): Promise<FileServerStatus> {
    await demoWait(60);
    return clone(demoFileServer);
  }

  const mqttTimers = new Map<string, ReturnType<typeof setInterval>>();

  async function toolMqttStart(input: MqttBrokerInput): Promise<void> {
    await demoWait(160);
    if (!input.host.trim()) {
      throw { code: "invalidHost", detail: "Broker host is required" };
    }
    if (mqttTimers.has(input.brokerId)) {
      throw { code: "brokerAlreadyRunning", detail: "This broker is already connected" };
    }
    emit("mqtt:state", {
      brokerId: input.brokerId,
      status: "connecting",
      detail: null,
      timestampMs: Date.now(),
    });
    let step = 0;
    let connected = false;
    const timer = setInterval(() => {
      step += 1;
      if (!connected) {
        connected = true;
        emit("mqtt:state", {
          brokerId: input.brokerId,
          status: "connected",
          detail: null,
          timestampMs: Date.now(),
        });
        return;
      }
      if (step % 3 === 0) {
        const payload = JSON.stringify({ demo: true, seq: step, value: 20 + Math.random() * 6 });
        emit("mqtt:message", {
          brokerId: input.brokerId,
          topic: "demo/telemetry",
          payloadText: payload,
          payloadHex: Array.from(new TextEncoder().encode(payload))
            .map((byte) => byte.toString(16).padStart(2, "0").toUpperCase())
            .join(" "),
          payloadBytes: new TextEncoder().encode(payload).length,
          payloadTruncated: false,
          qos: 0,
          retain: false,
          timestampMs: Date.now(),
        });
      }
    }, 700);
    mqttTimers.set(input.brokerId, timer);
  }

  async function toolMqttStop(brokerId: string): Promise<boolean> {
    await demoWait(120);
    const timer = mqttTimers.get(brokerId);
    if (!timer) return false;
    clearInterval(timer);
    mqttTimers.delete(brokerId);
    emit("mqtt:state", {
      brokerId,
      status: "disconnected",
      detail: null,
      timestampMs: Date.now(),
    });
    return true;
  }

  async function toolMqttPublish(input: MqttPublishInput): Promise<void> {
    await demoWait(80);
    if (!mqttTimers.has(input.brokerId)) {
      throw { code: "brokerNotRunning", detail: "Broker is not connected" };
    }
    if (!input.topic.trim()) {
      throw { code: "invalidTopic", detail: "Topic is required" };
    }
  }

  async function toolMqttSubscribe(input: MqttSubscribeInput): Promise<void> {
    await demoWait(80);
    if (!mqttTimers.has(input.brokerId)) {
      throw { code: "brokerNotRunning", detail: "Broker is not connected" };
    }
    if (!input.topic.trim()) {
      throw { code: "invalidTopic", detail: "Topic filter is required" };
    }
  }

  async function toolMqttUnsubscribe(_brokerId: string, _topic: string): Promise<void> {
    await demoWait(60);
  }

  async function on<K extends AppEventName>(
    event: K,
    handler: (payload: AppEventMap[K]) => void,
  ): Promise<() => void> {
    const set = listeners.get(event) ?? new Set<AnyHandler>();
    set.add(handler as AnyHandler);
    listeners.set(event, set);
    return () => {
      set.delete(handler as AnyHandler);
    };
  }

  return {
    mode: "demo",
    boot,
    listSerialPorts,
    createSession,
    deleteSession,
    connectSession,
    disconnectSession,
    updateSession,
    writeSlaveValue,
    saveTextFile,
    openTextFile,
    setPolling,
    readOnce,
    setDefinitionEnabled,
    createDefinition,
    updateDefinition,
    deleteDefinition,
    scanSlaveIds,
    scanAddresses,
    scanBaudRates,
    scanNetwork,
    cancelScan,
    toolDnsQuery,
    toolTelnetTest,
    toolHttpRequest,
    toolPingStart,
    toolLatencyStart,
    toolTracerouteStart,
    toolTaskCancel,
    toolArpCapabilities,
    toolArpScan,
    toolHashStart,
    toolSearchStart,
    toolFileserverStart,
    toolFileserverStop,
    toolFileserverStatus,
    toolMqttStart,
    toolMqttStop,
    toolMqttPublish,
    toolMqttSubscribe,
    toolMqttUnsubscribe,
    pickFile,
    pickDirectory,
    readTextFile,
    writeRegister,
    writeCustom,
    startDebug,
    stopDebug,
    sendDebug,
    startNetwork,
    stopNetwork,
    sendNetwork,
    on,
  };
}
