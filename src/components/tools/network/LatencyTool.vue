<script setup lang="ts">
import { Activity, Download, Square } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { useToolEventBuffer } from "@/composables/useToolEventBuffer";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import LatencyChart from "@/components/tools/network/LatencyChart.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { LatencyDone, LatencySample, ToolEvent } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const host = toolbox.toolPref("network.latency.host", "8.8.8.8");
const intervalMs = toolbox.toolPref("network.latency.interval", "1000");
const timeoutMs = toolbox.toolPref("network.latency.timeout", "1000");
const payloadSize = toolbox.toolPref("network.latency.payload", "32");
const maxPoints = toolbox.toolPref("network.latency.maxPoints", "600");
const running = ref(false);
const samples = ref<LatencySample[]>([]);
const done = ref<LatencyDone | null>(null);
const error = ref<string | null>(null);

const stats = computed(() => {
  const source = samples.value;
  const received = source.filter((sample) => typeof sample.rttMs === "number");
  const rtts = received.map((sample) => sample.rttMs as number);
  const sent = running.value || !done.value ? source.length : done.value.sent;
  const recv = running.value || !done.value ? received.length : done.value.received;
  const loss = sent ? Math.round(((sent - recv) / sent) * 1000) / 10 : 0;
  let jitter = 0;
  if (rtts.length > 1) {
    let total = 0;
    for (let index = 1; index < rtts.length; index += 1) {
      total += Math.abs(rtts[index] - rtts[index - 1]);
    }
    jitter = total / (rtts.length - 1);
  }
  const round = (value: number): number => Math.round(value * 10) / 10;
  const last = received[received.length - 1]?.rttMs;
  const finished = !running.value && done.value ? done.value : null;
  return {
    sent,
    received: recv,
    loss,
    min: finished?.minMs ?? (rtts.length ? round(Math.min(...rtts)) : null),
    max: finished?.maxMs ?? (rtts.length ? round(Math.max(...rtts)) : null),
    avg:
      finished?.avgMs ??
      (rtts.length ? round(rtts.reduce((sum, value) => sum + value, 0) / rtts.length) : null),
    jitter: jitter ? round(jitter) : null,
    current: typeof last === "number" ? round(last) : null,
    address: source[source.length - 1]?.address ?? "",
  };
});

const chartHeight = "h-[330px]";

function onEvent(event: ToolEvent): void {
  if (event.event === "latencySample") {
    const limit = Math.min(5000, Math.max(60, Number(maxPoints.value) || 600));
    const next = [...samples.value, event.sample];
    samples.value = next.length > limit ? next.slice(next.length - limit) : next;
  } else if (event.event === "latencyDone") {
    running.value = false;
    unbind();
    done.value = event;
    if (event.error && event.error.code !== "cancelled") {
      error.value = toolErrorText(t, event.error);
    }
  }
}

const { taskId, bind, unbind } = useToolEventBuffer(onEvent);

async function start(): Promise<void> {
  if (running.value) return;
  if (!host.value.trim()) {
    error.value = t("tools.errors.invalidHost");
    return;
  }
  error.value = null;
  samples.value = [];
  done.value = null;
  running.value = true;
  try {
    bind(
      await ipc.toolLatencyStart({
        host: host.value.trim(),
        intervalMs: Math.min(60_000, Math.max(200, Number(intervalMs.value) || 1000)),
        timeoutMs: Math.min(10_000, Math.max(100, Number(timeoutMs.value) || 1000)),
        payloadSize: Math.min(1400, Math.max(8, Number(payloadSize.value) || 32)),
      }),
    );
  } catch (cause: unknown) {
    running.value = false;
    unbind();
    error.value = toolErrorText(t, cause);
  }
}

async function stop(): Promise<void> {
  const current = taskId.value;
  if (current) await ipc.toolTaskCancel(current);
}

function clear(): void {
  samples.value = [];
  done.value = null;
  error.value = null;
}

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportCsv(): Promise<void> {
  if (!samples.value.length) {
    toast.info(t("networkTools.latency.exportEmpty"));
    return;
  }
  const lines = ["seq,timestamp,status,rttMs,address"];
  for (const sample of samples.value) {
    lines.push(
      [
        sample.seq,
        new Date(sample.timestampMs).toISOString(),
        sample.status,
        sample.rttMs ?? "",
        sample.address,
      ]
        .map(csvCell)
        .join(","),
    );
  }
  try {
    const saved = await ipc.saveTextFile(
      `modbus-tool-latency-${new Date().toISOString().replace(/[:.]/g, "-")}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) toast.success(t("networkTools.latency.exported", { count: samples.value.length }));
  } catch (cause: unknown) {
    toast.error(t("networkTools.latency.exportFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
}

const statCards = computed(() => [
  { label: t("networkTools.latency.current"), value: stats.value.current, tone: "text-rx" },
  { label: t("networkTools.latency.avg"), value: stats.value.avg, tone: "text-foreground" },
  { label: t("networkTools.latency.min"), value: stats.value.min, tone: "text-muted-foreground" },
  { label: t("networkTools.latency.max"), value: stats.value.max, tone: "text-muted-foreground" },
  { label: t("networkTools.latency.jitter"), value: stats.value.jitter, tone: "text-sim" },
]);
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.latency.title')"
    :description="t('networkTools.tools.latency.description')"
    :icon="Activity"
  >
    <div class="mx-auto max-w-[1180px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-7">
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.latency.host")
            }}</Label>
            <Input
              v-model="host"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              placeholder="8.8.8.8"
              @keydown.enter="!running && start()"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.latency.interval")
            }}</Label>
            <Input
              v-model="intervalMs"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.latency.timeout")
            }}</Label>
            <Input
              v-model="timeoutMs"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.latency.payload")
            }}</Label>
            <Input
              v-model="payloadSize"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.latency.maxPoints")
            }}</Label>
            <Input
              v-model="maxPoints"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="flex items-end gap-1.5">
            <Button v-if="!running" size="sm" class="h-9 flex-1 gap-1.5 text-[12px]" @click="start">
              <Activity class="size-3.5" />
              {{ t("networkTools.latency.start") }}
            </Button>
            <Button
              v-else
              variant="outline"
              size="sm"
              class="h-9 flex-1 gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
              @click="stop"
            >
              <Square class="size-3" />
              {{ t("networkTools.latency.stop") }}
            </Button>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            class="h-7 gap-1 text-[11px] text-muted-foreground"
            :disabled="!samples.length"
            @click="clear"
          >
            {{ t("common.clear") }}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-7 gap-1 text-[11px] text-muted-foreground"
            :disabled="!samples.length"
            @click="exportCsv"
          >
            <Download class="size-3" />
            {{ t("networkTools.latency.export") }}
          </Button>
          <span class="numeric ml-auto text-[10.5px] text-muted-foreground">
            {{ stats.address || "—" }} · {{ stats.received }}/{{ stats.sent }}
            {{ t("networkTools.latency.received") }} · {{ stats.loss }}%
            {{ t("networkTools.latency.loss") }}
          </span>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div class="grid grid-cols-2 gap-2 md:grid-cols-5">
        <div
          v-for="card in statCards"
          :key="card.label"
          class="rounded-xl border border-hairline bg-well-soft px-3 py-2"
        >
          <p class="text-[10px] text-muted-foreground">{{ card.label }}</p>
          <p class="numeric mt-1 text-[17px] font-medium" :class="card.tone">
            {{ card.value === null ? "—" : card.value }}
            <span v-if="card.value !== null" class="text-[10px] text-muted-foreground">ms</span>
          </p>
        </div>
      </div>

      <div class="rounded-xl border border-hairline bg-well-soft">
        <div class="flex flex-wrap items-center gap-2 border-b border-hairline px-3 py-2">
          <span class="text-[11px] font-medium">{{ t("networkTools.latency.timeline") }}</span>
          <span class="text-[10.5px] text-muted-foreground">{{
            t("networkTools.latency.lossHint")
          }}</span>
          <span
            v-if="running"
            class="status-dot status-dot-live ml-auto scale-75 text-signal"
            aria-hidden="true"
          />
          <span v-if="running" class="text-[10.5px] text-signal">{{
            t("networkTools.latency.running")
          }}</span>
          <span
            v-else-if="samples.length"
            class="ml-auto numeric text-[10.5px] text-muted-foreground"
          >
            {{ samples.length }} / {{ maxPoints }}
          </span>
        </div>
        <div :class="chartHeight" class="px-2 pb-1 pt-2">
          <LatencyChart v-if="samples.length" :samples="samples" />
          <div
            v-else
            class="flex h-full items-center justify-center rounded-lg border border-dashed border-hairline text-[11px] text-muted-foreground"
          >
            {{ t("networkTools.latency.hint") }}
          </div>
        </div>
      </div>
    </div>
  </ToolPanelFrame>
</template>
