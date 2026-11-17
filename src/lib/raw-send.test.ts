import { describe, expect, it } from "vitest";
import { MAX_RAW_SEND_BYTES, encodeRawPayload, formatDebugBytes, rawByteLength } from "./raw-send";

describe("manual raw send encoding", () => {
  it("sends ASCII text verbatim", () => {
    expect(Array.from(encodeRawPayload("AT+INFO\r\n", "ascii"))).toEqual([
      0x41, 0x54, 0x2b, 0x49, 0x4e, 0x46, 0x4f, 0x0d, 0x0a,
    ]);
  });

  it("encodes UTF-8 text without dropping bytes", () => {
    expect(encodeRawPayload("温度", "ascii")).toEqual(new TextEncoder().encode("温度"));
  });

  it("parses hex with spaces, separators and 0x prefixes", () => {
    expect(Array.from(encodeRawPayload("0x01 03, 00-0A", "hex"))).toEqual([0x01, 0x03, 0x00, 0x0a]);
  });

  it("rejects empty, odd and invalid hex input", () => {
    expect(() => encodeRawPayload("", "ascii")).toThrow("发送内容为空");
    expect(() => encodeRawPayload("  ", "hex")).toThrow("HEX 内容为空");
    expect(() => encodeRawPayload("0", "hex")).toThrow("偶数");
    expect(() => encodeRawPayload("0G", "hex")).toThrow("非法字符");
  });

  it("caps oversized payloads and reports preview length", () => {
    expect(() => encodeRawPayload("A".repeat(MAX_RAW_SEND_BYTES + 1), "ascii")).toThrow("4096");
    expect(rawByteLength("01 02", "hex")).toBe(2);
    expect(rawByteLength("xyz", "hex")).toBeNull();
  });

  it("appends configurable CR/LF suffixes", () => {
    expect(
      Array.from(
        encodeRawPayload("ls", "ascii", {
          appendCr: true,
          appendLf: true,
        }),
      ),
    ).toEqual([0x6c, 0x73, 0x0d, 0x0a]);
    expect(
      Array.from(
        encodeRawPayload("", "ascii", {
          appendLf: true,
        }),
      ),
    ).toEqual([0x0a]);
    expect(rawByteLength("ls", "ascii", { appendCr: true })).toBe(3);
  });

  it("formats debug bytes as text or hex", () => {
    const bytes = [0x41, 0x42, 0x0d, 0x0a, 0x00, 0xff];
    expect(formatDebugBytes(bytes, "ascii")).toBe("AB\n··");
    expect(formatDebugBytes(bytes, "hex")).toBe("41 42 0D 0A 00 FF");
  });
});
