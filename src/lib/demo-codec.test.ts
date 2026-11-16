import { describe, expect, it } from "vitest";
import {
  applyByteOrder,
  asciiFrame,
  buildDecodedFrame,
  crc16Modbus,
  crcBytes,
  decodeFloatFromWords,
  floatToWords,
  lrcByte,
  mbapFrame,
  readRequestPdu,
  readResponsePdu,
  rtuFrame,
  wordsToDataBytes,
} from "./demo-codec";

describe("modbus demo codec", () => {
  it("matches the standard CRC16 vector", () => {
    const request = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0a];
    expect(crc16Modbus(request)).toBe(0xcdc5);
    expect(crcBytes(request)).toEqual([0xc5, 0xcd]);
  });

  it("computes the standard LRC vector", () => {
    expect(lrcByte([0x01, 0x03, 0x00, 0x00, 0x00, 0x0a])).toBe(0xf2);
  });

  it("builds MBAP frames with unit id and length", () => {
    const frame = mbapFrame(1, 1, readRequestPdu(0, 16));
    expect(frame).toEqual([0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00, 0x00, 0x00, 0x10]);
    const decoded = buildDecodedFrame("mbap", frame);
    expect(decoded.slaveId).toBe(1);
    expect(decoded.functionCode).toBe(3);
    expect(decoded.fields.map((field) => field.tone)).toContain("header");
  });

  it("appends CRC bytes in RTU order", () => {
    const frame = rtuFrame(1, [0x03, 0x00, 0x00, 0x00, 0x0a]);
    expect(frame.slice(0, 6)).toEqual([0x01, 0x03, 0x00, 0x00, 0x00, 0x0a]);
    expect(frame.slice(6)).toEqual([0xc5, 0xcd]);
    const decoded = buildDecodedFrame("rtu", frame);
    expect(decoded.fields.at(-1)?.tone).toBe("checksum");
  });

  it("encodes ASCII frames with colon, uppercase hex and CRLF", () => {
    const frame = asciiFrame(1, [0x03, 0x00, 0x00, 0x00, 0x0a]);
    const text = new TextDecoder().decode(new Uint8Array(frame));
    expect(text.startsWith(":01030000000AF2")).toBe(true);
    expect(text.endsWith("\r\n")).toBe(true);
  });

  it("applies all four byte/word order presets", () => {
    const words = [0x1122, 0x3344];
    expect(applyByteOrder(words, "abcd")).toEqual([0x1122, 0x3344]);
    expect(applyByteOrder(words, "cdab")).toEqual([0x3344, 0x1122]);
    expect(applyByteOrder(words, "badc")).toEqual([0x2211, 0x4433]);
    expect(applyByteOrder(words, "dcba")).toEqual([0x4433, 0x2211]);
  });

  it("round-trips float32 values and register bytes", () => {
    const words = floatToWords(23.46, "abcd");
    expect(words).toHaveLength(2);
    expect(decodeFloatFromWords(words)).toBeCloseTo(23.46, 3);
    expect(wordsToDataBytes([0x1234, 0xabcd])).toEqual([0x12, 0x34, 0xab, 0xcd]);
  });

  it("builds read responses with byte count", () => {
    const pdu = readResponsePdu(3, [0x00, 0x2a, 0x00, 0x2b]);
    expect(pdu).toEqual([0x03, 0x04, 0x00, 0x2a, 0x00, 0x2b]);
  });
});
