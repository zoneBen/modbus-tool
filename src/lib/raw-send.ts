import type { RawSendMode } from "@/types/models";

export const MAX_RAW_SEND_BYTES = 4096;

const HEX_SEPARATORS = /[\s,_:-]+/g;

export function encodeRawPayload(data: string, mode: RawSendMode): Uint8Array {
  const bytes = mode === "ascii" ? new TextEncoder().encode(data) : decodeHex(data);
  if (!bytes.length) throw new Error("发送内容为空");
  if (bytes.length > MAX_RAW_SEND_BYTES) {
    throw new Error(`单次发送不能超过 ${MAX_RAW_SEND_BYTES} 字节，当前 ${bytes.length} 字节`);
  }
  return bytes;
}

export function rawByteLength(data: string, mode: RawSendMode): number | null {
  try {
    return encodeRawPayload(data, mode).length;
  } catch {
    return null;
  }
}

function decodeHex(data: string): Uint8Array {
  const cleaned = data.replace(/0x/gi, "").replace(HEX_SEPARATORS, "");
  if (!cleaned) throw new Error("HEX 内容为空");
  if (!/^[0-9a-f]+$/i.test(cleaned)) {
    throw new Error("HEX 包含非法字符，仅支持 0–9 / A–F");
  }
  if (cleaned.length % 2 !== 0) {
    throw new Error("HEX 内容需为偶数个十六进制字符");
  }
  const bytes = new Uint8Array(cleaned.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(cleaned.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
