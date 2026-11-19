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

export interface HttpHeaderInput {
  name: string;
  value: string;
}

export interface HttpRequestInput {
  method: HttpMethod;
  url: string;
  headers: HttpHeaderInput[];
  body?: string;
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
  | { event: "tracerouteDone"; taskId: string; reached: boolean; error?: ToolErrorPayload };

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
  reason?: string;
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
