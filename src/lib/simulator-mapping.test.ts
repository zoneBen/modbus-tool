import { describe, expect, it } from "vitest";
import {
  mappingRange,
  mappingRangeValid,
  overlappingPoint,
  pointEnd,
} from "@/lib/simulator-mapping";
import type { RegisterPoint } from "@/types/models";

function point(address: number, dataType: RegisterPoint["dataType"] = "u16"): RegisterPoint {
  return {
    address,
    dataType,
    byteOrder: "abcd",
    meaning: "",
    unit: "",
    scale: 1,
    offset: 0,
    alerts: [],
  };
}

describe("simulator mapping helpers", () => {
  it("treats an empty mapping as an empty range", () => {
    expect(mappingRange([])).toEqual({ startAddress: 0, quantity: 0 });
    expect(mappingRangeValid([])).toBe(true);
  });

  it("derives a sparse address range from the configured points", () => {
    const range = mappingRange([point(1000), point(1002, "f32")]);
    expect(range).toEqual({ startAddress: 1000, quantity: 4 });
    expect(pointEnd(point(1002, "f32"))).toBe(1004);
  });

  it("detects overlapping register spans", () => {
    const existing = point(10, "f32");
    expect(overlappingPoint([existing], 11, 1)).toBe(existing);
    expect(overlappingPoint([existing], 12, 1)).toBeNull();
    expect(overlappingPoint([existing], 10, 2, 10)).toBeNull();
  });

  it("rejects mappings wider than the 16-bit quantity field", () => {
    expect(mappingRangeValid([point(0), point(65535)])).toBe(false);
    expect(mappingRangeValid([point(65535)])).toBe(true);
    expect(mappingRangeValid([point(65535, "f32")])).toBe(false);
  });
});
