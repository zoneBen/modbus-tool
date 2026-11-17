import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import type { AppEventMap, AppEventName, IpcApi } from "@/types/commands";
import type {
  AddressScanInput,
  BaudScanInput,
  CustomWriteInput,
  Definition,
  DefinitionInput,
  NetworkScanInput,
  NewSessionInput,
  RawSendInput,
  RawSendResult,
  SerialPortInfo,
  Session,
  SlaveIdScanInput,
  SlaveValueInput,
  UpdateSessionInput,
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
    connectSession: (sessionId: string) => invoke<void>("session_connect", { sessionId }),
    disconnectSession: (sessionId: string) => invoke<void>("session_disconnect", { sessionId }),
    updateSession: (sessionId: string, input: UpdateSessionInput) =>
      invoke<void>("session_update", { sessionId, input }),
    writeSlaveValue: (sessionId: string, input: SlaveValueInput) =>
      invoke<number>("slave_write_value", { sessionId, input }),
    saveTextFile: async (suggestedName: string, contents: string) => {
      const path = await save({
        defaultPath: suggestedName,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!path) return false;
      await invoke<void>("save_text_file", { path, contents });
      return true;
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
    sendRaw: (sessionId: string, input: RawSendInput) =>
      invoke<RawSendResult>("session_send_raw", { sessionId, input }),
    on: async <K extends AppEventName>(
      event: K,
      handler: (payload: AppEventMap[K]) => void,
    ): Promise<() => void> => listen<AppEventMap[K]>(event, (message) => handler(message.payload)),
  };
}

export const ipc: IpcApi = isTauri() ? createTauriBridge() : createDemoBridge();
export const isDemoMode = ipc.mode === "demo";
