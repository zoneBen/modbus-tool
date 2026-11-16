import { describe, expect, it } from "vitest";
import {
  formatAddress,
  formatBinary,
  formatHexBytes,
  formatInterval,
  formatLatency,
  formatRange,
  formatWord,
  historyKey,
  snapshotKey,
} from "./format";

describe("format helpers", () => {
  it("formats addresses with the configured base", () => {
    expect(formatAddress(0, 0)).toBe("00000");
    expect(formatAddress(0, 1)).toBe("00001");
    expect(formatAddress(1234, 0)).toBe("01234");
    expect(formatAddress(65535, 1)).toBe("65536");
  });

  it("formats address ranges", () => {
    expect(formatRange(0, 1, 1)).toBe("00001");
    expect(formatRange(0, 16, 1)).toBe("00001 – 00016");
    expect(formatRange(99, 8, 0)).toBe("00099 – 00106");
  });

  it("formats words, binary and byte sequences", () => {
    expect(formatWord(0x1a2b)).toBe("0x1A2B");
    expect(formatBinary(0x00f0)).toBe("0000000011110000");
    expect(formatHexBytes([0x01, 0xab, 0x00])).toBe("01 AB 00");
  });

  it("formats latency and intervals for the status bar", () => {
    expect(formatLatency(8.42)).toBe("8.4 ms");
    expect(formatLatency(21.7)).toBe("22 ms");
    expect(formatLatency(Number.NaN)).toBe("—");
    expect(formatInterval(760)).toBe("760 ms");
    expect(formatInterval(2000)).toBe("2 s");
    expect(formatInterval(1500)).toBe("1.5 s");
  });

  it("builds stable keys for snapshots and history", () => {
    expect(snapshotKey("plc-01", "d-main")).toBe("plc-01:d-main");
    expect(historyKey("d-main", 100)).toBe("d-main:100");
  });
});
