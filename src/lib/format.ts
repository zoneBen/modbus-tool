const numberFormatter = new Intl.NumberFormat("en-US");

export function formatAddress(address: number, base: 0 | 1): string {
  const display = address + base;
  return display.toString().padStart(5, "0");
}

export function formatRange(start: number, quantity: number, base: 0 | 1): string {
  const first = formatAddress(start, base);
  const last = formatAddress(start + Math.max(quantity - 1, 0), base);
  return first === last ? first : `${first} – ${last}`;
}

export function formatWord(word: number): string {
  return `0x${(word & 0xffff).toString(16).toUpperCase().padStart(4, "0")}`;
}

export function formatNumber(value: number): string {
  return numberFormatter.format(value);
}

export function formatBinary(word: number): string {
  return (word & 0xffff).toString(2).padStart(16, "0");
}

export function formatHexBytes(bytes: readonly number[], uppercase = true): string {
  return bytes
    .map((byte) => {
      const text = byte.toString(16).padStart(2, "0");
      return uppercase ? text.toUpperCase() : text;
    })
    .join(" ");
}

export function formatAsciiBytes(bytes: readonly number[]): string {
  return bytes
    .map((byte) => (byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : "·"))
    .join("");
}

export function formatLatency(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  if (ms < 10) return `${ms.toFixed(1)} ms`;
  return `${Math.round(ms)} ms`;
}

export function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  const hh = date.getHours().toString().padStart(2, "0");
  const mm = date.getMinutes().toString().padStart(2, "0");
  const ss = date.getSeconds().toString().padStart(2, "0");
  const ms = date.getMilliseconds().toString().padStart(3, "0");
  return `${hh}:${mm}:${ss}.${ms}`;
}

export function formatClock(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleTimeString("zh-CN", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function formatInterval(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(ms % 1000 === 0 ? 0 : 1)} s`;
}

export function formatFunctionCode(code: number): string {
  return `FC${code.toString().padStart(2, "0")}`;
}

export function formatByteCount(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export function snapshotKey(sessionId: string, definitionId: string): string {
  return `${sessionId}:${definitionId}`;
}

export function historyKey(definitionId: string, address: number): string {
  return `${definitionId}:${address}`;
}
