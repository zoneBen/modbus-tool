import type {
  AddressScanInput,
  BaudScanInput,
  CustomWriteInput,
  Definition,
  DefinitionInput,
  DefinitionSnapshot,
  Frame,
  NetworkScanInput,
  NewSessionInput,
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

export type AppEventName =
  | "session:data"
  | "session:frames"
  | "session:status"
  | "workspace:updated"
  | "scan:progress"
  | "scan:result"
  | "scan:done";

export interface AppEventMap {
  "session:data": DefinitionSnapshot;
  "session:frames": { sessionId: string; frames: Frame[] };
  "session:status": SessionStatusEvent;
  "workspace:updated": WorkspaceSnapshot;
  "scan:progress": ScanProgress;
  "scan:result": ScanResult;
  "scan:done": ScanDone;
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
  saveTextFile(suggestedName: string, contents: string): Promise<boolean>;
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
  on<K extends AppEventName>(
    event: K,
    handler: (payload: AppEventMap[K]) => void,
  ): Promise<() => void>;
}
