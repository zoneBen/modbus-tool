import { describe, expect, it } from "vitest";
import { parseCandidates } from "./decode-preview";
import type { RegisterValue } from "@/types/models";

const values: RegisterValue[] = [
  {
    address: 0,
    words: [0x3f80, 0x0000, 0x0000, 0x0000],
    formatted: "—",
    numeric: null,
  },
];

describe("parse candidates", () => {
  it("offers multiple type/order interpretations for one register", () => {
    const candidates = parseCandidates(values, 0);
    expect(candidates.some((item) => item.label === "UINT16" && item.value === "16256")).toBe(true);
    const floats = candidates.filter((item) => item.label === "FLOAT32");
    expect(floats.some((item) => item.byteOrder === "abcd" && item.value === "1")).toBe(true);
    expect(new Set(floats.map((item) => item.value)).size).toBeGreaterThan(1);
  });

  it("returns nothing when the address is outside the snapshot", () => {
    expect(parseCandidates(values, 20)).toEqual([]);
  });
});
