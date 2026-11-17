<script setup lang="ts">
import { BellOff, Download, BellRing } from "@lucide/vue";
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatAddress, formatTime } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { useAlertsStore } from "@/stores/alerts";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { AlertOperator } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const alerts = useAlertsStore();
const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const operatorSymbol: Record<AlertOperator, string> = {
  gt: ">",
  lt: "<",
  gte: "≥",
  lte: "≤",
  eq: "=",
  ne: "≠",
};

const records = computed(() => alerts.records.slice(0, 300));

watch(open, (isOpen) => {
  if (isOpen) alerts.markSeen();
});

function sessionLabel(sessionId: string): string {
  return sessions.sessionName(sessionId);
}

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportAlerts(): Promise<void> {
  if (!alerts.records.length) {
    toast.info(t("alerts.empty"));
    return;
  }
  const header = [
    "time",
    "state",
    "session",
    "definition",
    "address",
    "meaning",
    "operator",
    "threshold",
    "value",
    "unit",
    "delayMs",
  ];
  const lines = [header.map(csvCell).join(",")];
  for (const record of alerts.records) {
    lines.push(
      [
        new Date(record.timestamp).toISOString(),
        record.triggered ? "triggered" : "cleared",
        sessionLabel(record.sessionId),
        record.definitionName,
        record.address,
        record.meaning,
        operatorSymbol[record.operator],
        record.threshold,
        record.value,
        record.unit,
        record.delayMs,
      ]
        .map(csvCell)
        .join(","),
    );
  }
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  try {
    const saved = await ipc.saveTextFile(
      `modbus-alerts-${stamp}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) toast.success(t("alerts.exported", { count: alerts.records.length }));
  } catch (error: unknown) {
    toast.error(t("alerts.exportFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="flex h-[min(700px,84vh)] flex-col gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/30 backdrop-blur-2xl sm:max-w-[820px]"
    >
      <DialogHeader class="shrink-0 border-b border-hairline px-5 py-4">
        <div class="flex items-start gap-3 pr-7">
          <span
            class="flex size-9 shrink-0 items-center justify-center rounded-xl border border-warn/25 bg-warn/10 text-warn"
          >
            <BellRing class="size-4" />
          </span>
          <div class="min-w-0 flex-1">
            <DialogTitle class="text-[14px]">{{ t("alerts.title") }}</DialogTitle>
            <DialogDescription class="mt-0.5 text-[11px] text-muted-foreground">
              {{ t("alerts.subtitle") }}
            </DialogDescription>
          </div>
          <span
            v-if="alerts.activeCount"
            class="shrink-0 rounded-full border border-warn/25 bg-warn/10 px-2 py-1 text-[10.5px] font-medium text-warn"
          >
            {{ t("alerts.active", { count: alerts.activeCount }) }}
          </span>
        </div>
      </DialogHeader>

      <div class="flex shrink-0 items-center gap-2 border-b border-hairline px-4 py-2">
        <span class="text-[11px] text-muted-foreground">
          {{ t("alerts.records", { count: alerts.records.length }) }}
        </span>
        <div class="ml-auto flex items-center gap-1.5">
          <button
            type="button"
            class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
            :title="t('alerts.export')"
            @click="exportAlerts"
          >
            <Download class="size-3" />
            <span class="hidden sm:inline">{{ t("alerts.export") }}</span>
          </button>
          <button
            type="button"
            class="flex items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] text-muted-foreground transition-colors hover:text-error"
            :title="t('alerts.clear')"
            @click="alerts.clear()"
          >
            <BellOff class="size-3" />
            <span class="hidden sm:inline">{{ t("alerts.clear") }}</span>
          </button>
        </div>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-auto px-4 py-3">
        <div v-if="records.length" class="space-y-2">
          <div
            v-for="record in records"
            :key="record.id"
            class="rounded-xl border px-3 py-2.5"
            :class="
              record.triggered
                ? 'border-warn/25 bg-warn/5'
                : 'border-hairline bg-overlay-1 opacity-80'
            "
          >
            <div class="flex items-start gap-2">
              <span
                class="mt-0.5 inline-flex shrink-0 items-center rounded border px-1 py-0.5 text-[9.5px] font-medium"
                :class="
                  record.triggered
                    ? 'border-warn/30 bg-warn/10 text-warn'
                    : 'border-signal/25 bg-signal/10 text-signal'
                "
              >
                {{ record.triggered ? t("alerts.triggered") : t("alerts.cleared") }}
              </span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-[12px] font-medium text-foreground">
                  {{ record.meaning || t("alerts.register", { address: record.address }) }}
                  <span class="ml-1 numeric text-[10.5px] text-muted-foreground">
                    @{{ formatAddress(record.address, settings.addressBase) }}
                  </span>
                </p>
                <p class="mt-0.5 text-[10.5px] text-muted-foreground">
                  {{ sessionLabel(record.sessionId) }} · {{ record.definitionName }}
                </p>
              </div>
              <span class="numeric shrink-0 text-[10.5px] text-muted-foreground">
                {{ formatTime(record.timestamp) }}
              </span>
            </div>
            <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px]">
              <span class="numeric rounded border border-hairline bg-well px-2 py-0.5">
                {{
                  t("alerts.currentValue", {
                    value: record.value,
                    unit: record.unit ? ` ${record.unit}` : "",
                  })
                }}
              </span>
              <span class="numeric text-muted-foreground">
                {{ operatorSymbol[record.operator] }} {{ record.threshold
                }}{{ record.unit ? ` ${record.unit}` : "" }}
              </span>
              <span class="text-muted-foreground/70">
                {{ t("alerts.delay", { delay: record.delayMs }) }}
              </span>
            </div>
          </div>
        </div>
        <EmptyState
          v-else
          :title="t('alerts.empty')"
          :description="t('alerts.emptyHint')"
          class="h-full"
        >
          <template #icon>
            <BellRing class="size-4" />
          </template>
        </EmptyState>
      </div>
    </DialogContent>
  </Dialog>
</template>
