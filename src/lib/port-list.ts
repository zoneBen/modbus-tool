export const MAX_SCAN_PORTS = 256;
export const MAX_SCAN_PROBES = 65_536;

export type PortListError = "empty" | "invalid" | "tooMany";

export interface PortListResult {
  ports: number[];
  error: PortListError | null;
}

const TOKEN_SEPARATOR = /[\s,;]+/;
const RANGE_SEPARATOR = /^(\d+)\s*[-:]\s*(\d+)$/;

function parsePort(value: string): number | null {
  if (!/^\d+$/.test(value)) return null;
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65_535) return null;
  return parsed;
}

export function parsePortList(text: string): PortListResult {
  const tokens = text.trim().split(TOKEN_SEPARATOR).filter(Boolean);
  if (!tokens.length) return { ports: [], error: "empty" };

  const ports = new Set<number>();
  for (const token of tokens) {
    const range = RANGE_SEPARATOR.exec(token);
    if (range) {
      const start = parsePort(range[1] ?? "");
      const end = parsePort(range[2] ?? "");
      if (start === null || end === null || start > end) {
        return { ports: [], error: "invalid" };
      }
      for (let port = start; port <= end; port += 1) {
        ports.add(port);
        if (ports.size > MAX_SCAN_PORTS) return { ports: [], error: "tooMany" };
      }
      continue;
    }
    const port = parsePort(token);
    if (port === null) return { ports: [], error: "invalid" };
    ports.add(port);
    if (ports.size > MAX_SCAN_PORTS) return { ports: [], error: "tooMany" };
  }

  return { ports: [...ports].sort((left, right) => left - right), error: null };
}

export function formatPortList(ports: readonly number[]): string {
  return [...new Set(ports)].sort((left, right) => left - right).join(", ");
}
