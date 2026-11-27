export type NetworkToolId =
  | "arp"
  | "ping"
  | "latency"
  | "traceroute"
  | "dns"
  | "telnet"
  | "http"
  | "fileserver"
  | "mqtt"
  | "publicip"
  | "ssh"
  | "discovery"
  | "speedtest"
  | "ssl"
  | "snmp";

export type DevToolId =
  | "formatter"
  | "yaml"
  | "timestamp"
  | "base"
  | "hash"
  | "search"
  | "codec"
  | "regex"
  | "jwt"
  | "cron"
  | "diff"
  | "compose"
  | "k8s"
  | "sql"
  | "uuid"
  | "encoding"
  | "image"
  | "qr";

export type SecurityToolId = "password" | "rename" | "log" | "env" | "ports";

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
  {
    id: "publicip",
    labelKey: "networkTools.tools.publicip.title",
    descriptionKey: "networkTools.tools.publicip.description",
    phase: 1,
  },
  {
    id: "ssh",
    labelKey: "networkTools.tools.ssh.title",
    descriptionKey: "networkTools.tools.ssh.description",
    phase: 1,
  },
  {
    id: "discovery",
    labelKey: "networkTools.tools.discovery.title",
    descriptionKey: "networkTools.tools.discovery.description",
    phase: 1,
  },
  {
    id: "speedtest",
    labelKey: "networkTools.tools.speedtest.title",
    descriptionKey: "networkTools.tools.speedtest.description",
    phase: 1,
  },
  {
    id: "ssl",
    labelKey: "networkTools.tools.ssl.title",
    descriptionKey: "networkTools.tools.ssl.description",
    phase: 1,
  },
  {
    id: "snmp",
    labelKey: "networkTools.tools.snmp.title",
    descriptionKey: "networkTools.tools.snmp.description",
    phase: 1,
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
  {
    id: "codec",
    labelKey: "devTools.tools.codec.title",
    descriptionKey: "devTools.tools.codec.description",
    phase: 1,
  },
  {
    id: "regex",
    labelKey: "devTools.tools.regex.title",
    descriptionKey: "devTools.tools.regex.description",
    phase: 1,
  },
  {
    id: "jwt",
    labelKey: "devTools.tools.jwt.title",
    descriptionKey: "devTools.tools.jwt.description",
    phase: 1,
  },
  {
    id: "cron",
    labelKey: "devTools.tools.cron.title",
    descriptionKey: "devTools.tools.cron.description",
    phase: 1,
  },
  {
    id: "diff",
    labelKey: "devTools.tools.diff.title",
    descriptionKey: "devTools.tools.diff.description",
    phase: 1,
  },
  {
    id: "compose",
    labelKey: "devTools.tools.compose.title",
    descriptionKey: "devTools.tools.compose.description",
    phase: 1,
  },
  {
    id: "k8s",
    labelKey: "devTools.tools.k8s.title",
    descriptionKey: "devTools.tools.k8s.description",
    phase: 1,
  },
  {
    id: "sql",
    labelKey: "devTools.tools.sql.title",
    descriptionKey: "devTools.tools.sql.description",
    phase: 1,
  },
  {
    id: "uuid",
    labelKey: "devTools.tools.uuid.title",
    descriptionKey: "devTools.tools.uuid.description",
    phase: 1,
  },
  {
    id: "encoding",
    labelKey: "devTools.tools.encoding.title",
    descriptionKey: "devTools.tools.encoding.description",
    phase: 1,
  },
  {
    id: "image",
    labelKey: "devTools.tools.image.title",
    descriptionKey: "devTools.tools.image.description",
    phase: 1,
  },
  {
    id: "qr",
    labelKey: "devTools.tools.qr.title",
    descriptionKey: "devTools.tools.qr.description",
    phase: 1,
  },
];

export const SECURITY_TOOLS: readonly ToolCatalogEntry<SecurityToolId>[] = [
  {
    id: "password",
    labelKey: "securityTools.tools.password.title",
    descriptionKey: "securityTools.tools.password.description",
    phase: 1,
  },
  {
    id: "rename",
    labelKey: "securityTools.tools.rename.title",
    descriptionKey: "securityTools.tools.rename.description",
    phase: 1,
  },
  {
    id: "log",
    labelKey: "securityTools.tools.log.title",
    descriptionKey: "securityTools.tools.log.description",
    phase: 1,
  },
  {
    id: "env",
    labelKey: "securityTools.tools.env.title",
    descriptionKey: "securityTools.tools.env.description",
    phase: 1,
  },
  {
    id: "ports",
    labelKey: "securityTools.tools.ports.title",
    descriptionKey: "securityTools.tools.ports.description",
    phase: 1,
  },
];

export function isNetworkToolId(value: unknown): value is NetworkToolId {
  return typeof value === "string" && NETWORK_TOOLS.some((tool) => tool.id === value);
}

export function isDevToolId(value: unknown): value is DevToolId {
  return typeof value === "string" && DEV_TOOLS.some((tool) => tool.id === value);
}

export function isSecurityToolId(value: unknown): value is SecurityToolId {
  return typeof value === "string" && SECURITY_TOOLS.some((tool) => tool.id === value);
}

export function isToolAvailable(
  tool: ToolCatalogEntry<string>,
  implemented: ReadonlySet<string>,
): boolean {
  return tool.phase <= 2 && implemented.has(tool.id);
}
