import { describe, expect, it } from "vitest";
import { diffLines, diffStats } from "./diff";

describe("line diff", () => {
  it("marks additions, removals and unchanged lines", () => {
    const lines = diffLines("a\nb\nc", "a\nB\nc\nd");
    expect(lines.map((line) => line.kind)).toEqual(["same", "remove", "add", "same", "add"]);
    expect(diffStats(lines)).toEqual({ added: 2, removed: 1, same: 2 });
  });

  it("keeps line numbers", () => {
    const lines = diffLines("a", "b");
    const removed = lines.find((line) => line.kind === "remove");
    const added = lines.find((line) => line.kind === "add");
    expect(removed?.leftNo).toBe(1);
    expect(added?.rightNo).toBe(1);
  });
});
