import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { AlertEvent } from "@/types/models";

const MAX_RECORDS = 500;

export interface AlertRecord extends AlertEvent {
  id: number;
}

export const useAlertsStore = defineStore("alerts", () => {
  const records = ref<AlertRecord[]>([]);
  const unread = ref(0);
  let nextId = 1;

  const activeCount = computed(() => {
    const active = new Set<string>();
    for (const record of [...records.value].reverse()) {
      const key = `${record.sessionId}:${record.definitionId}:${record.address}:${record.ruleId}`;
      if (record.triggered) active.add(key);
      else active.delete(key);
    }
    return active.size;
  });

  function apply(event: AlertEvent): void {
    const record: AlertRecord = { ...event, id: nextId++ };
    records.value = [record, ...records.value].slice(0, MAX_RECORDS);
    if (event.triggered) unread.value += 1;
  }

  function markSeen(): void {
    unread.value = 0;
  }

  function clear(): void {
    records.value = [];
    unread.value = 0;
  }

  return { records, unread, activeCount, apply, markSeen, clear };
});
