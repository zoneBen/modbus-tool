export type EpochUnit = "auto" | "s" | "ms" | "us" | "ns";

export interface EpochValue {
  ms: number;
  unit: Exclude<EpochUnit, "auto">;
}

export type ParseResult<T> = { ok: true; value: T } | { ok: false; error: "empty" | "invalid" };

export function parseEpoch(text: string, unit: EpochUnit = "auto"): ParseResult<EpochValue> {
  const trimmed = text.trim();
  if (!trimmed) return { ok: false, error: "empty" };
  if (!/^-?\d+$/.test(trimmed)) return { ok: false, error: "invalid" };
  let epoch: bigint;
  try {
    epoch = BigInt(trimmed);
  } catch {
    return { ok: false, error: "invalid" };
  }
  const resolvedUnit = unit === "auto" ? detectEpochUnit(trimmed) : unit;
  const ms = Number(epochToMilliseconds(epoch, resolvedUnit));
  if (!Number.isFinite(ms) || Math.abs(ms) > 8.64e15) {
    return { ok: false, error: "invalid" };
  }
  return { ok: true, value: { ms, unit: resolvedUnit } };
}

function detectEpochUnit(text: string): Exclude<EpochUnit, "auto"> {
  const digits = text.replace("-", "").length;
  if (digits <= 10) return "s";
  if (digits <= 13) return "ms";
  if (digits <= 16) return "us";
  return "ns";
}

function epochToMilliseconds(epoch: bigint, unit: Exclude<EpochUnit, "auto">): bigint {
  switch (unit) {
    case "s":
      return epoch * 1000n;
    case "ms":
      return epoch;
    case "us":
      return epoch / 1000n;
    case "ns":
      return epoch / 1_000_000n;
  }
}

export function epochToMs(text: string, unit: EpochUnit): number | null {
  const trimmed = text.trim();
  if (!trimmed || !/^-?\d+$/.test(trimmed)) return null;
  const epoch = BigInt(trimmed);
  const resolved = unit === "auto" ? detectEpochUnit(trimmed) : unit;
  return Number(epochToMilliseconds(epoch, resolved));
}

export interface EpochOutputs {
  local: string;
  utc: string;
  iso: string;
  seconds: string;
  milliseconds: string;
  weekday: string;
}

export function describeEpoch(ms: number): EpochOutputs {
  const date = new Date(ms);
  return {
    local: formatLocal(date),
    utc: formatUtc(date),
    iso: date.toISOString(),
    seconds: String(Math.trunc(ms / 1000)),
    milliseconds: String(ms),
    weekday: new Intl.DateTimeFormat(undefined, { weekday: "long" }).format(date),
  };
}

export function parseDateText(text: string): ParseResult<number> {
  const trimmed = text.trim();
  if (!trimmed) return { ok: false, error: "empty" };
  const ms = Date.parse(trimmed);
  if (!Number.isFinite(ms)) return { ok: false, error: "invalid" };
  return { ok: true, value: ms };
}

function formatLocal(date: Date): string {
  const pad = (value: number, width = 2) => String(value).padStart(width, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`;
}

function formatUtc(date: Date): string {
  const pad = (value: number, width = 2) => String(value).padStart(width, "0");
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(
    date.getUTCHours(),
  )}:${pad(date.getUTCMinutes())}:${pad(date.getUTCSeconds())}.${pad(
    date.getUTCMilliseconds(),
    3,
  )}`;
}

export type BaseRadix = 2 | 8 | 10 | 16;

export interface BaseOutputs {
  binary: string;
  octal: string;
  decimal: string;
  hex: string;
  unsigned8: string;
  signed8: string;
  unsigned16: string;
  signed16: string;
  unsigned32: string;
  signed32: string;
  unsigned64: string;
  signed64: string;
  float32: string;
  float64: string;
}

export function parseInteger(text: string, radix: BaseRadix): ParseResult<bigint> {
  const trimmed = text.trim().replace(/[\s_]/g, "");
  if (!trimmed) return { ok: false, error: "empty" };
  let normalized = trimmed;
  if (normalized.startsWith("-0x")) normalized = `-${normalized.slice(3)}`;
  else if (normalized.startsWith("+0x")) normalized = normalized.slice(3);
  else if (normalized.startsWith("0x")) normalized = normalized.slice(2);
  if (radix === 16 && /^[0-9a-f]+$/i.test(normalized)) {
    return { ok: true, value: BigInt(`0x${normalized}`) };
  }
  const sign = normalized.startsWith("-") ? -1n : 1n;
  const digits = normalized.replace(/^[+-]/, "");
  if (!digits) return { ok: false, error: "invalid" };
  const pattern =
    radix === 2 ? /^[01]+$/ : radix === 8 ? /^[0-7]+$/ : radix === 10 ? /^\d+$/ : /^[0-9a-f]+$/i;
  if (!pattern.test(digits)) return { ok: false, error: "invalid" };
  let value = 0n;
  const base = BigInt(radix);
  for (const character of digits.toLowerCase()) {
    const digit = BigInt(Number.parseInt(character, 16));
    if (digit >= base) return { ok: false, error: "invalid" };
    value = value * base + digit;
  }
  return { ok: true, value: sign * value };
}

export function describeBase(value: bigint): BaseOutputs {
  const unsigned64 = BigInt.asUintN(64, value);
  const unsigned32 = BigInt.asUintN(32, value);
  const unsigned16 = BigInt.asUintN(16, value);
  const unsigned8 = BigInt.asUintN(8, value);
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setBigUint64(0, unsigned64, false);
  const float64 = view.getFloat64(0, false);
  view.setUint32(0, Number(unsigned32), false);
  const float32 = view.getFloat32(0, false);
  return {
    binary: formatInteger(value, 2, 4),
    octal: formatInteger(value, 8, 3),
    decimal: value.toString(10),
    hex: formatInteger(value, 16, 4).toUpperCase(),
    unsigned8: unsigned8.toString(10),
    signed8: BigInt.asIntN(8, unsigned8).toString(10),
    unsigned16: unsigned16.toString(10),
    signed16: BigInt.asIntN(16, unsigned16).toString(10),
    unsigned32: unsigned32.toString(10),
    signed32: BigInt.asIntN(32, unsigned32).toString(10),
    unsigned64: unsigned64.toString(10),
    signed64: BigInt.asIntN(64, unsigned64).toString(10),
    float32: Number.isFinite(float32) ? String(float32) : String(float32),
    float64: Number.isFinite(float64) ? String(float64) : String(float64),
  };
}

export function formatInteger(value: bigint, radix: BaseRadix, group = 0): string {
  const negative = value < 0n;
  const digits = (negative ? -value : value).toString(radix);
  const grouped =
    group > 0 ? digits.replace(new RegExp(`\\B(?=(.{${group}})+$)`, "g"), " ") : digits;
  return negative ? `-${grouped}` : grouped;
}
