import { describe, expect, it } from "vitest";
import {
  base64ToHex,
  decodeBase64,
  decodeUrl,
  encodeBase64,
  encodeUrl,
  hexToBase64,
  hexToText,
  textToHex,
} from "./codec";

describe("codec helpers", () => {
  it("round-trips unicode through base64", () => {
    const encoded = encodeBase64("Modbus 调试");
    expect(decodeBase64(encoded)).toBe("Modbus 调试");
  });

  it("supports url-safe base64", () => {
    const encoded = encodeBase64("a?b/c+d", true);
    expect(encoded).not.toMatch(/[+/=]/);
    expect(decodeBase64(encoded)).toBe("a?b/c+d");
  });

  it("round-trips url encoding", () => {
    const encoded = encodeUrl("a b&c=d/中文");
    expect(encoded).toContain("%20");
    expect(decodeUrl(encoded)).toBe("a b&c=d/中文");
  });

  it("round-trips hex text", () => {
    const hex = textToHex("AB中");
    expect(hex).toBe("41 42 E4 B8 AD");
    expect(hexToText(hex)).toBe("AB中");
  });

  it("rejects odd-length hex", () => {
    expect(() => hexToText("ABC")).toThrow("hexOddLength");
  });

  it("converts between base64 and hex bytes", () => {
    expect(base64ToHex("QUI=")).toBe("41 42");
    expect(hexToBase64("41 42")).toBe("QUI=");
  });
});
