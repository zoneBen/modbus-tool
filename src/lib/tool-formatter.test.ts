import { describe, expect, it } from "vitest";
import { formatText } from "./tool-formatter";

describe("tool formatter", () => {
  it("formats JSON and JavaScript", async () => {
    const json = await formatText('{"a":1,"b":2}', "json", 2);
    expect(json).toContain('"a": 1');
    const js = await formatText("const a={b:1};", "javascript", 2);
    expect(js).toContain("const a = { b: 1 };");
  });

  it("formats XML", async () => {
    const xml = await formatText("<root><item>1</item></root>", "xml", 2);
    expect(xml).toContain("<root>");
    expect(xml).toContain("  <item>1</item>");
  });

  it("rejects malformed JSON", async () => {
    await expect(formatText("{", "json", 2)).rejects.toThrow();
  });
});
