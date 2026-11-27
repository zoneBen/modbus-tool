export type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" | "FATAL" | "UNKNOWN";

export interface LogLine {
  line: number;
  text: string;
  level: LogLevel;
  timestamp?: Date;
}

export interface LogAnalysis {
  lines: LogLine[];
  filtered: LogLine[];
  counts: Record<LogLevel, number>;
  firstTime?: Date;
  lastTime?: Date;
  timeline: Array<{ minute: string; count: number }>;
}

const LEVEL_PATTERNS: Array<[LogLevel, RegExp]> = [
  ["FATAL", /\b(FATAL|CRITICAL|CRIT|PANIC)\b/i],
  ["ERROR", /\b(ERROR|ERR|SEVERE)\b/i],
  ["WARN", /\b(WARN|WARNING)\b/i],
  ["INFO", /\b(INFO|INFORMATION|NOTICE)\b/i],
  ["DEBUG", /\b(DEBUG|TRACE|VERBOSE)\b/i],
];

const TIMESTAMP_PATTERNS = [
  /(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)/,
  /(\d{4}\/\d{2}\/\d{2} \d{2}:\d{2}:\d{2})/,
  /(\d{2}:\d{2}:\d{2}(?:\.\d+)?)/,
];

export function detectLevel(text: string): LogLevel {
  for (const [level, pattern] of LEVEL_PATTERNS) {
    if (pattern.test(text)) return level;
  }
  return "UNKNOWN";
}

export function detectTimestamp(text: string, base = new Date()): Date | undefined {
  for (const pattern of TIMESTAMP_PATTERNS) {
    const match = pattern.exec(text);
    if (!match) continue;
    const raw = match[1];
    if (/^\d{2}:\d{2}/.test(raw)) {
      const [hour, minute, second] = raw.split(":").map(Number);
      const date = new Date(base);
      date.setHours(hour, minute, Number.isFinite(second) ? second : 0, 0);
      return date;
    }
    const parsed = new Date(raw.replace(/\//g, "-").replace(" ", "T"));
    if (!Number.isNaN(parsed.getTime())) return parsed;
  }
  return undefined;
}

export interface AnalyzeOptions {
  query: string;
  level: LogLevel | "ALL";
  caseSensitive: boolean;
  limit?: number;
}

export function analyzeLogs(text: string, options: AnalyzeOptions): LogAnalysis {
  const lines: LogLine[] = [];
  text.split(/\r?\n/).forEach((value, index) => {
    if (!value.trim()) return;
    lines.push({
      line: index + 1,
      text: value,
      level: detectLevel(value),
      timestamp: detectTimestamp(value),
    });
  });

  const counts: Record<LogLevel, number> = {
    TRACE: 0,
    DEBUG: 0,
    INFO: 0,
    WARN: 0,
    ERROR: 0,
    FATAL: 0,
    UNKNOWN: 0,
  };
  for (const line of lines) counts[line.level] += 1;

  const needle = options.query.trim();
  const haystackNeedle = options.caseSensitive ? needle : needle.toLowerCase();
  const filtered = lines.filter((line) => {
    if (options.level !== "ALL" && line.level !== options.level) return false;
    if (!needle) return true;
    const value = options.caseSensitive ? line.text : line.text.toLowerCase();
    return value.includes(haystackNeedle);
  });
  const limited = options.limit ? filtered.slice(0, options.limit) : filtered;

  const timed = lines.filter((line) => line.timestamp);
  const firstTime = timed[0]?.timestamp;
  const lastTime = timed[timed.length - 1]?.timestamp;

  const buckets = new Map<string, number>();
  for (const line of timed) {
    if (!line.timestamp) continue;
    const key = `${line.timestamp.toISOString().slice(0, 16).replace("T", " ")}`;
    buckets.set(key, (buckets.get(key) ?? 0) + 1);
  }
  const timeline = Array.from(buckets.entries())
    .map(([minute, count]) => ({ minute, count }))
    .sort((left, right) => left.minute.localeCompare(right.minute))
    .slice(-40);

  return { lines, filtered: limited, counts, firstTime, lastTime, timeline };
}
