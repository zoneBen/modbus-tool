export interface EnvEntry {
  key: string;
  rawValue: string;
  value: string;
  line: number;
  comment: boolean;
  error?: string;
}

export interface EnvParseResult {
  entries: EnvEntry[];
  values: Record<string, string>;
  unresolved: string[];
}

function unquote(value: string): { value: string; quote: '"' | "'" | null } {
  const trimmed = value.trim();
  if (trimmed.length >= 2 && trimmed[0] === trimmed[trimmed.length - 1]) {
    if (trimmed[0] === '"' || trimmed[0] === "'") {
      return { value: trimmed.slice(1, -1), quote: trimmed[0] };
    }
  }
  return { value: trimmed, quote: null };
}

export function parseEnv(text: string): EnvParseResult {
  const entries: EnvEntry[] = [];
  const values: Record<string, string> = {};
  const unresolved: string[] = [];

  text.split(/\r?\n/).forEach((line, index) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    if (trimmed.startsWith("#")) {
      entries.push({
        key: trimmed,
        rawValue: "",
        value: "",
        line: index + 1,
        comment: true,
      });
      return;
    }
    const withoutExport = trimmed.startsWith("export ") ? trimmed.slice(7).trim() : trimmed;
    const separator = withoutExport.indexOf("=");
    if (separator <= 0) {
      entries.push({
        key: withoutExport,
        rawValue: "",
        value: "",
        line: index + 1,
        comment: false,
        error: "envMissingEquals",
      });
      return;
    }
    const key = withoutExport.slice(0, separator).trim();
    const rawValue = withoutExport.slice(separator + 1);
    const { value: unquoted, quote } = unquote(rawValue);
    let value = unquoted;
    if (quote !== "'") {
      value = value.replace(
        /\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)/g,
        (match, braced, bare) => {
          const name = String(braced ?? bare);
          if (name in values) return values[name];
          if (!unresolved.includes(name)) unresolved.push(name);
          return match;
        },
      );
    }
    values[key] = value;
    entries.push({ key, rawValue, value, line: index + 1, comment: false });
  });

  return { entries, values, unresolved };
}

export function resolvedEnvText(result: EnvParseResult): string {
  return result.entries
    .filter((entry) => !entry.comment && !entry.error)
    .map((entry) => `${entry.key}=${entry.value}`)
    .join("\n");
}
