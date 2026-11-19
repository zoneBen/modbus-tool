import { onBeforeUnmount, onMounted, ref } from "vue";
import { ipc } from "@/lib/ipc";
import type { ToolEvent } from "@/types/tools";

/**
 * Subscribes to the shared `tool:event` channel and buffers events that arrive before the
 * backend command has returned its task id. Without the buffer a fast task (small hash, quick
 * search) can finish before the renderer knows which id to filter on, leaving the UI at 0%.
 */
export function useToolEventBuffer(apply: (event: ToolEvent) => void) {
  const taskId = ref<string | null>(null);
  const pending: ToolEvent[] = [];
  let unlisten: (() => void) | null = null;

  function dispatch(event: ToolEvent): void {
    if (!taskId.value) {
      pending.push(event);
      if (pending.length > 2048) pending.shift();
      return;
    }
    if (event.taskId === taskId.value) apply(event);
  }

  onMounted(async () => {
    unlisten = await ipc.on("tool:event", dispatch);
  });

  onBeforeUnmount(() => {
    unlisten?.();
    unlisten = null;
  });

  function bind(id: string): void {
    taskId.value = id;
    const queued = pending.splice(0);
    for (const event of queued) {
      if (event.taskId === id) apply(event);
    }
  }

  function unbind(): void {
    taskId.value = null;
  }

  return { taskId, bind, unbind };
}
