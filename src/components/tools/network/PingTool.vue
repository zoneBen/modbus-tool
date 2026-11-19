<script setup lang="ts">
import { Activity, Download, Square } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { useToolboxStore } from "@/stores/toolbox";
import { toolErrorText } from "@/lib/tool-error";
import type { PingHostResult, ToolErrorPayload } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const targets = toolbox.toolPref("network.ping.targets", "192.168.1.0/24");
const count = toolbox.toolPref("network.ping.count", "3");
const intervalMs = toolbox.toolPref("network.ping.interval", "200");
const timeoutMs = toolbox.toolPref("network.ping.timeout", "1000");
const concurrency = toolbox.toolPref("network.ping.concurrency", "16");
const payloadSize = toolbox.toolPref("network.ping.payload", "32");
const running = ref(false);
const taskId = ref<string | null>(null);
const progress = ref({ done: 0, total: 0 });
const results = ref<PingHostResult[]>([]);
const error = ref<string | null>(null);
let unlisten: (() => void) | null = null;

const percent = computed(() =>
  progress.value.total > 0
    ? Math.min(100, Math.round((progress.value.done / progress.value.total) * 100))
    : 0,
);
const okCount = computed(() => results.value.filter((result) => result.status === "ok").length);

onMounted(async () => {
  unlisten = await ipc.on("tool:event", (event) => {
    if (!taskId.value || event.taskId !== taskId.value) return;
    if (event.event === "pingProgress") {
      progress.value = { done: event.done, total: event.total };
    } else if (event.event === "pingResult") {
      results.value = [...results.value, event.result];
    } else if (event.event === "pingDone") {
      running.value = false;
      taskId.value = null;
      if (event.error) {
        error.value = toolErrorText(t, event.error);
      }
    }
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  unlisten = null;
});

async function start(): Promise<void> {
  if (running.value) return;
  error.value = null;
  results.value = [];
  progress.value = { done: 0, total: 0 };
  running.value = true;
  try {
    taskId.value = await ipc.toolPingStart({
      targets: targets.value,
      count: Math.min(10, Math.max(1, Number(count.value) || 3)),
      intervalMs: Math.min(5000, Math.max(100, Number(intervalMs.value) || 200)),
      timeoutMs: Math.min(5000, Math.max(100, Number(timeoutMs.value) || 1000)),
      concurrency: Math.min(32, Math.max(1, Number(concurrency.value) || 16)),
      payloadSize: Math.min(1400, Math.max(8, Number(payloadSize.value) || 32)),
    });
  } catch (cause: unknown) {
    running.value = false;
    error.value = toolErrorText(t, cause);
  }
}

async function cancel(): Promise<void> {
  const current = taskId.value;
  if (!current) return;
  await ipc.toolTaskCancel(current);
}

function errorText(error: ToolErrorPayload | undefined): string {
  if (!error) return "";
  const key = `tools.errors.${error.code}`;
  const translated = t(key);
  return translated === key ? error.detail : translated;
}

function csvCell(value: unknown): string {
  return `"${String(value ?? "").replace(/"/g, '""')}"`;
}

async function exportCsv(): Promise<void> {
  if (!results.value.length) {
    toast.info(t("networkTools.ping.exportEmpty"));
    return;
  }
  const header = [
    "host",
    "address",
    "sent",
    "received",
    "lossPercent",
    "minMs",
    "avgMs",
    "maxMs",
    "status",
  ];
  const lines = [header.map(csvCell).join(",")];
  for (const result of results.value) {
    lines.push(
      [
        result.host,
        result.address ?? "",
        result.sent,
        result.received,
        result.lossPercent,
        result.minMs ?? "",
        result.avgMs ?? "",
        result.maxMs ?? "",
        result.status,
      ]
        .map(csvCell)
        .join(","),
    );
  }
  try {
    const saved = await ipc.saveTextFile(
      `modbus-tool-ping-${new Date().toISOString().replace(/[:.]/g, "-")}.csv`,
      `${String.fromCharCode(0xfeff)}${lines.join("\r\n")}`,
    );
    if (saved) toast.success(t("networkTools.ping.exported", { count: results.value.length }));
  } catch (cause: unknown) {
    toast.error(t("networkTools.ping.exportFailed"), {
      description: cause instanceof Error ? cause.message : String(cause),
    });
  }
}

const statusTone: Record<string, string> = {
  ok: "border-signal/25 bg-signal/10 text-signal",
  timeout: "border-warn/25 bg-warn/10 text-warn",
  error: "border-error/25 bg-error/10 text-error",
  cancelled: "border-hairline bg-overlay-1 text-muted-foreground",
};
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.ping.title')"
    :description="t('networkTools.tools.ping.description')"
    :icon="Activity"
  >
    <div class="mx-auto max-w-[1120px] space-y-4">
      <div class="space-y-3 rounded-xl border border-hairline bg-well-soft p-3">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.ping.targets")
          }}</Label>
          <textarea
            v-model="targets"
            rows="3"
            class="scrollbar-thin numeric w-full resize-y rounded-lg border border-hairline-strong bg-well px-3 py-2 text-[11.5px] text-foreground outline-none transition-colors focus:border-signal/50"
            :placeholder="t('networkTools.ping.targetsPlaceholder')"
          />
        </div>
        <div class="grid grid-cols-2 gap-2 lg:grid-cols-5">
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ping.count")
            }}</Label>
            <Input
              v-model="count"
              class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ping.interval")
            }}</Label>
            <Input
              v-model="intervalMs"
              class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ping.timeout")
            }}</Label>
            <Input
              v-model="timeoutMs"
              class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ping.concurrency")
            }}</Label>
            <Input
              v-model="concurrency"
              class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11.5px]"
            />
          </div>
          <div class="space-y-1">
            <Label class="text-[11px] text-muted-foreground">{{
              t("networkTools.ping.payload")
            }}</Label>
            <Input
              v-model="payloadSize"
              class="numeric h-8 rounded-lg border-hairline-strong bg-well text-[11.5px]"
            />
          </div>
        </div>
        <div class="flex items-center gap-2">
          <Button v-if="!running" size="sm" class="h-8 gap-1.5 text-[12px]" @click="start">
            <Activity class="size-3.5" />
            {{ t("networkTools.ping.start") }}
          </Button>
          <Button
            v-else
            variant="outline"
            size="sm"
            class="h-8 gap-1.5 border-warn/25 bg-warn/10 text-[12px] text-warn"
            @click="cancel"
          >
            <Square class="size-3" />
            {{ t("networkTools.ping.cancel") }}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1 text-[11px] text-muted-foreground"
            :disabled="!results.length"
            @click="exportCsv"
          >
            <Download class="size-3" />
            {{ t("networkTools.ping.export") }}
          </Button>
        </div>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
      >
        {{ error }}
      </p>

      <div v-if="progress.total || results.length" class="rounded-xl border border-hairline">
        <div class="border-b border-hairline px-3 py-2">
          <div class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
            <span>{{ t("networkTools.ping.progress") }}</span>
            <span class="numeric">{{ progress.done }}/{{ progress.total }}</span>
            <span class="ml-auto">{{ t("networkTools.ping.okCount", { count: okCount }) }}</span>
          </div>
          <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-overlay-2">
            <div
              class="h-full rounded-full bg-gradient-to-r from-signal to-rx transition-[width] duration-200"
              :style="{ width: `${percent}%` }"
            />
          </div>
        </div>
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-[11px]">
            <thead class="bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.ping.host") }}
                </th>
                <th class="px-2 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.ping.address") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.sent") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.received") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.loss") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.min") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.avg") }}
                </th>
                <th class="px-2 py-2 text-right font-medium text-muted-foreground">
                  {{ t("networkTools.ping.max") }}
                </th>
                <th class="px-3 py-2 text-left font-medium text-muted-foreground">
                  {{ t("networkTools.ping.statusLabel") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="result in results"
                :key="result.host"
                class="border-b border-hairline last:border-0"
              >
                <td class="numeric px-3 py-1.5 text-foreground">{{ result.host }}</td>
                <td class="numeric px-2 py-1.5 text-muted-foreground">
                  {{ result.address ?? "—" }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.sent }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.received }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.lossPercent }}%
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.minMs ?? "—" }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.avgMs ?? "—" }}
                </td>
                <td class="numeric px-2 py-1.5 text-right text-muted-foreground">
                  {{ result.maxMs ?? "—" }}
                </td>
                <td class="px-3 py-1.5">
                  <span
                    class="rounded border px-1 py-0.5 text-[10px]"
                    :class="statusTone[result.status]"
                  >
                    {{ t(`networkTools.ping.statuses.${result.status}`) }}
                  </span>
                  <span
                    v-if="result.error"
                    class="ml-2 text-[10px] text-muted-foreground"
                    :title="errorText(result.error)"
                  >
                    {{ errorText(result.error) }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div
        v-else-if="!running"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.ping.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
