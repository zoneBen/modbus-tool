import { describe, expect, it } from "vitest";
import { nextRuns, parseCron, validateCron } from "./cron";

describe("cron parser", () => {
  it("parses steps and ranges", () => {
    const parsed = parseCron("*/15 8-10 * * 1-5");
    expect(parsed.minutes.values.has(0)).toBe(true);
    expect(parsed.minutes.values.has(15)).toBe(true);
    expect(parsed.hours.values.has(8)).toBe(true);
    expect(parsed.hours.values.has(11)).toBe(false);
    expect(parsed.weekdays.values.has(5)).toBe(true);
  });

  it("expands a start value with a step to the maximum", () => {
    expect([...parseCron("5/10 * * * *").minutes.values]).toEqual([5, 15, 25, 35, 45, 55]);
  });

  it("normalizes Sunday 7 to 0", () => {
    expect(parseCron("0 0 * * 7").weekdays.values.has(0)).toBe(true);
  });

  it("returns the next runs", () => {
    const runs = nextRuns("*/10 * * * *", 3, new Date("2026-01-01T00:00:00"));
    expect(runs.map((run) => run.getMinutes())).toEqual([10, 20, 30]);
  });

  it("rejects malformed expressions", () => {
    expect(validateCron("* * *")).toBe("cronFieldCount");
    expect(validateCron("61 * * * *")).toBe("cronOutOfRange");
    expect(validateCron("*/0 * * * *")).toBe("cronInvalidStep");
    expect(validateCron("*/5 * * * *")).toBeNull();
  });
});
