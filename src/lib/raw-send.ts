import { i18n } from "@/i18n";
import { formatHexBytes } from "@/lib/format";
import type { RawSendMode } from "@/types/models";

export const MAX_RAW_SEND_BYTES = 4096;

const HEX_SEPARATORS = /[\s,_:-]+/g;

export interface RawSendSuffix {
  appendCr?: boolean;
  appendLf?: boolean;
}

export function encodeRawPayload(
  data: string,
  mode: RawSendMode,
  suffix: RawSendSuffix = {},
): Uint8Array {
  const base = mode === "ascii" ? new TextEncoder().encode(data) : decodeHex(data);
  const hasSuffix = Boolean(suffix.appendCr || suffix.appendLf);
  if (!base.length && !hasSuffix) {
    throw new Error(
      mode === "hex" ? i18n.global.t("errors.rawEmptyHex") : i18n.global.t("errors.rawEmptyAscii"),
    );
  }
  const extra = (suffix.appendCr ? 1 : 0) + (suffix.appendLf ? 1 : 0);
  const bytes = new Uint8Array(base.length + extra);
  bytes.set(base);
  let offset = base.length;
  if (suffix.appendCr) bytes[offset++] = 0x0d;
  if (suffix.appendLf) bytes[offset] = 0x0a;
  if (bytes.length > MAX_RAW_SEND_BYTES) {
    throw new Error(
      i18n.global.t("errors.rawTooLarge", {
        max: MAX_RAW_SEND_BYTES,
        actual: bytes.length,
      }),
    );
  }
  return bytes;
}

export function rawByteLength(
  data: string,
  mode: RawSendMode,
  suffix: RawSendSuffix = {},
): number | null {
  try {
    return encodeRawPayload(data, mode, suffix).length;
  } catch {
    return null;
  }
}

export function formatDebugBytes(data: readonly number[], mode: "ascii" | "hex"): string {
  if (mode === "hex") return formatHexBytes(data);
  let text = "";
  for (const byte of data) {
    if (byte === 0x0a) text += "\n";
    else if (byte === 0x0d) continue;
    else if (byte === 0x09) text += "  ";
    else if (byte >= 0x20 && byte <= 0x7e) text += String.fromCharCode(byte);
    else text += "·";
  }
  return text;
}

function decodeHex(data: string): Uint8Array {
  const cleaned = data.replace(/0x/gi, "").replace(HEX_SEPARATORS, "");
  if (!cleaned) return new Uint8Array();
  if (!/^[0-9a-f]+$/i.test(cleaned)) {
    throw new Error(i18n.global.t("errors.rawHexInvalid"));
  }
  if (cleaned.length % 2 !== 0) {
    throw new Error(i18n.global.t("errors.rawHexOdd"));
  }
  const bytes = new Uint8Array(cleaned.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(cleaned.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
