import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppEventMap, AppEventName, IpcApi } from "@/types/commands";
import type {
  AddressScanInput,
  BaudScanInput,
  Definition,
  DefinitionInput,
  NetworkScanInput,
  NewSessionInput,
  SerialPortInfo,
  Session,
  SlaveIdScanInput,
  WorkspaceSnapshot,
} from "@/types/models";
import { createDemoBridge } from "./demo-bridge";

function createTauriBridge(): IpcApi {
  return {
    mode: "tauri",
    boot: () => invoke<WorkspaceSnapshot>("workspace_get"),
    listSerialPorts: () => invoke<SerialPortInfo[]>("list_serial_ports"),
    createSession: (input: NewSessionInput) => invoke<Session>("session_create", { input }),
    deleteSession: (sessionId: string) => invoke<void>("session_delete", { sessionId }),
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
    on: async <K extends AppEventName>(
      event: K,
      handler: (payload: AppEventMap[K]) => void,
    ): Promise<() => void> => listen<AppEventMap[K]>(event, (message) => handler(message.payload)),
  };
}

export const ipc: IpcApi = isTauri() ? createTauriBridge() : createDemoBridge();
export const isDemoMode = ipc.mode === "demo";
