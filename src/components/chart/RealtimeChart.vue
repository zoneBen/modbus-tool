<script setup lang="ts">
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import type { ChartData, ChartSeries } from "@/types/chart";

const props = defineProps<{
  data: ChartData;
  series: ChartSeries[];
}>();

const settings = useSettingsStore();
const { t } = useI18n();

interface ChartPalette {
  surface: string;
  axis: string;
  text: string;
  grid: string;
  tick: string;
  labelBackground: string;
}

function cssColor(variable: string, fallback: string): string {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(variable).trim();
  return value || fallback;
}

function currentPalette(): ChartPalette {
  return {
    surface: cssColor("--surface", "#1c2129"),
    axis: cssColor("--muted-foreground", "#b9bcc4"),
    text: cssColor("--foreground", "#f0f2f5"),
    grid: cssColor("--grid-line", "rgba(255,255,255,0.05)"),
    tick: cssColor("--separator", "rgba(255,255,255,0.08)"),
    labelBackground: cssColor("--popover", "#252a33"),
  };
}

const palette = ref<ChartPalette>(currentPalette());

const container = ref<HTMLElement | null>(null);
const tooltip = ref<{
  visible: boolean;
  left: number;
  top: number;
  time: string;
  rows: Array<{ label: string; color: string; value: string }>;
}>({ visible: false, left: 0, top: 0, time: "", rows: [] });

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

function formatValue(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "—";
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 3 }).format(value);
}

function formatAxis(value: number): string {
  const absolute = Math.abs(value);
  if (absolute >= 10000) {
    return new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);
  }
  if (Number.isInteger(value)) {
    return new Intl.NumberFormat("en-US").format(value);
  }
  return value.toFixed(2);
}

function updateCursor(self: uPlot): void {
  const index = self.cursor.idx;
  if (index === null || index === undefined) {
    tooltip.value.visible = false;
    return;
  }
  const timestamp = Number(self.data[0]?.[index] ?? 0) * 1000;
  const rows = props.series.map((series, seriesIndex) => {
    const value = self.data[seriesIndex + 1]?.[index];
    return {
      label: series.label,
      color: series.color,
      value: formatValue(typeof value === "number" ? value : null),
    };
  });
  const left = Number(self.cursor.left ?? 0);
  const top = Number(self.cursor.top ?? 0);
  const maxLeft = Math.max(8, (self.width || 400) - 190);
  tooltip.value = {
    visible: true,
    left: Math.min(Math.max(left + 14, 8), maxLeft),
    top: Math.max(top - 10, 8),
    time: preciseTime(timestamp),
    rows,
  };
}

function drawPrimaryLabel(self: uPlot): void {
  if (!props.series.length || props.series.length > 4) return;
  const series = props.series[0];
  if (!series) return;
  const values = self.data[1];
  const times = self.data[0];
  if (!values || !times) return;

  let index = -1;
  for (let cursor = values.length - 1; cursor >= 0; cursor -= 1) {
    if (values[cursor] !== null && values[cursor] !== undefined) {
      index = cursor;
      break;
    }
  }
  if (index < 0) return;

  const value = values[index];
  const timestamp = times[index];
  if (typeof value !== "number" || typeof timestamp !== "number") return;

  const x = self.valToPos(timestamp, "x", true);
  const y = self.valToPos(value, "y", true);
  if (!Number.isFinite(x) || !Number.isFinite(y)) return;

  const context = self.ctx;
  const colors = palette.value;
  context.save();

  // End dot: 8px marker with a 2px surface ring.
  context.beginPath();
  context.arc(x, y, 4, 0, Math.PI * 2);
  context.fillStyle = series.color;
  context.fill();
  context.lineWidth = 2;
  context.strokeStyle = colors.surface;
  context.stroke();

  const label = `${series.label}  ${formatValue(value)}`;
  context.font = "11px 'JetBrains Mono Variable', monospace";
  const padding = 5;
  const textWidth = context.measureText(label).width;
  const boxWidth = textWidth + padding * 2;
  const boxHeight = 19;
  const placeLeft = x + 10 + boxWidth > self.width - 4;
  const boxX = placeLeft ? x - 10 - boxWidth : x + 10;
  const boxY = Math.min(Math.max(y - boxHeight / 2, 2), self.height - boxHeight - 2);

  context.beginPath();
  context.roundRect(boxX, boxY, boxWidth, boxHeight, 4);
  context.fillStyle = colors.labelBackground;
  context.fill();
  context.fillStyle = colors.text;
  context.textBaseline = "middle";
  context.fillText(label, boxX + padding, boxY + boxHeight / 2 + 0.5);
  context.restore();
}

function buildOptions(width: number, height: number): uPlot.Options {
  const single = props.series.length === 1;
  const colors = palette.value;
  return {
    width,
    height,
    padding: [8, 10, 2, 0],
    cursor: {
      show: true,
      x: true,
      y: false,
      drag: { setScale: false },
      points: { show: true, size: 7, width: 2, stroke: colors.surface },
    },
    scales: {
      x: { time: true },
      y: { auto: true },
    },
    axes: [
      {
        stroke: colors.axis,
        font: "11px 'JetBrains Mono Variable', monospace",
        grid: { stroke: colors.grid, width: 1 },
        ticks: { stroke: colors.tick, width: 1 },
        values: (_self, splits) => splits.map((value) => shortTime(value * 1000)),
      },
      {
        stroke: colors.axis,
        font: "11px 'JetBrains Mono Variable', monospace",
        grid: { stroke: colors.grid, width: 1 },
        ticks: { show: false },
        size: 52,
        values: (_self, splits) => splits.map((value) => formatAxis(value)),
      },
    ],
    series: [
      { label: t("chart.time") },
      ...props.series.map((series) => ({
        label: series.label,
        stroke: series.color,
        width: 2,
        spanGaps: false,
        points: { show: false },
        fill: single ? `${series.color}1f` : undefined,
      })),
    ],
    legend: { show: false },
    hooks: {
      setCursor: [updateCursor],
      draw: [drawPrimaryLabel],
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
  if (!container.value || !props.series.length) return;
  const width = Math.max(container.value.clientWidth, 80);
  const height = Math.max(container.value.clientHeight, 80);
  plot = new uPlot(buildOptions(width, height), props.data as uPlot.AlignedData, container.value);
}

function seriesSignature(): string {
  return props.series.map((series) => `${series.key}:${series.label}:${series.color}`).join("|");
}

watch(
  () => props.data,
  (data) => {
    if (plot) plot.setData(data as uPlot.AlignedData, true);
  },
);
watch(seriesSignature, createPlot);
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
      class="glass-strong pointer-events-none absolute z-20 w-[184px] rounded-lg px-2.5 py-2 shadow-xl shadow-black/25"
      :style="{ left: `${tooltip.left}px`, top: `${tooltip.top}px` }"
    >
      <p class="numeric text-[11px] text-muted-foreground">{{ tooltip.time }}</p>
      <div v-for="row in tooltip.rows" :key="row.label" class="mt-1 flex items-center gap-1.5">
        <span class="size-1.5 shrink-0 rounded-full" :style="{ background: row.color }" />
        <span class="min-w-0 truncate text-[11px] text-muted-foreground">{{ row.label }}</span>
        <span class="numeric ml-auto text-[11px] text-foreground">{{ row.value }}</span>
      </div>
    </div>
  </div>
</template>
