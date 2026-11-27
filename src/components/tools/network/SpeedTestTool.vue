<script setup lang="ts">
import { Activity, ArrowDown, ArrowUp, Gauge, Timer } from "@lucide/vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolPanelFrame from "@/components/tools/ToolPanelFrame.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc } from "@/lib/ipc";
import { toolErrorText } from "@/lib/tool-error";
import { useToolboxStore } from "@/stores/toolbox";
import type { SpeedTestResult } from "@/types/tools";

const { t } = useI18n();
const toolbox = useToolboxStore();
const downloadMb = toolbox.toolPref("network.speedtest.downloadMb", "25");
const uploadMb = toolbox.toolPref("network.speedtest.uploadMb", "10");
const samples = toolbox.toolPref("network.speedtest.samples", "8");
const busy = ref(false);
const error = ref<string | null>(null);
const result = ref<SpeedTestResult | null>(null);

const quality = computed(() => {
  if (!result.value) return null;
  const latency = result.value.latencyMs;
  if (latency < 30 && result.value.jitterMs < 10) return "excellent";
  if (latency < 80 && result.value.jitterMs < 25) return "good";
  return "fair";
});

async function run(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  result.value = null;
  try {
    result.value = await ipc.toolSpeedtestRun({
      downloadBytes: Math.round(
        Math.min(200, Math.max(1, Number(downloadMb.value) || 25)) * 1_000_000,
      ),
      uploadBytes: Math.round(
        Math.min(100, Math.max(0.5, Number(uploadMb.value) || 10)) * 1_000_000,
      ),
      pingSamples: Math.min(20, Math.max(3, Number(samples.value) || 8)),
    });
  } catch (cause: unknown) {
    error.value = toolErrorText(t, cause);
  } finally {
    busy.value = false;
  }
}

function formatMbps(value: number): string {
  return value >= 100 ? value.toFixed(0) : value.toFixed(1);
}
</script>

<template>
  <ToolPanelFrame
    :title="t('networkTools.tools.speedtest.title')"
    :description="t('networkTools.tools.speedtest.description')"
    :icon="Gauge"
  >
    <div class="mx-auto max-w-[980px] space-y-4">
      <div
        class="flex flex-wrap items-end gap-3 rounded-xl border border-hairline bg-well-soft p-3"
      >
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.speedtest.downloadSize")
          }}</Label>
          <Input
            v-model="downloadMb"
            class="numeric h-9 w-24 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.speedtest.uploadSize")
          }}</Label>
          <Input
            v-model="uploadMb"
            class="numeric h-9 w-24 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{
            t("networkTools.speedtest.samples")
          }}</Label>
          <Input
            v-model="samples"
            class="numeric h-9 w-20 rounded-lg border-hairline-strong bg-well text-[12px]"
          />
        </div>
        <p class="max-w-[300px] text-[10px] leading-relaxed text-muted-foreground">
          {{ t("networkTools.speedtest.note") }}
        </p>
        <Button size="sm" class="ml-auto h-9 gap-1.5 text-[12px]" :disabled="busy" @click="run">
          <Activity class="size-3.5" :class="{ 'animate-pulse': busy }" />
          {{ busy ? t("networkTools.speedtest.running") : t("networkTools.speedtest.start") }}
        </Button>
      </div>

      <p
        v-if="error"
        class="rounded-lg border border-error/25 bg-error/10 px-3 py-2 text-[11px] text-error"
      >
        {{ error }}
      </p>

      <div v-if="result" class="space-y-3">
        <div class="grid gap-3 sm:grid-cols-2">
          <div class="relative overflow-hidden rounded-xl border border-rx/25 bg-rx/[0.06] p-4">
            <ArrowDown class="absolute right-3 top-3 size-5 text-rx/40" />
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.speedtest.download") }}
            </p>
            <p class="numeric mt-1 text-[36px] font-semibold leading-none tracking-tight text-rx">
              {{ formatMbps(result.downloadMbps) }}
              <span class="text-[13px] font-normal text-muted-foreground">Mbps</span>
            </p>
          </div>
          <div class="relative overflow-hidden rounded-xl border border-sim/25 bg-sim/[0.06] p-4">
            <ArrowUp class="absolute right-3 top-3 size-5 text-sim/40" />
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.speedtest.upload") }}
            </p>
            <p class="numeric mt-1 text-[36px] font-semibold leading-none tracking-tight text-sim">
              {{ formatMbps(result.uploadMbps) }}
              <span class="text-[13px] font-normal text-muted-foreground">Mbps</span>
            </p>
          </div>
        </div>

        <div class="grid gap-2 sm:grid-cols-3">
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p
              class="flex items-center gap-1 text-[10px] uppercase tracking-wider text-muted-foreground"
            >
              <Timer class="size-3" />
              {{ t("networkTools.speedtest.latency") }}
            </p>
            <p class="numeric mt-1 text-[20px] font-semibold">{{ result.latencyMs }} ms</p>
          </div>
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.speedtest.jitter") }}
            </p>
            <p class="numeric mt-1 text-[20px] font-semibold">{{ result.jitterMs }} ms</p>
          </div>
          <div class="rounded-xl border border-hairline bg-well-soft p-3">
            <p class="text-[10px] uppercase tracking-wider text-muted-foreground">
              {{ t("networkTools.speedtest.quality") }}
            </p>
            <p class="mt-1 text-[20px] font-semibold capitalize">
              {{ quality ? t(`networkTools.speedtest.levels.${quality}`) : "—" }}
            </p>
          </div>
        </div>

        <p class="numeric text-right text-[10.5px] text-muted-foreground">
          {{ result.server }} · {{ result.elapsedMs }} ms
        </p>
      </div>

      <div
        v-else-if="!busy && !error"
        class="rounded-xl border border-dashed border-hairline px-4 py-10 text-center text-[11px] text-muted-foreground"
      >
        {{ t("networkTools.speedtest.hint") }}
      </div>
    </div>
  </ToolPanelFrame>
</template>
