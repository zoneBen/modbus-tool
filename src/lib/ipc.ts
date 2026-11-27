import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { AppEventMap, AppEventName, IpcApi } from "@/types/commands";
import type {
  AddressScanInput,
  BaudScanInput,
  CustomWriteInput,
  Definition,
  DefinitionInput,
  NetworkScanInput,
  NetworkStartInput,
  NetworkState,
  NewSessionInput,
  RawSendInput,
  SerialPortInfo,
  Session,
  SlaveIdScanInput,
  SlaveValueInput,
  UpdateSessionInput,
  WorkspaceSnapshot,
} from "@/types/models";
import type {
  ArpCapabilities,
  ArpScanInput,
  ArpScanResult,
  ContinuousPingInput,
  DiscoveryInput,
  DiscoveryResult,
  DnsQueryInput,
  DnsResult,
  FileServerStartInput,
  FsListInput,
  FsListResult,
  FileServerStatus,
  HttpRequestInput,
  HashInput,
  HttpResponse,
  MqttBrokerInput,
  MqttPublishInput,
  MqttSubscribeInput,
  PingInput,
  PortListInput,
  PortListResult,
  PublicIpInfo,
  RenameInput,
  RenameResult,
  SearchInput,
  McpToolDescriptor,
  SnmpInput,
  SnmpResult,
  SpeedTestInput,
  SpeedTestResult,
  SshConnectInput,
  SshDownloadInput,
  SshInput,
  SshListResult,
  SshReadResult,
  SshResult,
  SshSessionState,
  SshTransferInput,
  SslCertInput,
  SslCertResult,
  TelnetResult,
  TextConvertInput,
  TextConvertResult,
  TelnetTestInput,
  ToolFileFilter,
  TracerouteInput,
} from "@/types/tools";
import { createDemoBridge } from "./demo-bridge";

function createTauriBridge(): IpcApi {
  return {
    mode: "tauri",
    boot: () => invoke<WorkspaceSnapshot>("workspace_get"),
    listSerialPorts: () => invoke<SerialPortInfo[]>("list_serial_ports"),
    createSession: (input: NewSessionInput) => invoke<Session>("session_create", { input }),
    deleteSession: (sessionId: string) => invoke<void>("session_delete", { sessionId }),
    connectSession: (sessionId: string) => invoke<void>("session_connect", { sessionId }),
    disconnectSession: (sessionId: string) => invoke<void>("session_disconnect", { sessionId }),
    updateSession: (sessionId: string, input: UpdateSessionInput) =>
      invoke<void>("session_update", { sessionId, input }),
    writeSlaveValue: (sessionId: string, input: SlaveValueInput) =>
      invoke<number>("slave_write_value", { sessionId, input }),
    saveTextFile: async (
      suggestedName: string,
      contents: string,
      format: "csv" | "json" = "csv",
    ) => {
      const path = await save({
        defaultPath: suggestedName,
        filters: [
          format === "json"
            ? { name: "JSON", extensions: ["json"] }
            : { name: "CSV", extensions: ["csv"] },
        ],
      });
      if (!path) return false;
      await invoke<void>("save_text_file", { path, contents });
      return true;
    },
    openTextFile: async () => {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path || Array.isArray(path)) return null;
      return invoke<string>("read_text_file", { path });
    },
    setPolling: (sessionId: string, enabled: boolean) =>
      invoke<void>("session_set_polling", { sessionId, enabled }),
    readOnce: (sessionId: string, definitionId: string) =>
      invoke<void>("definition_read_once", { sessionId, definitionId }),
    setDefinitionEnabled: (sessionId: string, definitionId: string, enabled: boolean) =>
      invoke<void>("definition_set_enabled", { sessionId, definitionId, enabled }),
    createDefinition: (sessionId: string, input: DefinitionInput) =>
      invoke<Definition>("definition_create", { sessionId, input }),
    updateDefinition: (sessionId: string, definitionId: string, input: DefinitionInput) =>
      invoke<Definition>("definition_update", { sessionId, definitionId, input }),
    deleteDefinition: (sessionId: string, definitionId: string) =>
      invoke<void>("definition_delete", { sessionId, definitionId }),
    scanSlaveIds: (input: SlaveIdScanInput) => invoke<string>("scan_slave_ids", { input }),
    scanAddresses: (input: AddressScanInput) => invoke<string>("scan_addresses", { input }),
    scanBaudRates: (input: BaudScanInput) => invoke<string>("scan_baud_rates", { input }),
    scanNetwork: (input: NetworkScanInput) => invoke<string>("scan_network", { input }),
    cancelScan: (scanId: string) => invoke<void>("scan_cancel", { scanId }),
    writeRegister: (sessionId: string, definitionId: string, address: number, value: string) =>
      invoke<void>("write_register", { sessionId, definitionId, address, value }),
    writeCustom: (sessionId: string, definitionId: string | null, input: CustomWriteInput) =>
      invoke<void>("write_custom", { sessionId, definitionId, input }),
    startDebug: (sessionId: string) => invoke<void>("debug_start", { sessionId }),
    stopDebug: (sessionId: string) => invoke<void>("debug_stop", { sessionId }),
    sendDebug: (sessionId: string, input: RawSendInput) =>
      invoke<number>("debug_send", { sessionId, input }),
    startNetwork: (sessionId: string, input: NetworkStartInput) =>
      invoke<NetworkState>("network_start", { sessionId, input }),
    stopNetwork: (sessionId: string) => invoke<void>("network_stop", { sessionId }),
    sendNetwork: (sessionId: string, input: RawSendInput) =>
      invoke<number>("network_send", { sessionId, input }),
    toolDnsQuery: (input: DnsQueryInput) => invoke<DnsResult>("tool_dns_query", { input }),
    toolTelnetTest: (input: TelnetTestInput) => invoke<TelnetResult>("tool_telnet_test", { input }),
    toolHttpRequest: (input: HttpRequestInput) =>
      invoke<HttpResponse>("tool_http_request", { input }),
    pickFile: async (filters: ToolFileFilter[] = []) => {
      const path = await open({
        multiple: false,
        directory: false,
        filters: filters.length ? filters : undefined,
      });
      return typeof path === "string" ? path : null;
    },
    pickDirectory: async () => {
      const path = await open({ multiple: false, directory: true });
      return typeof path === "string" ? path : null;
    },
    readTextFile: (path: string) => invoke<string>("read_text_file", { path }),
    toolPingStart: (input: PingInput) => invoke<string>("tool_ping_start", { input }),
    toolLatencyStart: (input: ContinuousPingInput) =>
      invoke<string>("tool_latency_start", { input }),
    toolTracerouteStart: (input: TracerouteInput) =>
      invoke<string>("tool_traceroute_start", { input }),
    toolTaskCancel: (taskId: string) => invoke<boolean>("tool_task_cancel", { taskId }),
    toolArpCapabilities: () => invoke<ArpCapabilities>("tool_arp_capabilities"),
    toolArpScan: (input: ArpScanInput) => invoke<ArpScanResult>("tool_arp_scan", { input }),
    toolHashStart: (input: HashInput) => invoke<string>("tool_hash_start", { input }),
    toolSearchStart: (input: SearchInput) => invoke<string>("tool_search_start", { input }),
    toolFileserverStart: (input: FileServerStartInput) =>
      invoke<FileServerStatus>("tool_fileserver_start", { input }),
    toolFileserverStop: () => invoke<FileServerStatus>("tool_fileserver_stop"),
    toolFileserverStatus: (path?: string) =>
      invoke<FileServerStatus>("tool_fileserver_status", { path: path || null }),
    toolMqttStart: (input: MqttBrokerInput) => invoke<void>("tool_mqtt_start", { input }),
    toolMqttStop: (brokerId: string) => invoke<boolean>("tool_mqtt_stop", { brokerId }),
    toolMqttPublish: (input: MqttPublishInput) => invoke<void>("tool_mqtt_publish", { input }),
    toolMqttSubscribe: (input: MqttSubscribeInput) =>
      invoke<void>("tool_mqtt_subscribe", { input }),
    toolMqttUnsubscribe: (brokerId: string, topic: string) =>
      invoke<void>("tool_mqtt_unsubscribe", { brokerId, topic }),
    toolPublicIp: () => invoke<PublicIpInfo>("tool_public_ip"),
    toolSshConnect: (input: SshConnectInput) =>
      invoke<SshSessionState>("tool_ssh_connect", { input }),
    toolSshList: () => invoke<SshSessionState[]>("tool_ssh_list"),
    toolSshWrite: (sessionId: string, data: string) =>
      invoke<void>("tool_ssh_write", { sessionId, data }),
    toolSshResize: (sessionId: string, cols: number, rows: number) =>
      invoke<void>("tool_ssh_resize", { sessionId, cols, rows }),
    toolSshDisconnect: (sessionId: string) => invoke<boolean>("tool_ssh_disconnect", { sessionId }),
    toolSshSftpList: (sessionId: string, path?: string) =>
      invoke<SshListResult>("tool_ssh_sftp_list", { sessionId, path: path || null }),
    toolSshSftpMkdir: (sessionId: string, path: string) =>
      invoke<void>("tool_ssh_sftp_mkdir", { sessionId, path }),
    toolSshSftpRemove: (sessionId: string, path: string, isDir: boolean) =>
      invoke<void>("tool_ssh_sftp_remove", { sessionId, path, isDir }),
    toolSshSftpRename: (sessionId: string, from: string, to: string) =>
      invoke<void>("tool_ssh_sftp_rename", { sessionId, from, to }),
    toolSshUpload: (input: SshTransferInput) => invoke<void>("tool_ssh_upload", { input }),
    toolSshDownload: (input: SshDownloadInput) => invoke<void>("tool_ssh_download", { input }),
    toolSshSetCwdTracking: (sessionId: string, enabled: boolean) =>
      invoke<void>("tool_ssh_set_cwd_tracking", { sessionId, enabled }),
    toolSshReadOutput: (sessionId: string, cursor?: number) =>
      invoke<SshReadResult>("tool_ssh_read_output", { sessionId, cursor: cursor ?? null }),
    toolSshMcpTools: () => invoke<McpToolDescriptor[]>("tool_ssh_mcp_tools"),
    toolSshMcpCall: (name: string, args: Record<string, unknown>) =>
      invoke<unknown>("tool_ssh_mcp_call", { request: { name, arguments: args } }),
    pickSavePath: async (suggestedName: string) => {
      const path = await save({ defaultPath: suggestedName });
      return typeof path === "string" ? path : null;
    },
    toolSshExecute: (input: SshInput) => invoke<SshResult>("tool_ssh_execute", { input }),
    toolLanDiscovery: (input: DiscoveryInput) =>
      invoke<DiscoveryResult>("tool_lan_discovery", { input }),
    toolSpeedtestRun: (input: SpeedTestInput) =>
      invoke<SpeedTestResult>("tool_speedtest_run", { input }),
    toolSslCert: (input: SslCertInput) => invoke<SslCertResult>("tool_ssl_cert", { input }),
    toolSnmpQuery: (input: SnmpInput) => invoke<SnmpResult>("tool_snmp_query", { input }),
    toolTextConvert: (input: TextConvertInput) =>
      invoke<TextConvertResult>("tool_text_convert", { input }),
    toolFsList: (input: FsListInput) => invoke<FsListResult>("tool_fs_list", { input }),
    toolFsRename: (input: RenameInput) => invoke<RenameResult>("tool_fs_rename", { input }),
    toolPortList: (input: PortListInput) => invoke<PortListResult>("tool_port_list", { input }),
    on: async <K extends AppEventName>(
      event: K,
      handler: (payload: AppEventMap[K]) => void,
    ): Promise<() => void> => listen<AppEventMap[K]>(event, (message) => handler(message.payload)),
  };
}

export const ipc: IpcApi = isTauri() ? createTauriBridge() : createDemoBridge();
export const isDemoMode = ipc.mode === "demo";
