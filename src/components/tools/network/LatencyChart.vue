<script setup lang="ts">
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import type { LatencySample } from "@/types/tools";

const props = defineProps<{
  samples: LatencySample[];
}>();

const settings = useSettingsStore();
const { t } = useI18n();
const container = ref<HTMLElement | null>(null);
const tooltip = ref<{
  visible: boolean;
  left: number;
  top: number;
  time: string;
  latency: string;
  status: string;
}>({ visible: false, left: 0, top: 0, time: "", latency: "—", status: "" });

interface Palette {
  surface: string;
  axis: string;
  text: string;
  grid: string;
  tick: string;
  labelBackground: string;
  latency: string;
  loss: string;
}

function cssColor(variable: string, fallback: string): string {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(variable).trim();
  return value || fallback;
}

function currentPalette(): Palette {
  return {
    surface: cssColor("--surface", "#1c2129"),
    axis: cssColor("--muted-foreground", "#b9bcc4"),
    text: cssColor("--foreground", "#f0f2f5"),
    grid: cssColor("--grid-line", "rgba(255,255,255,0.05)"),
    tick: cssColor("--separator", "rgba(255,255,255,0.08)"),
    labelBackground: cssColor("--popover", "#252a33"),
    latency: cssColor("--rx", "#6ea8fe"),
    loss: cssColor("--error", "#f87171"),
  };
}

const palette = ref<Palette>(currentPalette());
let plot: uPlot | null = null;
let resizeObserver: ResizeObserver | null = null;

function shortTime(timestamp: number): string {
  const date = new Date(timestamp);
  return `${date.getHours().toString().padStart(2, "0")}:${date
    .getMinutes()
    .toString()
    .padStart(2, "0")}:${date.getSeconds().toString().padStart(2, "0")}`;
}

function preciseTime(timestamp: number): string {
  const date = new Date(timestamp);
  return `${shortTime(timestamp)}.${date.getMilliseconds().toString().padStart(3, "0")}`;
}

function axisTime(timestamp: number, includeMillis: boolean): string {
  const date = new Date(timestamp);
  return includeMillis
    ? `${shortTime(timestamp)}.${date.getMilliseconds().toString().padStart(3, "0")}`
    : shortTime(timestamp);
}

function formatMs(value: number): string {
  return `${new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 }).format(value)} ms`;
}

function updateCursor(self: uPlot): void {
  const index = self.cursor.idx;
  if (index === null || index === undefined || !props.samples[index]) {
    tooltip.value.visible = false;
    return;
  }
  const sample = props.samples[index];
  const left = Number(self.cursor.left ?? 0);
  const top = Number(self.cursor.top ?? 0);
  const statusKey = `networkTools.latency.statuses.${sample.status}`;
  const statusText = t(statusKey);
  tooltip.value = {
    visible: true,
    left: Math.min(Math.max(left + 14, 8), Math.max(8, (self.width || 400) - 200)),
    top: Math.max(top - 10, 8),
    time: preciseTime(sample.timestampMs),
    latency: sample.rttMs === undefined || sample.rttMs === null ? "—" : formatMs(sample.rttMs),
    status: statusText === statusKey ? sample.status : statusText,
  };
}

function drawLossBands(self: uPlot): void {
  const values = self.data[1];
  if (!values) return;
  const colors = palette.value;
  const top = self.bbox.top;
  const height = self.bbox.height;
  self.ctx.save();
  self.ctx.globalAlpha = 0.16;
  self.ctx.fillStyle = colors.loss;
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value !== null && value !== undefined) continue;
    const timestamp = self.data[0]?.[index];
    if (typeof timestamp !== "number") continue;
    const x = self.valToPos(timestamp, "x", true);
    if (!Number.isFinite(x)) continue;
    self.ctx.fillRect(x - 1.5, top, 3, height);
  }
  self.ctx.globalAlpha = 0.9;
  self.ctx.fillStyle = colors.loss;
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] !== null && values[index] !== undefined) continue;
    const timestamp = self.data[0]?.[index];
    if (typeof timestamp !== "number") continue;
    const x = self.valToPos(timestamp, "x", true);
    if (!Number.isFinite(x)) continue;
    self.ctx.beginPath();
    self.ctx.arc(x, top + height - 3, 2, 0, Math.PI * 2);
    self.ctx.fill();
  }
  self.ctx.restore();
}

function buildOptions(width: number, height: number): uPlot.Options {
  const colors = palette.value;
  return {
    width,
    height,
    padding: [10, 12, 2, 0],
    cursor: {
      show: true,
      x: true,
      y: false,
      drag: { setScale: false },
      points: { show: true, size: 7, width: 2, stroke: colors.surface },
    },
    scales: { x: { time: true }, y: { auto: true } },
    axes: [
      {
        stroke: colors.axis,
        font: "11px 'JetBrains Mono Variable', monospace",
        grid: { stroke: colors.grid, width: 1 },
        ticks: { stroke: colors.tick, width: 1 },
        space: 110,
        values: (self, splits) => {
          const min = self.scales.x.min ?? 0;
          const max = self.scales.x.max ?? 0;
          const includeMillis = max - min < 30;
          return splits.map((value) => axisTime(value * 1000, includeMillis));
        },
      },
      {
        stroke: colors.axis,
        font: "11px 'JetBrains Mono Variable', monospace",
        grid: { stroke: colors.grid, width: 1 },
        ticks: { show: false },
        size: 54,
        values: (_self, splits) =>
          splits.map((value) =>
            new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 }).format(value),
          ),
      },
    ],
    series: [
      { label: t("chart.time") },
      {
        label: t("networkTools.latency.rtt"),
        stroke: colors.latency,
        width: 2,
        spanGaps: false,
        points: { show: false },
      },
    ],
    legend: { show: false },
    hooks: {
      setCursor: [updateCursor],
      draw: [drawLossBands],
    },
  };
}

function destroyPlot(): void {
  plot?.destroy();
  plot = null;
  tooltip.value.visible = false;
}

function createPlot(): void {
  destroyPlot();
  if (!container.value) return;
  const width = Math.max(container.value.clientWidth, 80);
  const height = Math.max(container.value.clientHeight, 80);
  const data: uPlot.AlignedData = [
    props.samples.map((sample) => sample.timestampMs / 1000),
    props.samples.map((sample) => sample.rttMs ?? null),
  ];
  plot = new uPlot(buildOptions(width, height), data, container.value);
}

function updateData(): void {
  if (!plot) return;
  plot.setData(
    [
      props.samples.map((sample) => sample.timestampMs / 1000),
      props.samples.map((sample) => sample.rttMs ?? null),
    ] as uPlot.AlignedData,
    true,
  );
}

watch(
  () => props.samples,
  () => updateData(),
  { deep: false },
);
watch(
  () => settings.resolvedTheme,
  async () => {
    await nextTick();
    palette.value = currentPalette();
    createPlot();
  },
);

onMounted(() => {
  createPlot();
  if (container.value) {
    resizeObserver = new ResizeObserver(() => {
      if (!plot || !container.value) return;
      plot.setSize({
        width: Math.max(container.value.clientWidth, 80),
        height: Math.max(container.value.clientHeight, 80),
      });
    });
    resizeObserver.observe(container.value);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  destroyPlot();
});
</script>

<template>
  <div ref="container" class="relative h-full w-full overflow-hidden">
    <div
      v-show="tooltip.visible"
      class="glass-strong pointer-events-none absolute z-20 w-[196px] rounded-lg px-2.5 py-2 shadow-xl shadow-black/25"
      :style="{ left: `${tooltip.left}px`, top: `${tooltip.top}px` }"
    >
      <p class="numeric text-[11px] text-muted-foreground">{{ tooltip.time }}</p>
      <div class="mt-1.5 flex items-center justify-between gap-2">
        <span class="text-[11px] text-muted-foreground">{{ t("networkTools.latency.rtt") }}</span>
        <span class="numeric text-[11px] text-foreground">{{ tooltip.latency }}</span>
      </div>
      <div class="mt-0.5 flex items-center justify-between gap-2">
        <span class="text-[11px] text-muted-foreground">{{
          t("networkTools.latency.statusLabel")
        }}</span>
        <span class="text-[11px] text-foreground">{{ tooltip.status }}</span>
      </div>
    </div>
  </div>
</template>
