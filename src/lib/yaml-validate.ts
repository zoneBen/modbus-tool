import { parseAllDocuments } from "yaml";

export interface YamlIssue {
  line: number;
  column: number;
  message: string;
  severity: "error" | "warning";
}

export interface YamlValidationResult {
  valid: boolean;
  documents: number;
  errors: YamlIssue[];
  warnings: YamlIssue[];
}

function offsetToLineColumn(text: string, offset: number): { line: number; column: number } {
  const safeOffset = Math.max(0, Math.min(offset, text.length));
  const before = text.slice(0, safeOffset);
  const lines = before.split("\n");
  return { line: lines.length, column: (lines[lines.length - 1]?.length ?? 0) + 1 };
}

function issueFromError(
  text: string,
  error: {
    message: string;
    linePos?: Array<{ line: number; col: number }> | null;
    pos?: [number, number];
  },
  severity: "error" | "warning",
): YamlIssue {
  const linePos = error.linePos?.[0];
  const fallback = offsetToLineColumn(text, error.pos?.[0] ?? 0);
  return {
    line: linePos?.line ?? fallback.line,
    column: linePos?.col ?? fallback.column,
    message: error.message.replace(/\s+at line \d+.*$/s, "").trim(),
    severity,
  };
}

export function validateYaml(text: string): YamlValidationResult {
  if (!text.trim()) {
    return {
      valid: false,
      documents: 0,
      errors: [{ line: 1, column: 1, message: "empty", severity: "error" }],
      warnings: [],
    };
  }
  const documents = parseAllDocuments(text, { prettyErrors: false });
  const errors: YamlIssue[] = [];
  const warnings: YamlIssue[] = [];
  for (const document of documents) {
    for (const error of document.errors) {
      errors.push(issueFromError(text, error, "error"));
    }
    for (const warning of document.warnings) {
      warnings.push(issueFromError(text, warning, "warning"));
    }
  }
  errors.sort((a, b) => a.line - b.line || a.column - b.column);
  warnings.sort((a, b) => a.line - b.line || a.column - b.column);
  return {
    valid: errors.length === 0,
    documents: documents.length,
    errors,
    warnings,
  };
}
