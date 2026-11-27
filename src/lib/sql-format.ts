const CLAUSE_KEYWORDS = [
  "SELECT",
  "FROM",
  "WHERE",
  "GROUP BY",
  "ORDER BY",
  "HAVING",
  "LIMIT",
  "OFFSET",
  "UNION ALL",
  "UNION",
  "INSERT INTO",
  "VALUES",
  "UPDATE",
  "SET",
  "DELETE FROM",
  "RETURNING",
  "WITH",
];

const JOIN_KEYWORDS = [
  "LEFT OUTER JOIN",
  "RIGHT OUTER JOIN",
  "FULL OUTER JOIN",
  "LEFT JOIN",
  "RIGHT JOIN",
  "FULL JOIN",
  "INNER JOIN",
  "CROSS JOIN",
  "JOIN",
];

const ALL_KEYWORDS = [...CLAUSE_KEYWORDS, ...JOIN_KEYWORDS].sort(
  (left, right) => right.length - left.length,
);

interface Token {
  kind: "word" | "string" | "comment" | "punct" | "space";
  value: string;
}

function tokenize(sql: string): Token[] {
  const tokens: Token[] = [];
  let index = 0;
  while (index < sql.length) {
    const character = sql[index];
    if (/\s/.test(character)) {
      let value = "";
      while (index < sql.length && /\s/.test(sql[index])) {
        value += sql[index];
        index += 1;
      }
      tokens.push({ kind: "space", value });
      continue;
    }
    if (sql.startsWith("--", index)) {
      const end = sql.indexOf("\n", index);
      const value = end === -1 ? sql.slice(index) : sql.slice(index, end);
      tokens.push({ kind: "comment", value });
      index += value.length;
      continue;
    }
    if (sql.startsWith("/*", index)) {
      const end = sql.indexOf("*/", index + 2);
      const value = end === -1 ? sql.slice(index) : sql.slice(index, end + 2);
      tokens.push({ kind: "comment", value });
      index += value.length;
      continue;
    }
    if (character === "'" || character === '"' || character === "`") {
      let value = character;
      index += 1;
      while (index < sql.length) {
        const current = sql[index];
        value += current;
        index += 1;
        if (current === character) break;
        if (current === "\\" && index < sql.length) {
          value += sql[index];
          index += 1;
        }
      }
      tokens.push({ kind: "string", value });
      continue;
    }
    if (/[A-Za-z_]/.test(character)) {
      let value = "";
      while (index < sql.length && /[A-Za-z0-9_$.]/.test(sql[index])) {
        value += sql[index];
        index += 1;
      }
      tokens.push({ kind: "word", value });
      continue;
    }
    tokens.push({ kind: "punct", value: character });
    index += 1;
  }
  return tokens;
}

function matchClause(tokens: Token[], start: number): { clause: string; end: number } | null {
  const words: string[] = [];
  let cursor = start;
  while (cursor < tokens.length && words.length < 4) {
    const token = tokens[cursor];
    if (token.kind === "space") {
      cursor += 1;
      continue;
    }
    if (token.kind !== "word") break;
    words.push(token.value.toUpperCase());
    const combined = words.join(" ");
    if (!ALL_KEYWORDS.some((keyword) => keyword.startsWith(combined) && keyword !== combined)) {
      break;
    }
    cursor += 1;
  }
  const combined = words.join(" ");
  const clause = ALL_KEYWORDS.find(
    (keyword) => combined === keyword || combined.startsWith(`${keyword} `),
  );
  if (!clause) return null;
  const clauseWords = clause.split(" ").length;
  let end = start;
  let seen = 0;
  while (end < tokens.length && seen < clauseWords) {
    if (tokens[end].kind === "word") seen += 1;
    end += 1;
  }
  return { clause, end };
}

export function formatSql(sql: string, indentSize = 2): string {
  const tokens = tokenize(sql);
  const indent = " ".repeat(indentSize);
  const lines: string[] = [];
  let line = "";
  let depth = 0;
  let bodyIndent = 0;

  function pushLine(): void {
    const trimmed = line.trimEnd();
    if (trimmed.trim()) lines.push(trimmed);
    line = "";
  }

  function append(base: string): void {
    if (!line.trim()) {
      line = indent.repeat(depth + bodyIndent) + base;
      return;
    }
    if (!/[ =<>!+\-*/%(,.]$/.test(line)) line += " ";
    line += base;
  }

  let index = 0;
  while (index < tokens.length) {
    const token = tokens[index];
    if (token.kind === "space") {
      if (line && !line.endsWith(" ")) line += " ";
      index += 1;
      continue;
    }
    if (token.kind === "comment") {
      pushLine();
      lines.push(indent.repeat(depth) + token.value);
      index += 1;
      continue;
    }
    if (token.kind === "punct") {
      if (token.value === "(") {
        line = line.trimEnd() + "(";
        depth += 1;
        bodyIndent = 0;
      } else if (token.value === ")") {
        depth = Math.max(0, depth - 1);
        line = line.trimEnd() + ")";
      } else if (token.value === ",") {
        line = line.trimEnd() + ", ";
      } else if (token.value === ";") {
        line = line.trimEnd() + ";";
        pushLine();
      } else {
        line = line.trimEnd() + token.value;
      }
      index += 1;
      continue;
    }
    if (token.kind === "word") {
      const match = matchClause(tokens, index);
      if (match) {
        pushLine();
        bodyIndent =
          match.clause === "WHERE" || match.clause === "HAVING" || match.clause === "ON" ? 1 : 0;
        lines.push(indent.repeat(depth) + match.clause);
        index = match.end;
        continue;
      }
      const upper = token.value.toUpperCase();
      if (upper === "AND" || upper === "OR") {
        pushLine();
        line = indent.repeat(depth + 1) + upper + " ";
        index += 1;
        continue;
      }
      append(token.value);
      index += 1;
      continue;
    }
    append(token.value);
    index += 1;
  }
  pushLine();
  return lines.join("\n");
}
