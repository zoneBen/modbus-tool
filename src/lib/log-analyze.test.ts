import { describe, expect, it } from "vitest";
import { analyzeLogs, detectLevel } from "./log-analyze";

const LOG = `2026-01-01T10:00:01Z INFO boot complete
2026-01-01T10:00:05Z WARN sensor slow
2026-01-01T10:01:00Z ERROR read timeout
2026-01-01T10:01:02Z ERROR read timeout
plain line without level
`;

describe("log analyzer", () => {
  it("detects common levels", () => {
    expect(detectLevel("ERROR: boom")).toBe("ERROR");
    expect(detectLevel("[WARNING] slow")).toBe("WARN");
    expect(detectLevel("nothing here")).toBe("UNKNOWN");
  });

  it("counts and filters lines", () => {
    const result = analyzeLogs(LOG, { query: "timeout", level: "ALL", caseSensitive: false });
    expect(result.counts.ERROR).toBe(2);
    expect(result.counts.INFO).toBe(1);
    expect(result.filtered).toHaveLength(2);
  });

  it("filters by level", () => {
    const result = analyzeLogs(LOG, { query: "", level: "WARN", caseSensitive: false });
    expect(result.filtered).toHaveLength(1);
    expect(result.filtered[0].level).toBe("WARN");
  });

  it("builds a minute timeline", () => {
    const result = analyzeLogs(LOG, { query: "", level: "ALL", caseSensitive: false });
    expect(result.timeline).toHaveLength(2);
    expect(result.timeline[0].count).toBe(2);
  });
});
