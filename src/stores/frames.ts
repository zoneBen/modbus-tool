import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { formatHexBytes } from "@/lib/format";
import type { Frame } from "@/types/models";

const FRAME_LIMIT = 400;

export const useFramesStore = defineStore("frames", () => {
  const frames = ref<Frame[]>([]);
  const direction = ref<"all" | "tx" | "rx">("all");
  const search = ref("");
  const selectedSeq = ref<number | null>(null);

  const filteredFrames = computed(() => {
    const query = search.value.trim().toLowerCase();
    return frames.value.filter((frame) => {
      if (direction.value !== "all" && frame.direction !== direction.value) return false;
      if (!query) return true;
      const haystack =
        `${frame.summary} ${formatHexBytes(frame.raw)} ${frame.sessionId}`.toLowerCase();
      return haystack.includes(query);
    });
  });

  const selectedFrame = computed<Frame | null>(() => {
    if (selectedSeq.value === null) return filteredFrames.value.at(-1) ?? null;
    return frames.value.find((frame) => frame.seq === selectedSeq.value) ?? null;
  });

  const stats = computed(() => {
    let tx = 0;
    let rx = 0;
    let errors = 0;
    for (const frame of frames.value) {
      if (frame.direction === "tx") tx += 1;
      else rx += 1;
      if (frame.status !== "ok") errors += 1;
    }
    return { tx, rx, errors, total: frames.value.length };
  });

  function appendFrames(_sessionId: string, incoming: Frame[]): void {
    if (!incoming.length) return;
    const known = new Set(frames.value.map((frame) => frame.seq));
    const fresh = incoming.filter((frame) => !known.has(frame.seq));
    if (!fresh.length) return;
    frames.value = [...frames.value, ...fresh].slice(-FRAME_LIMIT);
  }

  function clearFrames(): void {
    frames.value = [];
    selectedSeq.value = null;
  }

  function selectFrame(seq: number): void {
    selectedSeq.value = seq;
  }

  return {
    frames,
    direction,
    search,
    selectedSeq,
    filteredFrames,
    selectedFrame,
    stats,
    appendFrames,
    clearFrames,
    selectFrame,
  };
});
