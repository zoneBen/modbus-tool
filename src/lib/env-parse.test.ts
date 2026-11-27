import { describe, expect, it } from "vitest";
import { parseEnv, resolvedEnvText } from "./env-parse";

describe("env parser", () => {
  it("parses variables and interpolates earlier values", () => {
    const result = parseEnv("HOST=127.0.0.1\nPORT=5432\nURL=postgres://${HOST}:${PORT}/app\n");
    expect(result.values.HOST).toBe("127.0.0.1");
    expect(result.values.URL).toBe("postgres://127.0.0.1:5432/app");
    expect(result.unresolved).toEqual([]);
  });

  it("supports export, quotes and comments", () => {
    const result = parseEnv("# note\nexport NAME=\"Modbus Tool\"\nSINGLE='$NAME'\n");
    expect(result.values.NAME).toBe("Modbus Tool");
    expect(result.values.SINGLE).toBe("$NAME");
    expect(result.entries[0].comment).toBe(true);
  });

  it("reports unresolved and malformed lines", () => {
    const result = parseEnv("URL=$MISSING\nBROKEN\n");
    expect(result.unresolved).toContain("MISSING");
    expect(result.entries.find((entry) => entry.key === "BROKEN")?.error).toBe("envMissingEquals");
  });

  it("builds resolved text without comments", () => {
    const result = parseEnv("# c\nA=1\nB=${A}2\n");
    expect(resolvedEnvText(result)).toBe("A=1\nB=12");
  });
});
