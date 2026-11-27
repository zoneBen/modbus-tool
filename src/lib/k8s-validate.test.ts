import { describe, expect, it } from "vitest";
import { validateK8s } from "./k8s-validate";

const DEPLOYMENT = `apiVersion: apps/v1
kind: Deployment
metadata:
  name: demo
spec:
  selector:
    matchLabels:
      app: demo
  template:
    spec:
      containers:
        - name: demo
          image: nginx:1.27
`;

describe("k8s validation", () => {
  it("accepts a valid deployment", () => {
    const report = validateK8s(DEPLOYMENT);
    expect(report.valid).toBe(true);
    expect(report.resources[0]).toMatchObject({ kind: "Deployment", name: "demo" });
  });

  it("requires metadata.name", () => {
    const report = validateK8s("apiVersion: v1\nkind: ConfigMap\n");
    expect(report.valid).toBe(false);
    expect(report.issues.some((issue) => issue.message === "nameMissing")).toBe(true);
  });

  it("requires containers on workloads", () => {
    const report = validateK8s(`apiVersion: apps/v1
kind: Deployment
metadata:
  name: demo
spec:
  selector:
    matchLabels:
      app: demo
  template:
    spec:
      containers: []
`);
    expect(report.issues.some((issue) => issue.message === "containersMissing")).toBe(true);
  });

  it("handles multiple documents and List resources", () => {
    const report = validateK8s(
      `${DEPLOYMENT}---
apiVersion: v1
kind: List
items:
  - apiVersion: v1
    kind: Service
    metadata:
      name: demo
    spec:
      ports:
        - port: 80
`,
    );
    expect(report.resources.map((resource) => resource.kind)).toEqual(["Deployment", "Service"]);
  });
});
