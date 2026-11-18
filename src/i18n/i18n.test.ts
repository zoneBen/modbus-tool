import { describe, expect, it } from "vitest";
import enUS from "./en-US";
import zhCN from "./zh-CN";

function flattenKeys(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }
  return Object.entries(value).flatMap(([key, child]) =>
    flattenKeys(child, prefix ? `${prefix}.${key}` : key),
  );
}

describe("locale messages", () => {
  it("keeps Chinese and English translation keys in sync", () => {
    const zhKeys = flattenKeys(zhCN).sort();
    const enKeys = flattenKeys(enUS).sort();
    expect(enKeys).toEqual(zhKeys);
  });

  it("has no empty translations", () => {
    const collect = (value: unknown, prefix = ""): string[] => {
      if (typeof value === "string") return value.trim() ? [] : [prefix];
      if (typeof value !== "object" || value === null) return [];
      return Object.entries(value).flatMap(([key, child]) =>
        collect(child, prefix ? `${prefix}.${key}` : key),
      );
    };
    expect(collect(zhCN)).toEqual([]);
    expect(collect(enUS)).toEqual([]);
  });
});
