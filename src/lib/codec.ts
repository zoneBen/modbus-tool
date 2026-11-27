function bytesToBinary(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return binary;
}

function binaryToBytes(binary: string): Uint8Array {
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index) & 0xff;
  }
  return bytes;
}

export function encodeBase64(text: string, urlSafe = false): string {
  const encoded = btoa(bytesToBinary(new TextEncoder().encode(text)));
  return urlSafe ? encoded.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "") : encoded;
}

export function decodeBase64(value: string): string {
  const normalized = value.trim().replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  const binary = atob(padded);
  return new TextDecoder("utf-8").decode(binaryToBytes(binary));
}

export function encodeUrl(value: string, component = true): string {
  return component ? encodeURIComponent(value) : encodeURI(value);
}

export function decodeUrl(value: string): string {
  return decodeURIComponent(value);
}

export function textToHex(text: string, separator = " "): string {
  return Array.from(new TextEncoder().encode(text), (byte) =>
    byte.toString(16).padStart(2, "0").toUpperCase(),
  ).join(separator);
}

export function hexToText(hex: string): string {
  const cleaned = hex.replace(/0x/gi, "").replace(/[^0-9a-f]/gi, "");
  if (cleaned.length % 2 !== 0) throw new Error("hexOddLength");
  const bytes = new Uint8Array(cleaned.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(cleaned.slice(index * 2, index * 2 + 2), 16);
  }
  return new TextDecoder("utf-8").decode(bytes);
}

export function base64ToHex(value: string): string {
  const normalized = value.trim().replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  return Array.from(binaryToBytes(atob(padded)), (byte) =>
    byte.toString(16).padStart(2, "0").toUpperCase(),
  ).join(" ");
}

export function hexToBase64(value: string): string {
  const cleaned = value.replace(/0x/gi, "").replace(/[^0-9a-f]/gi, "");
  if (cleaned.length % 2 !== 0) throw new Error("hexOddLength");
  const bytes = new Uint8Array(cleaned.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(cleaned.slice(index * 2, index * 2 + 2), 16);
  }
  return btoa(bytesToBinary(bytes));
}

export function escapeForDisplay(text: string): string {
  return text.replace(/\\/g, "\\\\").replace(/\n/g, "\\n").replace(/\r/g, "\\r");
}
