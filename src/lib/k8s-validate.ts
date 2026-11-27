import { parseAllDocuments } from "yaml";

export interface K8sIssue {
  level: "error" | "warning";
  document: number;
  path: string;
  message: string;
}

export interface K8sResource {
  document: number;
  apiVersion: string;
  kind: string;
  name: string;
  namespace?: string;
}

export interface K8sReport {
  valid: boolean;
  resources: K8sResource[];
  issues: K8sIssue[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function checkWorkload(document: Record<string, unknown>, index: number, issues: K8sIssue[]): void {
  const spec = document.spec;
  if (!isRecord(spec)) {
    issues.push({ level: "error", document: index, path: "spec", message: "specMissing" });
    return;
  }
  if (!isRecord(spec.selector)) {
    issues.push({
      level: "error",
      document: index,
      path: "spec.selector",
      message: "selectorMissing",
    });
  }
  if (!isRecord(spec.template)) {
    issues.push({
      level: "error",
      document: index,
      path: "spec.template",
      message: "podTemplateMissing",
    });
    return;
  }
  const templateSpec = spec.template.spec;
  if (!isRecord(templateSpec)) {
    issues.push({
      level: "error",
      document: index,
      path: "spec.template.spec",
      message: "podSpecMissing",
    });
    return;
  }
  const containers = templateSpec.containers;
  if (!Array.isArray(containers) || containers.length === 0) {
    issues.push({
      level: "error",
      document: index,
      path: "spec.template.spec.containers",
      message: "containersMissing",
    });
  } else {
    containers.forEach((container, containerIndex) => {
      if (!isRecord(container) || typeof container.name !== "string") {
        issues.push({
          level: "error",
          document: index,
          path: `spec.template.spec.containers[${containerIndex}].name`,
          message: "containerNameMissing",
        });
      }
      if (isRecord(container) && container.image === undefined) {
        issues.push({
          level: "error",
          document: index,
          path: `spec.template.spec.containers[${containerIndex}].image`,
          message: "containerImageMissing",
        });
      }
    });
  }
}

export function validateK8s(text: string): K8sReport {
  const documents = parseAllDocuments(text);
  const resources: K8sResource[] = [];
  const issues: K8sIssue[] = [];

  documents.forEach((parsed, index) => {
    if (parsed.errors.length) {
      for (const error of parsed.errors) {
        issues.push({
          level: "error",
          document: index + 1,
          path: "$",
          message: error.message,
        });
      }
      return;
    }
    const document: unknown = parsed.toJS();
    if (!isRecord(document)) {
      issues.push({
        level: "warning",
        document: index + 1,
        path: "$",
        message: "emptyDocument",
      });
      return;
    }
    if (document.kind === "List" && Array.isArray(document.items)) {
      document.items.forEach((item) => {
        if (isRecord(item)) validateResource(item, index + 1, resources, issues);
      });
      return;
    }
    validateResource(document, index + 1, resources, issues);
  });

  if (!resources.length && !issues.some((issue) => issue.level === "error")) {
    issues.push({ level: "error", document: 1, path: "$", message: "noResources" });
  }

  return {
    valid: !issues.some((issue) => issue.level === "error"),
    resources,
    issues,
  };
}

function validateResource(
  document: Record<string, unknown>,
  index: number,
  resources: K8sResource[],
  issues: K8sIssue[],
): void {
  const apiVersion = document.apiVersion;
  const kind = document.kind;
  const metadata = document.metadata;
  if (typeof apiVersion !== "string" || !apiVersion) {
    issues.push({
      level: "error",
      document: index,
      path: "apiVersion",
      message: "apiVersionMissing",
    });
  }
  if (typeof kind !== "string" || !kind) {
    issues.push({ level: "error", document: index, path: "kind", message: "kindMissing" });
  }
  if (!isRecord(metadata) || typeof metadata.name !== "string" || !metadata.name) {
    issues.push({ level: "error", document: index, path: "metadata.name", message: "nameMissing" });
  }

  resources.push({
    document: index,
    apiVersion: typeof apiVersion === "string" ? apiVersion : "—",
    kind: typeof kind === "string" ? kind : "—",
    name: isRecord(metadata) && typeof metadata.name === "string" ? metadata.name : "—",
    namespace:
      isRecord(metadata) && typeof metadata.namespace === "string" ? metadata.namespace : undefined,
  });

  if (
    kind === "Deployment" ||
    kind === "StatefulSet" ||
    kind === "DaemonSet" ||
    kind === "ReplicaSet" ||
    kind === "Job"
  ) {
    checkWorkload(document, index, issues);
  }
  if (kind === "Service") {
    const spec = document.spec;
    if (!isRecord(spec)) {
      issues.push({ level: "error", document: index, path: "spec", message: "specMissing" });
    } else if (!Array.isArray(spec.ports) || spec.ports.length === 0) {
      issues.push({
        level: "warning",
        document: index,
        path: "spec.ports",
        message: "servicePortsMissing",
      });
    }
  }
  if (kind === "ConfigMap" && !isRecord(document.data) && !isRecord(document.binaryData)) {
    issues.push({
      level: "warning",
      document: index,
      path: "data",
      message: "configMapDataMissing",
    });
  }
}
