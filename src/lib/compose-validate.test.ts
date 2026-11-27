import { describe, expect, it } from "vitest";
import { validateCompose } from "./compose-validate";

describe("compose validation", () => {
  it("accepts a valid compose file", () => {
    const report = validateCompose(`services:
  web:
    image: nginx:1.27
    ports:
      - "8080:80"
    depends_on:
      - api
  api:
    build: .
`);
    expect(report.valid).toBe(true);
    expect(report.services).toEqual(["web", "api"]);
  });

  it("reports missing services", () => {
    const report = validateCompose("version: '3'\n");
    expect(report.valid).toBe(false);
    expect(report.issues[0].message).toBe("servicesMissing");
  });

  it("detects missing dependencies and cycles", () => {
    const report = validateCompose(`services:
  a:
    image: alpine
    depends_on: [b]
  b:
    image: alpine
    depends_on: [a]
  c:
    image: alpine
    depends_on: [ghost]
`);
    expect(report.valid).toBe(false);
    expect(report.issues.some((issue) => issue.message === "dependsOnMissing:ghost")).toBe(true);
    expect(report.issues.some((issue) => issue.message.startsWith("dependencyCycle"))).toBe(true);
  });

  it("reports YAML syntax errors", () => {
    const report = validateCompose("services: [");
    expect(report.valid).toBe(false);
    expect(report.issues[0].level).toBe("error");
  });
});
