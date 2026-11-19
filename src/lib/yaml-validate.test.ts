import { describe, expect, it } from "vitest";
import { validateYaml } from "./yaml-validate";

describe("yaml validation", () => {
  it("accepts multi-document yaml", () => {
    const result = validateYaml("name: demo\nports:\n  - 502\n---\nname: second\n");
    expect(result.valid).toBe(true);
    expect(result.documents).toBe(2);
    expect(result.errors).toHaveLength(0);
  });

  it("reports the line of a syntax error", () => {
    const result = validateYaml("name: demo\nports: [502, 503\nenabled: true\n");
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].line).toBeGreaterThanOrEqual(2);
    expect(result.errors[0].message).not.toContain("at line");
  });

  it("rejects empty input", () => {
    const result = validateYaml("   ");
    expect(result.valid).toBe(false);
    expect(result.errors[0].message).toBe("empty");
  });
});
