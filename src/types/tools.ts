import type { RawSendMode } from "./models";

export interface ToolFileFilter {
  name: string;
  extensions: string[];
}

export interface ToolErrorPayload {
  code: string;
  detail: string;
}

export class ToolError extends Error {
  readonly code: string;
  readonly detail: string;

  constructor(code: string, detail: string) {
    super(detail || code);
    this.name = "ToolError";
    this.code = code;
    this.detail = detail;
  }
}

export function toToolError(error: unknown): ToolError {
  if (error instanceof ToolError) return error;
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<ToolErrorPayload>;
    if (typeof candidate.code === "string") {
      return new ToolError(candidate.code, candidate.detail ?? candidate.code);
    }
  }
  if (error instanceof Error) return new ToolError("unknown", error.message);
  return new ToolError("unknown", String(error));
}

export type DnsRecordType =
  "A" | "AAAA" | "CNAME" | "MX" | "TXT" | "NS" | "SOA" | "SRV" | "PTR" | "CAA";

export interface DnsQueryInput {
  name: string;
  recordType: DnsRecordType;
  server?: string;
  timeoutMs: number;
}

export interface DnsAnswer {
  name: string;
  recordType: string;
  ttl: number;
  value: string;
}

export interface DnsResult {
  question: string;
  recordType: string;
  server: string;
  status: "ok" | "nxdomain" | "noRecords";
  elapsedMs: number;
  answers: DnsAnswer[];
}

export interface TelnetTestInput {
  host: string;
  port: number;
  timeoutMs: number;
  readTimeoutMs: number;
  payload?: string;
  payloadMode: RawSendMode;
  appendCr: boolean;
  appendLf: boolean;
}

export interface TelnetResult {
  host: string;
  port: number;
  connected: boolean;
  latencyMs: number;
  bytesRead: number;
  text: string;
  hex: string;
}

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD";
export type HttpBodyMode = "none" | "raw" | "json" | "form" | "multipart";

export interface HttpHeaderInput {
  name: string;
  value: string;
}

export interface HttpFormFieldInput {
  name: string;
  value: string;
}

export interface HttpRequestInput {
  method: HttpMethod;
  url: string;
  headers: HttpHeaderInput[];
  body?: string;
  bodyMode: HttpBodyMode;
  formFields: HttpFormFieldInput[];
  uploadPath?: string;
  uploadField?: string;
  timeoutMs: number;
  followRedirects: boolean;
  caPath?: string;
}

export interface HttpHeader {
  name: string;
  value: string;
}

export interface HttpResponse {
  status: number;
  statusText: string;
  finalUrl: string;
  elapsedMs: number;
  headers: HttpHeader[];
  body: string;
  bodyBytes: number;
  truncated: boolean;
  contentType?: string;
  binary: boolean;
}

export interface PingInput {
  targets: string;
  count: number;
  intervalMs: number;
  timeoutMs: number;
  concurrency: number;
  payloadSize: number;
}

export interface PingHostResult {
  host: string;
  address?: string;
  sent: number;
  received: number;
  lossPercent: number;
  minMs?: number;
  avgMs?: number;
  maxMs?: number;
  status: "ok" | "timeout" | "error" | "cancelled";
  error?: ToolErrorPayload;
}

export interface PingDone {
  taskId: string;
  completed: number;
  failed: number;
  error?: ToolErrorPayload;
}

export interface ContinuousPingInput {
  host: string;
  intervalMs: number;
  timeoutMs: number;
  payloadSize: number;
}

export interface LatencySample {
  taskId: string;
  timestampMs: number;
  address: string;
  status: string;
  rttMs?: number;
  seq: number;
}

export interface LatencyDone {
  taskId: string;
  sent: number;
  received: number;
  lost: number;
  minMs?: number;
  avgMs?: number;
  maxMs?: number;
  error?: ToolErrorPayload;
}

export interface TracerouteInput {
  target: string;
  maxHops: number;
  probes: number;
  timeoutMs: number;
  resolveNames: boolean;
}

export interface TracerouteHop {
  hop: number;
  address?: string;
  hostname?: string;
  rttsMs: Array<number | null>;
  status: string;
  reached: boolean;
}

export interface TracerouteDone {
  taskId: string;
  reached: boolean;
  error?: ToolErrorPayload;
}

export type ToolEvent =
  | { event: "pingProgress"; taskId: string; done: number; total: number }
  | { event: "pingResult"; taskId: string; result: PingHostResult }
  | {
      event: "pingDone";
      taskId: string;
      completed: number;
      failed: number;
      error?: ToolErrorPayload;
    }
  | { event: "tracerouteHop"; taskId: string; hop: TracerouteHop }
  | { event: "tracerouteDone"; taskId: string; reached: boolean; error?: ToolErrorPayload }
  | { event: "latencySample"; taskId: string; sample: LatencySample }
  | {
      event: "latencyDone";
      taskId: string;
      sent: number;
      received: number;
      lost: number;
      minMs?: number;
      avgMs?: number;
      maxMs?: number;
      error?: ToolErrorPayload;
    }
  | { event: "hashProgress"; taskId: string; done: number; total: number }
  | { event: "hashDone"; taskId: string; result?: HashResult; error?: ToolErrorPayload }
  | { event: "searchProgress"; taskId: string; files: number; matches: number }
  | { event: "searchHits"; taskId: string; hits: SearchHit[] }
  | {
      event: "searchDone";
      taskId: string;
      files: number;
      matches: number;
      skipped: number;
      truncated: boolean;
      error?: ToolErrorPayload;
    };

export interface ArpAdapter {
  name: string;
  description: string;
  ip: string;
  cidr: string;
  mac: string;
  npcapName?: string;
}

export interface ArpInterface {
  name: string;
  description: string;
}

export interface ArpCapabilities {
  native: boolean;
  npcapAvailable: boolean;
  elevated: boolean;
  reason?: string | null;
  reasonCode?: string | null;
  adapters: ArpAdapter[];
  npcapInterfaces: ArpInterface[];
}

export interface ArpScanInput {
  cidr: string;
  timeoutMs: number;
  concurrency: number;
  enhanced: boolean;
  adapterName?: string;
  interfaceName?: string;
}

export interface ArpEntry {
  ip: string;
  mac: string;
  interface: string;
  source: "native" | "npcap";
  replies: number;
  conflict: boolean;
}

export interface ArpScanResult {
  cidr: string;
  elapsedMs: number;
  enhanced: boolean;
  entries: ArpEntry[];
  conflicts: number;
}

export interface HashInput {
  path: string;
  algorithms: string[];
}

export interface HashValue {
  algorithm: string;
  digest: string;
}

export interface HashResult {
  path: string;
  size: number;
  elapsedMs: number;
  values: HashValue[];
}

export interface HashDone {
  taskId: string;
  result?: HashResult;
  error?: ToolErrorPayload;
}

export interface SearchInput {
  folder: string;
  query: string;
  mode: "literal" | "regex";
  caseSensitive: boolean;
  include: string;
  exclude: string;
  encoding: "auto" | "utf-8" | "utf-16le" | "utf-16be" | "gb18030";
  maxFileSize: number;
  maxResults: number;
  skipHidden: boolean;
}

export interface SearchHit {
  path: string;
  line: number;
  column: number;
  preview: string;
  matched: string;
}

export interface SearchDone {
  taskId: string;
  files: number;
  matches: number;
  skipped: number;
  truncated: boolean;
  error?: ToolErrorPayload;
}

export interface FileServerStartInput {
  root: string;
  host: string;
  port: number;
}

export interface FileServerFile {
  name: string;
  size: number;
  modifiedMs: number;
}

export interface FileServerRequest {
  timestampMs: number;
  remote: string;
  method: string;
  path: string;
  status: number;
  bytes: number;
}

export interface FileServerStatus {
  running: boolean;
  root: string | null;
  host: string | null;
  port: number | null;
  urls: string[];
  startedAt: number | null;
  uploads: number;
  downloads: number;
  files: FileServerFile[];
  requests: FileServerRequest[];
}

export type MqttProtocol = "3.1.1" | "5";

export interface MqttBrokerInput {
  brokerId: string;
  host: string;
  port: number;
  protocol: MqttProtocol;
  clientId: string;
  username: string;
  password?: string;
  tls: boolean;
  caPath?: string;
  keepAlive: number;
  cleanSession: boolean;
}

export type MqttStatus = "connecting" | "connected" | "reconnecting" | "disconnected" | "error";

export interface MqttState {
  brokerId: string;
  status: MqttStatus;
  detail: string | null;
  timestampMs: number;
}

export interface MqttMessage {
  brokerId: string;
  topic: string;
  payloadText: string;
  payloadHex: string;
  payloadBytes: number;
  payloadTruncated: boolean;
  qos: number;
  retain: boolean;
  timestampMs: number;
}

export interface MqttPublishInput {
  brokerId: string;
  topic: string;
  payload: string;
  payloadMode: "text" | "hex";
  qos: number;
  retain: boolean;
}

export interface MqttSubscribeInput {
  brokerId: string;
  topic: string;
  qos: number;
}
