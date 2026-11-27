import { parse } from "yaml";

export interface ComposeIssue {
  level: "error" | "warning";
  path: string;
  message: string;
}

export interface ComposeReport {
  valid: boolean;
  services: string[];
  dependencies: Record<string, string[]>;
  issues: ComposeIssue[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function dependencies(service: unknown): string[] {
  if (!isRecord(service)) return [];
  const dependsOn = service.depends_on;
  if (Array.isArray(dependsOn))
    return dependsOn.filter((item): item is string => typeof item === "string");
  if (isRecord(dependsOn)) return Object.keys(dependsOn);
  return [];
}

function findCycle(graph: Map<string, string[]>): string[] | null {
  const visiting = new Set<string>();
  const visited = new Set<string>();
  const stack: string[] = [];

  function visit(node: string): string[] | null {
    if (visiting.has(node)) {
      const start = stack.indexOf(node);
      return [...stack.slice(start), node];
    }
    if (visited.has(node)) return null;
    visiting.add(node);
    stack.push(node);
    for (const next of graph.get(node) ?? []) {
      const cycle = visit(next);
      if (cycle) return cycle;
    }
    stack.pop();
    visiting.delete(node);
    visited.add(node);
    return null;
  }

  for (const node of graph.keys()) {
    const cycle = visit(node);
    if (cycle) return cycle;
  }
  return null;
}

function collectReferences(
  value: unknown,
  target: string,
  issues: ComposeIssue[],
  name: string,
): void {
  if (value === undefined) return;
  if (!Array.isArray(value)) return;
  for (const entry of value) {
    if (typeof entry === "string" && !entry.includes(":")) {
      issues.push({
        level: "warning",
        path: `services.${name}.${target}`,
        message: `"${entry}" references an undefined top-level ${target} entry`,
      });
    }
  }
}

export function validateCompose(text: string): ComposeReport {
  const issues: ComposeIssue[] = [];
  let document: unknown;
  try {
    document = parse(text);
  } catch (error: unknown) {
    return {
      valid: false,
      services: [],
      dependencies: {},
      issues: [
        {
          level: "error",
          path: "$",
          message: error instanceof Error ? error.message : String(error),
        },
      ],
    };
  }

  if (!isRecord(document)) {
    issues.push({ level: "error", path: "$", message: "composeFileMustBeObject" });
    return { valid: false, services: [], dependencies: {}, issues };
  }
  if (!isRecord(document.services)) {
    issues.push({ level: "error", path: "services", message: "servicesMissing" });
    return { valid: false, services: [], dependencies: {}, issues };
  }

  const services = Object.keys(document.services);
  const graph = new Map<string, string[]>();
  const topNetworks = isRecord(document.networks) ? Object.keys(document.networks) : [];
  const topVolumes = isRecord(document.volumes) ? Object.keys(document.volumes) : [];

  for (const name of services) {
    const service = document.services[name];
    if (!isRecord(service)) {
      issues.push({ level: "error", path: `services.${name}`, message: "serviceMustBeObject" });
      continue;
    }
    const deps = dependencies(service);
    graph.set(name, deps);
    for (const dependency of deps) {
      if (!(dependency in document.services)) {
        issues.push({
          level: "error",
          path: `services.${name}.depends_on`,
          message: `dependsOnMissing:${dependency}`,
        });
      }
    }
    if (service.image === undefined && service.build === undefined) {
      issues.push({
        level: "warning",
        path: `services.${name}`,
        message: "serviceNeedsImageOrBuild",
      });
    }
    if (service.ports !== undefined && !Array.isArray(service.ports)) {
      issues.push({
        level: "error",
        path: `services.${name}.ports`,
        message: "portsMustBeArray",
      });
    }
    if (service.networks !== undefined && !Array.isArray(service.networks)) {
      issues.push({
        level: "warning",
        path: `services.${name}.networks`,
        message: "networksShouldBeArray",
      });
    }
    if (Array.isArray(service.networks) && topNetworks.length) {
      collectReferences(service.networks, "networks", issues, name);
    }
    if (Array.isArray(service.volumes)) {
      for (const volume of service.volumes) {
        if (typeof volume === "string" && volume.includes(":")) {
          const source = volume.split(":")[0];
          if (
            source &&
            !source.startsWith(".") &&
            !source.startsWith("/") &&
            !source.includes("$")
          ) {
            if (topVolumes.length && !topVolumes.includes(source)) {
              issues.push({
                level: "warning",
                path: `services.${name}.volumes`,
                message: `volumeMissing:${source}`,
              });
            }
          }
        }
      }
    }
  }

  const cycle = findCycle(graph);
  if (cycle) {
    issues.push({
      level: "error",
      path: "services.depends_on",
      message: `dependencyCycle:${cycle.join(" -> ")}`,
    });
  }

  return {
    valid: !issues.some((issue) => issue.level === "error"),
    services,
    dependencies: Object.fromEntries(graph),
    issues,
  };
}
