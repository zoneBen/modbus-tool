import { ref } from "vue";
import { ipc } from "@/lib/ipc";
import type { SerialPortInfo } from "@/types/models";

export function useSerialPorts() {
  const ports = ref<SerialPortInfo[]>([]);
  const loading = ref(false);
  const error = ref("");
  let requestId = 0;

  async function refresh(): Promise<void> {
    const current = ++requestId;
    loading.value = true;
    error.value = "";
    try {
      const list = await ipc.listSerialPorts();
      if (current !== requestId) return;
      ports.value = list;
    } catch (cause: unknown) {
      if (current !== requestId) return;
      ports.value = [];
      error.value = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (current === requestId) loading.value = false;
    }
  }

  return { ports, loading, error, refresh };
}
