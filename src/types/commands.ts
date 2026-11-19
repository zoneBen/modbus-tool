import type {
  AddressScanInput,
  AlertEvent,
  BaudScanInput,
  CustomWriteInput,
  DebugData,
  DebugState,
  Definition,
  DefinitionInput,
  DefinitionSnapshot,
  Frame,
  NetworkData,
  NetworkScanInput,
  NetworkStartInput,
  NetworkState,
  NewSessionInput,
  RawSendInput,
  ScanDone,
  ScanProgress,
  ScanResult,
  SerialPortInfo,
  Session,
  SessionStatusEvent,
  SlaveIdScanInput,
  SlaveValueInput,
  UpdateSessionInput,
  WorkspaceSnapshot,
} from "./models";
import type {
  ArpCapabilities,
  ArpScanInput,
  ArpScanResult,
  ContinuousPingInput,
  DnsQueryInput,
  DnsResult,
  FileServerStartInput,
  FileServerStatus,
  HttpRequestInput,
  HashInput,
  HttpResponse,
  MqttBrokerInput,
  MqttMessage,
  MqttPublishInput,
  MqttState,
  MqttSubscribeInput,
  PingInput,
  SearchInput,
  TelnetResult,
  TelnetTestInput,
  ToolEvent,
  ToolFileFilter,
  TracerouteInput,
} from "./tools";

export type AppEventName =
  | "session:data"
  | "session:frames"
  | "session:status"
  | "debug:data"
  | "debug:state"
  | "alert:event"
  | "network:state"
  | "network:data"
  | "workspace:updated"
  | "scan:progress"
  | "scan:result"
  | "scan:done"
  | "tool:event"
  | "mqtt:state"
  | "mqtt:message";

export interface AppEventMap {
  "session:data": DefinitionSnapshot;
  "session:frames": { sessionId: string; frames: Frame[] };
  "session:status": SessionStatusEvent;
  "debug:data": DebugData;
  "debug:state": DebugState;
  "alert:event": AlertEvent;
  "network:state": NetworkState;
  "network:data": NetworkData;
  "workspace:updated": WorkspaceSnapshot;
  "scan:progress": ScanProgress;
  "scan:result": ScanResult;
  "scan:done": ScanDone;
  "tool:event": ToolEvent;
  "mqtt:state": MqttState;
  "mqtt:message": MqttMessage;
}

export interface IpcApi {
  readonly mode: "tauri" | "demo";
  boot(): Promise<WorkspaceSnapshot>;
  listSerialPorts(): Promise<SerialPortInfo[]>;
  createSession(input: NewSessionInput): Promise<Session>;
  deleteSession(sessionId: string): Promise<void>;
  connectSession(sessionId: string): Promise<void>;
  disconnectSession(sessionId: string): Promise<void>;
  updateSession(sessionId: string, input: UpdateSessionInput): Promise<void>;
  writeSlaveValue(sessionId: string, input: SlaveValueInput): Promise<number>;
  saveTextFile(suggestedName: string, contents: string, format?: "csv" | "json"): Promise<boolean>;
  openTextFile(): Promise<string | null>;
  setPolling(sessionId: string, enabled: boolean): Promise<void>;
  readOnce(sessionId: string, definitionId: string): Promise<void>;
  setDefinitionEnabled(sessionId: string, definitionId: string, enabled: boolean): Promise<void>;
  createDefinition(sessionId: string, input: DefinitionInput): Promise<Definition>;
  updateDefinition(
    sessionId: string,
    definitionId: string,
    input: DefinitionInput,
  ): Promise<Definition>;
  deleteDefinition(sessionId: string, definitionId: string): Promise<void>;
  scanSlaveIds(input: SlaveIdScanInput): Promise<string>;
  scanAddresses(input: AddressScanInput): Promise<string>;
  scanBaudRates(input: BaudScanInput): Promise<string>;
  scanNetwork(input: NetworkScanInput): Promise<string>;
  cancelScan(scanId: string): Promise<void>;
  writeRegister(
    sessionId: string,
    definitionId: string,
    address: number,
    value: string,
  ): Promise<void>;
  writeCustom(
    sessionId: string,
    definitionId: string | null,
    input: CustomWriteInput,
  ): Promise<void>;
  startDebug(sessionId: string): Promise<void>;
  stopDebug(sessionId: string): Promise<void>;
  sendDebug(sessionId: string, input: RawSendInput): Promise<number>;
  startNetwork(sessionId: string, input: NetworkStartInput): Promise<NetworkState>;
  stopNetwork(sessionId: string): Promise<void>;
  sendNetwork(sessionId: string, input: RawSendInput): Promise<number>;
  toolDnsQuery(input: DnsQueryInput): Promise<DnsResult>;
  toolTelnetTest(input: TelnetTestInput): Promise<TelnetResult>;
  toolHttpRequest(input: HttpRequestInput): Promise<HttpResponse>;
  pickFile(filters?: ToolFileFilter[]): Promise<string | null>;
  pickDirectory(): Promise<string | null>;
  readTextFile(path: string): Promise<string>;
  toolPingStart(input: PingInput): Promise<string>;
  toolLatencyStart(input: ContinuousPingInput): Promise<string>;
  toolTracerouteStart(input: TracerouteInput): Promise<string>;
  toolTaskCancel(taskId: string): Promise<boolean>;
  toolArpCapabilities(): Promise<ArpCapabilities>;
  toolArpScan(input: ArpScanInput): Promise<ArpScanResult>;
  toolHashStart(input: HashInput): Promise<string>;
  toolSearchStart(input: SearchInput): Promise<string>;
  toolFileserverStart(input: FileServerStartInput): Promise<FileServerStatus>;
  toolFileserverStop(): Promise<FileServerStatus>;
  toolFileserverStatus(): Promise<FileServerStatus>;
  toolMqttStart(input: MqttBrokerInput): Promise<void>;
  toolMqttStop(brokerId: string): Promise<boolean>;
  toolMqttPublish(input: MqttPublishInput): Promise<void>;
  toolMqttSubscribe(input: MqttSubscribeInput): Promise<void>;
  toolMqttUnsubscribe(brokerId: string, topic: string): Promise<void>;
  on<K extends AppEventName>(
    event: K,
    handler: (payload: AppEventMap[K]) => void,
  ): Promise<() => void>;
}
