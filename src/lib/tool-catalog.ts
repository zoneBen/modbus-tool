export type NetworkToolId =
  "arp" | "ping" | "latency" | "traceroute" | "dns" | "telnet" | "http" | "fileserver" | "mqtt";

export type DevToolId = "formatter" | "yaml" | "timestamp" | "base" | "hash" | "search";

export interface ToolCatalogEntry<Id extends string> {
  id: Id;
  labelKey: string;
  descriptionKey: string;
  phase: 1 | 2 | 3;
}

export const NETWORK_TOOLS: readonly ToolCatalogEntry<NetworkToolId>[] = [
  {
    id: "arp",
    labelKey: "networkTools.tools.arp.title",
    descriptionKey: "networkTools.tools.arp.description",
    phase: 1,
  },
  {
    id: "ping",
    labelKey: "networkTools.tools.ping.title",
    descriptionKey: "networkTools.tools.ping.description",
    phase: 1,
  },
  {
    id: "latency",
    labelKey: "networkTools.tools.latency.title",
    descriptionKey: "networkTools.tools.latency.description",
    phase: 1,
  },
  {
    id: "traceroute",
    labelKey: "networkTools.tools.traceroute.title",
    descriptionKey: "networkTools.tools.traceroute.description",
    phase: 1,
  },
  {
    id: "dns",
    labelKey: "networkTools.tools.dns.title",
    descriptionKey: "networkTools.tools.dns.description",
    phase: 1,
  },
  {
    id: "telnet",
    labelKey: "networkTools.tools.telnet.title",
    descriptionKey: "networkTools.tools.telnet.description",
    phase: 1,
  },
  {
    id: "http",
    labelKey: "networkTools.tools.http.title",
    descriptionKey: "networkTools.tools.http.description",
    phase: 1,
  },
  {
    id: "fileserver",
    labelKey: "networkTools.tools.fileserver.title",
    descriptionKey: "networkTools.tools.fileserver.description",
    phase: 2,
  },
  {
    id: "mqtt",
    labelKey: "networkTools.tools.mqtt.title",
    descriptionKey: "networkTools.tools.mqtt.description",
    phase: 2,
  },
];

export const DEV_TOOLS: readonly ToolCatalogEntry<DevToolId>[] = [
  {
    id: "formatter",
    labelKey: "devTools.tools.formatter.title",
    descriptionKey: "devTools.tools.formatter.description",
    phase: 1,
  },
  {
    id: "yaml",
    labelKey: "devTools.tools.yaml.title",
    descriptionKey: "devTools.tools.yaml.description",
    phase: 1,
  },
  {
    id: "timestamp",
    labelKey: "devTools.tools.timestamp.title",
    descriptionKey: "devTools.tools.timestamp.description",
    phase: 1,
  },
  {
    id: "base",
    labelKey: "devTools.tools.base.title",
    descriptionKey: "devTools.tools.base.description",
    phase: 1,
  },
  {
    id: "hash",
    labelKey: "devTools.tools.hash.title",
    descriptionKey: "devTools.tools.hash.description",
    phase: 1,
  },
  {
    id: "search",
    labelKey: "devTools.tools.search.title",
    descriptionKey: "devTools.tools.search.description",
    phase: 1,
  },
];

export function isNetworkToolId(value: unknown): value is NetworkToolId {
  return typeof value === "string" && NETWORK_TOOLS.some((tool) => tool.id === value);
}

export function isDevToolId(value: unknown): value is DevToolId {
  return typeof value === "string" && DEV_TOOLS.some((tool) => tool.id === value);
}

export function isToolAvailable(
  tool: ToolCatalogEntry<string>,
  implemented: ReadonlySet<string>,
): boolean {
  return tool.phase <= 2 && implemented.has(tool.id);
}
