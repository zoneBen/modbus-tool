<script setup lang="ts">
import { BellOff, BellRing, Download } from "@lucide/vue";
import { computed, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import { Button } from "@/components/ui/button";
import { formatAddress, formatTime } from "@/lib/format";
import { ipc } from "@/lib/ipc";
import { useAlertsStore } from "@/stores/alerts";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { AlertOperator, AlertSeverity } from "@/types/models";

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

const severityTone: Record<AlertSeverity, string> = {
  info: "border-rx/25 bg-rx/10 text-rx",
  warning: "border-warn/30 bg-warn/10 text-warn",
  critical: "border-error/30 bg-error/10 text-error",
};

function severityLabel(severity: AlertSeverity): string {
  if (severity === "info") return t("alerts.severityInfo");
  if (severity === "critical") return t("alerts.severityCritical");
  return t("alerts.severityWarning");
}

const sessionRecords = computed(() => {
  const sessionId = sessions.activeSessionId;
  if (!sessionId) return [];
  return alerts.records.filter((record) => record.sessionId === sessionId).slice(0, 300);
});

const sessionActiveCount = computed(() => {
  const sessionId = sessions.activeSessionId;
  if (!sessionId) return 0;
  const active = new Set<string>();
  for (const record of [...alerts.records].reverse()) {
    if (record.sessionId !== sessionId) continue;
    const key = `${record.definitionId}:${record.address}:${record.ruleId}`;
    if (record.triggered) active.add(key);
    else active.delete(key);
  }
  return active.size;
});

function recordCount(sessionId: string): number {
  return alerts.records.filter((record) => record.sessionId === sessionId).length;
}

function activeCountFor(sessionId: string): number {
  const active = new Set<string>();
  for (const record of [...alerts.records].reverse()) {
    if (record.sessionId !== sessionId) continue;
    const key = `${record.definitionId}:${record.address}:${record.ruleId}`;
    if (record.triggered) active.add(key);
    else active.delete(key);
  }
  return active.size;
}

onMounted(() => {
  if (!sessions.activeSessionId && sessions.sessions[0]) {
    sessions.setActiveSession(sessions.sessions[0].id);
  }
  alerts.markSeen();
});

watch(
  () => sessions.activeSessionId,
  () => alerts.markSeen(),
);

watch(
  () => alerts.records.length,
  () => alerts.markSeen(),
);

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportAlerts(): Promise<void> {
  const records = sessionRecords.value;
  if (!records.length) {
    toast.info(t("alerts.empty"));
    return;
  }
  const header = [
    "time",
    "state",
    "severity",
    "description",
    "session",
    "definition",
    "address",
    "meaning",
    "operator",
    "threshold",
    "value",
    "unit",
    "delayMs",
    "recoveryOperator",
    "recoveryThreshold",
    "recoveryDelayMs",
  ];
  const lines = [header.map(csvCell).join(",")];
  const sessionName = sessions.activeSession?.name ?? "";
  for (const record of records) {
    lines.push(
      [
        new Date(record.timestamp).toISOString(),
        record.triggered ? "triggered" : "cleared",
        record.severity,
        record.description,
        sessionName,
        record.definitionName,
        record.address,
        record.meaning,
        operatorSymbol[record.operator],
        record.threshold,
        record.value,
        record.unit,
        record.delayMs,
        record.recoveryOperator ? operatorSymbol[record.recoveryOperator] : "",
        record.recoveryThreshold ?? "",
        record.recoveryDelayMs ?? "",
      ]
        .map(csvCell)
        .join(","),
    );
  }
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const scope = sessionName.replace(/[\\/:*?"<>|\s]+/g, "_") || "session";
  try {
    const saved = await ipc.saveTextFile(
      `modbus-alerts-${scope}-${stamp}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) toast.success(t("alerts.exported", { count: records.length }));
  } catch (error: unknown) {
    toast.error(t("alerts.exportFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1">
    <aside class="flex h-full w-[268px] shrink-0 flex-col border-r border-hairline bg-overlay-1">
      <div class="flex h-[52px] shrink-0 items-center justify-between px-4">
        <div class="min-w-0">
          <p class="text-[13px] font-semibold tracking-tight">{{ t("alerts.title") }}</p>
          <p class="text-[11px] text-muted-foreground">
            {{ t("alerts.active", { count: alerts.activeCount }) }}
          </p>
        </div>
        <span
          v-if="alerts.activeCount"
          class="rounded-full border border-warn/25 bg-warn/10 px-2 py-1 text-[10px] text-warn"
        >
          {{ alerts.activeCount }}
        </span>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-2.5 pb-3">
        <div v-if="sessions.sessions.length" class="space-y-1.5">
          <button
            v-for="session in sessions.sessions"
            :key="session.id"
            type="button"
            class="group relative w-full rounded-xl border px-3 py-2.5 text-left transition-colors"
            :class="
              session.id === sessions.activeSessionId
                ? 'border-hairline-strong bg-overlay-2 shadow-lg shadow-black/20'
                : 'border-transparent hover:border-hairline hover:bg-overlay-1'
            "
            @click="sessions.setActiveSession(session.id)"
          >
            <span
              v-if="session.id === sessions.activeSessionId"
              class="absolute inset-y-3 left-0 w-[2px] rounded-full bg-warn"
            />
            <div class="flex items-start gap-2">
              <span
                class="mt-1 size-1.5 shrink-0 rounded-full"
                :class="
                  session.status === 'connected'
                    ? 'status-dot status-dot-live text-signal'
                    : 'bg-muted-foreground/40'
                "
              />
              <div class="min-w-0 flex-1">
                <p class="truncate text-[12px] font-medium">{{ session.name }}</p>
                <p class="numeric mt-1 truncate text-[10.5px] text-muted-foreground">
                  {{ session.endpoint }}
                </p>
                <div class="mt-1 flex items-center gap-2 text-[10px] text-muted-foreground/80">
                  <span>{{ t("alerts.sessionRecords", { count: recordCount(session.id) }) }}</span>
                  <span
                    v-if="activeCountFor(session.id)"
                    class="rounded border border-warn/25 bg-warn/10 px-1 py-0.5 text-warn"
                  >
                    {{ t("alerts.sessionActive", { count: activeCountFor(session.id) }) }}
                  </span>
                </div>
              </div>
            </div>
          </button>
        </div>

        <div v-else class="mt-8 flex flex-col items-center gap-2 px-3 text-center">
          <span
            class="flex size-10 items-center justify-center rounded-xl border border-warn/20 bg-warn/10 text-warn"
          >
            <BellRing class="size-4" />
          </span>
          <p class="text-[12px] text-muted-foreground">{{ t("alerts.noSessions") }}</p>
          <p class="text-[10.5px] leading-relaxed text-muted-foreground/75">
            {{ t("alerts.noSessionsHint") }}
          </p>
        </div>
      </div>
    </aside>

    <div v-if="sessions.activeSession" class="flex min-h-0 min-w-0 flex-1 flex-col">
      <header class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
        <span
          class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-warn/25 bg-warn/10 text-warn"
        >
          <BellRing class="size-4" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="truncate text-[13px] font-semibold">
            {{ t("alerts.recordsFor", { name: sessions.activeSession.name }) }}
          </p>
          <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">
            {{ sessions.activeSession.endpoint }} · {{ t("alerts.subtitle") }}
          </p>
        </div>
        <span
          v-if="sessionActiveCount"
          class="shrink-0 rounded-full border border-warn/25 bg-warn/10 px-2 py-1 text-[10.5px] text-warn"
        >
          {{ t("alerts.active", { count: sessionActiveCount }) }}
        </span>
        <span class="shrink-0 text-[10.5px] text-muted-foreground">
          {{ t("alerts.records", { count: sessionRecords.length }) }}
        </span>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 shrink-0 gap-1 text-[10.5px] text-muted-foreground"
          :disabled="!sessionRecords.length"
          @click="exportAlerts"
        >
          <Download class="size-3" />
          {{ t("alerts.export") }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-8 shrink-0 gap-1 text-[10.5px] text-muted-foreground hover:text-error"
          :disabled="!sessionRecords.length"
          @click="alerts.clearSession(sessions.activeSessionId ?? '')"
        >
          <BellOff class="size-3" />
          {{ t("alerts.clearSession") }}
        </Button>
      </header>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-auto px-4 py-3">
        <div v-if="sessionRecords.length" class="space-y-2">
          <div
            v-for="record in sessionRecords"
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
              <span
                class="mt-0.5 inline-flex shrink-0 items-center rounded border px-1 py-0.5 text-[9.5px] font-medium"
                :class="severityTone[record.severity]"
              >
                {{ severityLabel(record.severity) }}
              </span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-[12px] font-medium text-foreground">
                  {{
                    record.description ||
                    record.meaning ||
                    t("alerts.register", { address: record.address })
                  }}
                </p>
                <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">
                  <span v-if="record.meaning">{{ record.meaning }} · </span>
                  <span class="numeric"
                    >@{{ formatAddress(record.address, settings.addressBase) }}</span
                  >
                  · {{ record.definitionName }}
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
              <span
                v-if="record.recoveryOperator && record.recoveryThreshold !== undefined"
                class="numeric rounded border border-signal/20 bg-signal/5 px-2 py-0.5 text-signal/90"
              >
                {{
                  t("alerts.recoveryCondition", {
                    operator: operatorSymbol[record.recoveryOperator],
                    value: record.recoveryThreshold,
                    unit: record.unit ? ` ${record.unit}` : "",
                  })
                }}
                <span v-if="record.recoveryDelayMs !== undefined" class="ml-1 text-signal/70">
                  · {{ t("alerts.recoveryDelay", { delay: record.recoveryDelayMs }) }}
                </span>
              </span>
              <span class="text-muted-foreground/70">
                {{ t("alerts.delay", { delay: record.delayMs }) }}
              </span>
            </div>
          </div>
        </div>
        <EmptyState
          v-else
          :title="t('alerts.noSessionAlerts')"
          :description="t('alerts.noSessionAlertsHint')"
          class="h-full"
        >
          <template #icon>
            <BellRing class="size-4" />
          </template>
        </EmptyState>
      </div>
    </div>

    <EmptyState
      v-else
      :title="t('alerts.selectSession')"
      :description="t('alerts.selectSessionHint')"
      class="m-auto flex-1"
    >
      <template #icon>
        <BellRing class="size-5" />
      </template>
    </EmptyState>
  </div>
</template>
