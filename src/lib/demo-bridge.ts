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
  NewSessionInput,
  RegisterValue,
  ScanKind,
  ScanProgress,
  ScanResult,
  SerialPortInfo,
  Session,
  SessionAccent,
  SessionStatusEvent,
  SlaveIdScanInput,
  WorkspaceSnapshot,
} from "@/types/models";
import {
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
import { formatFunctionCode, formatNumber, snapshotKey } from "./format";

const TICK_MS = 720;
const MAX_FRAME_HISTORY = 400;

type AnyHandler = (payload: unknown) => void;

function clone<T>(value: T): T {
  return structuredClone(value);
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
  return {
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
  };
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

  function emit<K extends AppEventName>(event: K, payload: AppEventMap[K]): void {
    listeners.get(event)?.forEach((handler) => handler(payload));
  }

  function sessionById(sessionId: string): Session {
    const session = sessions.find((candidate) => candidate.id === sessionId);
    if (!session) throw new Error(`会话不存在: ${sessionId}`);
    return session;
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

  function snapshotFor(
    session: Session,
    definition: Definition,
    currentTick = tick,
  ): DefinitionSnapshot {
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
            16,
            "u16",
            "abcd",
            900,
            { 0: "设定温度 ×10", 1: "设定压力 ×100", 2: "运行模式", 3: "心跳计数" },
            input.slaveId ?? 10,
          )
        : null;
    const session: Session = {
      id,
      name: input.name || (role === "slave" ? "模拟器 · 新从站" : "新会话"),
      role,
      transport: input.transport,
      framing: input.framing,
      endpoint: input.endpoint,
      status: "connected",
      polling: role === "master",
      accent,
      stats: { tx: 0, rx: 0, errors: 0, latencyMs: input.transport === "serial" ? 18 : 6 },
      startedAt: Date.now(),
      definitions: definition ? [definition] : [],
    };
    sessions.push(session);
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
    emit("session:status", statusPayload(session));
    setTimeout(() => {
      if (session.role === "master" && session.polling) runSessionTick(session, tick);
    }, 180);
    return clone(session);
  }

  async function deleteSession(sessionId: string): Promise<void> {
    const index = sessions.findIndex((session) => session.id === sessionId);
    if (index < 0) throw new Error("会话不存在");
    sessions.splice(index, 1);
    for (const key of [...latestSnapshots.keys()]) {
      if (key.startsWith(`${sessionId}:`)) latestSnapshots.delete(key);
    }
    emit("workspace:updated", { sessions: clone(sessions), version: 1, demo: true });
  }

  async function setPolling(sessionId: string, enabled: boolean): Promise<void> {
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
    const foundHosts = new Map<number, string>([
      [12, "192.168.1.10"],
      [28, "192.168.1.20"],
      [95, "192.168.1.100"],
    ]);
    return startDemoScan(
      "network",
      total,
      (step) => {
        const host = foundHosts.get(step);
        if (!host) return null;
        return {
          label: `${host}:${input.port}`,
          status: input.probeModbus ? "ok" : "open",
          detail: input.probeModbus ? "Modbus 从站响应" : "TCP 端口开放",
          endpoint: `${host}:${input.port}`,
        };
      },
      45,
    );
  }

  async function cancelScan(scanId: string): Promise<void> {
    const interval = scanTimers.get(scanId);
    if (interval) clearInterval(interval);
    scanTimers.delete(scanId);
    finishDemoScan(scanId, "扫描已取消");
  }

  async function createDefinition(sessionId: string, input: DefinitionInput): Promise<Definition> {
    const session = sessionById(sessionId);
    const definition: Definition = {
      ...input,
      id: `demo-def-${Date.now().toString(36)}`,
      dataType: input.functionCode === 1 || input.functionCode === 2 ? "bool" : input.dataType,
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
    const definition: Definition = {
      ...input,
      id: definitionId,
      dataType: input.functionCode === 1 || input.functionCode === 2 ? "bool" : input.dataType,
    };
    session.definitions[index] = definition;
    lastRun.delete(definitionId);
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
    writeRegister,
    writeCustom,
    on,
  };
}
