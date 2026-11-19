import { describe, expect, it } from "vitest";
import { describeBase, epochToMs, parseDateText, parseEpoch, parseInteger } from "./converters";

describe("timestamp conversion", () => {
  it("detects epoch units and produces milliseconds", () => {
    const expected = 1_700_000_000_000;
    expect(parseEpoch("1700000000", "auto")).toEqual({
      ok: true,
      value: { ms: expected, unit: "s" },
    });
    expect(epochToMs("1700000000000", "ms")).toBe(expected);
    expect(epochToMs("1700000000000000", "us")).toBe(expected);
    expect(epochToMs("1700000000000000000", "ns")).toBe(expected);
  });

  it("rejects empty and malformed epochs", () => {
    expect(parseEpoch("", "auto")).toEqual({ ok: false, error: "empty" });
    expect(parseEpoch("12x", "auto")).toEqual({ ok: false, error: "invalid" });
  });

  it("parses ISO date text", () => {
    expect(parseDateText("2023-11-14T22:13:20.000Z")).toEqual({
      ok: true,
      value: 1_700_000_000_000,
    });
    expect(parseDateText("not-a-date").ok).toBe(false);
  });
});

describe("base conversion", () => {
  it("parses integers with radix-specific digits", () => {
    expect(parseInteger("FF", 16)).toEqual({ ok: true, value: 255n });
    expect(parseInteger("0xFF", 16)).toEqual({ ok: true, value: 255n });
    expect(parseInteger("-1010", 2)).toEqual({ ok: true, value: -10n });
    expect(parseInteger("8", 8).ok).toBe(false);
    expect(parseInteger("", 10)).toEqual({ ok: false, error: "empty" });
  });

  it("describes width-specific signed and unsigned values", () => {
    const outputs = describeBase(255n);
    expect(outputs.hex).toBe("FF");
    expect(outputs.binary).toBe("1111 1111");
    expect(outputs.unsigned8).toBe("255");
    expect(outputs.signed8).toBe("-1");

    const max32 = describeBase(0xffff_ffffn);
    expect(max32.unsigned32).toBe("4294967295");
    expect(max32.signed32).toBe("-1");
  });

  it("decodes IEEE-754 bit patterns", () => {
    const one32 = describeBase(0x3f80_0000n);
    expect(Number(one32.float32)).toBe(1);
    const one64 = describeBase(0x3ff0_0000_0000_0000n);
    expect(Number(one64.float64)).toBe(1);
  });
});
