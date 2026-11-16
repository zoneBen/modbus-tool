export type TransportKind = "serial" | "tcp" | "udp";
export type Framing = "rtu" | "ascii" | "mbap";
export type SessionRole = "master" | "slave";
export type SessionStatus = "connected" | "connecting" | "disconnected" | "error";
export type SessionAccent = "signal" | "sim" | "rx";
export type FunctionCode = 1 | 2 | 3 | 4 | 5 | 6 | 15 | 16;

export type DataType =
  | "bool"
  | "i8"
  | "u8"
  | "i16"
  | "u16"
  | "i32"
  | "u32"
  | "i64"
  | "u64"
  | "f32"
  | "f64"
  | "hex"
  | "bin"
  | "ascii";

export type ByteOrder = "abcd" | "dcba" | "badc" | "cdab";

export interface Definition {
  id: string;
  name: string;
  functionCode: FunctionCode;
  slaveId: number;
  startAddress: number;
  quantity: number;
  dataType: DataType;
  byteOrder: ByteOrder;
  intervalMs: number;
  enabled: boolean;
  aliases: Record<number, string>;
}

export type DefinitionInput = Omit<Definition, "id">;

export type WriteFunctionCode = 5 | 6 | 15 | 16;

export interface CustomWriteInput {
  slaveId: number;
  functionCode: WriteFunctionCode;
  address: number;
  values: string[];
}

export interface SessionStats {
  tx: number;
  rx: number;
  errors: number;
  latencyMs: number;
}

export interface Session {
  id: string;
  name: string;
  role: SessionRole;
  transport: TransportKind;
  framing: Framing;
  endpoint: string;
  status: SessionStatus;
  polling: boolean;
  accent: SessionAccent;
  definitions: Definition[];
  stats: SessionStats;
  startedAt: number;
  host?: string;
  port?: number;
  serialPort?: string;
  baudRate?: number;
  dataBits?: number;
  parity?: "none" | "even" | "odd";
  stopBits?: number;
  timeoutMs?: number;
  retries?: number;
  interFrameDelayMs?: number;
  autoReconnect?: boolean;
}

export interface RegisterValue {
  address: number;
  words: number[];
  formatted: string;
  numeric: number | null;
  stale?: boolean;
  error?: string;
}

export interface DefinitionTimeline {
  timestamps: number[];
  series: Record<number, Array<number | null>>;
}

export interface DefinitionSnapshot {
  sessionId: string;
  definitionId: string;
  timestamp: number;
  values: RegisterValue[];
  latencyMs: number;
  error?: string;
}

export interface FrameField {
  label: string;
  value: string;
  offset: number;
  length: number;
  tone: "header" | "unit" | "function" | "data" | "checksum";
}

export interface DecodedFrame {
  slaveId: number;
  functionCode: number;
  ascii?: string;
  fields: FrameField[];
}

export interface Frame {
  seq: number;
  sessionId: string;
  timestamp: number;
  direction: "tx" | "rx";
  raw: number[];
  decoded: DecodedFrame;
  summary: string;
  latencyMs?: number;
  status: "ok" | "exception" | "timeout" | "error";
}

export interface SerialPortInfo {
  path: string;
  label: string;
  manufacturer?: string;
}

export interface WorkspaceSnapshot {
  sessions: Session[];
  version: number;
  demo: boolean;
}

export interface NewSessionInput {
  name: string;
  role: SessionRole;
  transport: TransportKind;
  framing: Framing;
  endpoint: string;
  host?: string;
  port?: number;
  serialPort?: string;
  baudRate?: number;
  dataBits?: number;
  parity?: "none" | "even" | "odd";
  stopBits?: number;
  timeoutMs?: number;
  retries?: number;
  interFrameDelayMs?: number;
  autoReconnect?: boolean;
  slaveId?: number;
}

export interface UpdateSessionInput {
  name: string;
  transport: TransportKind;
  framing: Framing;
  endpoint: string;
  host?: string;
  port?: number;
  serialPort?: string;
  baudRate?: number;
  dataBits?: number;
  parity?: "none" | "even" | "odd";
  stopBits?: number;
  timeoutMs?: number;
  retries?: number;
  interFrameDelayMs?: number;
  autoReconnect?: boolean;
}

export interface SlaveValueInput {
  address: number;
  dataType: DataType;
  byteOrder: ByteOrder;
  value: string;
}

export interface SessionStatusEvent {
  sessionId: string;
  status: SessionStatus;
  polling: boolean;
  stats: SessionStats;
  error?: string;
}

export type ScanKind = "slaveId" | "address" | "baud" | "network";

export interface ScanLinkInput {
  transport: TransportKind;
  framing: Framing;
  host?: string;
  port?: number;
  serialPort?: string;
  baudRate?: number;
  dataBits?: number;
  parity?: "none" | "even" | "odd";
  stopBits?: number;
  timeoutMs?: number;
}

export interface SlaveIdScanInput {
  link: ScanLinkInput;
  functionCode: FunctionCode;
  address: number;
  quantity: number;
  idStart: number;
  idEnd: number;
}

export interface AddressScanInput {
  link: ScanLinkInput;
  functionCode: FunctionCode;
  slaveId: number;
  startAddress: number;
  endAddress: number;
  blockSize: number;
  nonZeroOnly: boolean;
}

export interface BaudScanInput {
  serialPort: string;
  framing: Framing;
  dataBits?: number;
  parity?: "none" | "even" | "odd";
  stopBits?: number;
  baudRates: number[];
  slaveId: number;
  functionCode: FunctionCode;
  address: number;
  quantity: number;
  timeoutMs: number;
}

export interface NetworkScanInput {
  cidr: string;
  port: number;
  timeoutMs: number;
  concurrency: number;
  probeModbus: boolean;
  slaveId: number;
}

export interface ScanProgress {
  scanId: string;
  kind: ScanKind;
  done: number;
  total: number;
  found: number;
  phase: string;
  finished: boolean;
  error?: string;
}

export interface ScanResult {
  scanId: string;
  kind: ScanKind;
  label: string;
  status: "ok" | "exception" | "timeout" | "error" | "open";
  detail: string;
  latencyMs?: number;
  slaveId?: number;
  address?: number;
  baudRate?: number;
  endpoint?: string;
}

export interface ScanDone {
  scanId: string;
  kind: ScanKind;
  found: number;
  error?: string;
}
