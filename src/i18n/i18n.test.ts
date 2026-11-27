import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import enUS from "./en-US";
import zhCN from "./zh-CN";
import { EN_EXTRA, ZH_EXTRA } from "./tools-extra";

function merge(
  base: Record<string, unknown>,
  extra: Record<string, unknown>,
): Record<string, unknown> {
  const result: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(extra)) {
    const current = result[key];
    result[key] =
      typeof current === "object" &&
      current !== null &&
      !Array.isArray(current) &&
      typeof value === "object" &&
      value !== null &&
      !Array.isArray(value)
        ? merge(current as Record<string, unknown>, value as Record<string, unknown>)
        : value;
  }
  return result;
}

const zhAll = merge(
  zhCN as unknown as Record<string, unknown>,
  ZH_EXTRA as unknown as Record<string, unknown>,
);
const enAll = merge(
  enUS as unknown as Record<string, unknown>,
  EN_EXTRA as unknown as Record<string, unknown>,
);

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
    const zhKeys = flattenKeys(zhAll).sort();
    const enKeys = flattenKeys(enAll).sort();
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
    expect(collect(zhAll)).toEqual([]);
    expect(collect(enAll)).toEqual([]);
  });

  it("has every literal translation key used by components", () => {
    const keys = new Set(flattenKeys(zhAll));
    const missing: string[] = [];
    const walk = (dir: string): void => {
      for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        const stat = statSync(path);
        if (stat.isDirectory()) {
          walk(path);
          continue;
        }
        if (!/\.(vue|ts)$/.test(name) || name.endsWith(".test.ts")) continue;
        const source = readFileSync(path, "utf8");
        const pattern = /(?<!\w)t\(\s*["']([^"']+)["']/g;
        for (const match of source.matchAll(pattern)) {
          const key = match[1];
          if (key.includes("${")) continue;
          if (!keys.has(key)) missing.push(`${path}: ${key}`);
        }
      }
    };
    walk("src");
    expect(missing).toEqual([]);
  });
});
