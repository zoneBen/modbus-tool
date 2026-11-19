import { describe, expect, it } from "vitest";
import { MAX_SCAN_PORTS, formatPortList, parsePortList } from "./port-list";

describe("port list parsing", () => {
  it("parses single ports, ranges, duplicates and separators", () => {
    expect(parsePortList("502")).toEqual({ ports: [502], error: null });
    expect(parsePortList("80, 443;8000-8003 80")).toEqual({
      ports: [80, 443, 8000, 8001, 8002, 8003],
      error: null,
    });
    expect(parsePortList("1:3")).toEqual({ ports: [1, 2, 3], error: null });
  });

  it("rejects empty, malformed and reversed ranges", () => {
    expect(parsePortList("  ").error).toBe("empty");
    expect(parsePortList("0").error).toBe("invalid");
    expect(parsePortList("65536").error).toBe("invalid");
    expect(parsePortList("abc").error).toBe("invalid");
    expect(parsePortList("90-80").error).toBe("invalid");
    expect(parsePortList("80-").error).toBe("invalid");
  });

  it("caps the number of selected ports", () => {
    expect(parsePortList(`1-${MAX_SCAN_PORTS}`)).toEqual({
      ports: Array.from({ length: MAX_SCAN_PORTS }, (_, index) => index + 1),
      error: null,
    });
    expect(parsePortList(`1-${MAX_SCAN_PORTS + 1}`).error).toBe("tooMany");
  });

  it("formats a stable summary", () => {
    expect(formatPortList([8080, 80, 8080, 443])).toBe("80, 443, 8080");
  });
});
