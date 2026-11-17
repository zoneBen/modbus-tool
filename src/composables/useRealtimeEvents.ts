import { onMounted, onUnmounted } from "vue";
import { ipc } from "@/lib/ipc";
import { useDebugStore } from "@/stores/debug";
import { useFramesStore } from "@/stores/frames";
import { useScansStore } from "@/stores/scans";
import { useSessionsStore } from "@/stores/sessions";

export function useRealtimeEvents(): void {
  const sessions = useSessionsStore();
  const frames = useFramesStore();
  const scans = useScansStore();
  const debug = useDebugStore();
  const unsubscribers: Array<() => void> = [];

  onMounted(async () => {
    unsubscribers.push(
      await ipc.on("session:data", (snapshot) => sessions.applySnapshot(snapshot)),
    );
    unsubscribers.push(
      await ipc.on("session:frames", (payload) =>
        frames.appendFrames(payload.sessionId, payload.frames),
      ),
    );
    unsubscribers.push(await ipc.on("session:status", (status) => sessions.applyStatus(status)));
    unsubscribers.push(await ipc.on("debug:data", (data) => debug.applyData(data)));
    unsubscribers.push(await ipc.on("debug:state", (state) => debug.applyState(state)));
    unsubscribers.push(
      await ipc.on("workspace:updated", (workspace) => sessions.applyWorkspace(workspace)),
    );
    unsubscribers.push(await ipc.on("scan:progress", (progress) => scans.applyProgress(progress)));
    unsubscribers.push(await ipc.on("scan:result", (result) => scans.applyResult(result)));
    unsubscribers.push(await ipc.on("scan:done", (done) => scans.applyDone(done)));
    await sessions.boot();
  });

  onUnmounted(() => {
    for (const unsubscribe of unsubscribers) unsubscribe();
  });
}
