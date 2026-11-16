<script setup lang="ts">
import { Activity, LineChart, Pause, Play, Table2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import EmptyState from "@/components/common/EmptyState.vue";
import RealtimeChart from "@/components/chart/RealtimeChart.vue";
import { Button } from "@/components/ui/button";
import { formatAddress, formatTime } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ChartData, ChartSeries } from "@/types/chart";

// Validated for the dark chart surface with the dataviz palette validator:
// all six slots pass lightness, chroma, adjacent CVD, normal-vision and contrast.
const PALETTE = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#0ca30c"];
const MAX_SERIES = 6;
const WINDOW_OPTIONS = [
  { label: "1m", value: 60 },
  { label: "5m", value: 300 },
  { label: "15m", value: 900 },
  { label: "30m", value: 1800 },
];

const sessions = useSessionsStore();
const settings = useSettingsStore();

const windowSec = ref(300);
const paused = ref(false);
const frozen = ref<ChartData | null>(null);
const showTable = ref(false);
const selectedAddresses = ref<number[]>([]);

const definition = computed(() => sessions.activeDefinition);
const timeline = computed(() =>
  definition.value ? sessions.timelines[definition.value.id] : undefined,
);

interface SeriesEntry {
  address: number;
  label: string;
  color: string;
  values: Array<number | null>;
}

const numericCount = computed(() => {
  const tl = timeline.value;
  if (!tl) return 0;
  return Object.values(tl.series).filter((values) => values.some((value) => value !== null)).length;
});

const allSeries = computed<SeriesEntry[]>(() => {
  const tl = timeline.value;
  const def = definition.value;
  if (!tl || !def) return [];
  return Object.entries(tl.series)
    .map(([address, values]) => ({ address: Number(address), values }))
    .filter((entry) => entry.values.some((value) => value !== null))
    .sort((left, right) => left.address - right.address)
    .slice(0, MAX_SERIES)
    .map((entry, index) => ({
      ...entry,
      label:
        def.aliases[entry.address] ?? `地址 ${formatAddress(entry.address, settings.addressBase)}`,
      color: PALETTE[index] ?? PALETTE[0],
    }));
});

watch(
  allSeries,
  (series) => {
    const valid = selectedAddresses.value.filter((address) =>
      series.some((entry) => entry.address === address),
    );
    selectedAddresses.value = valid.length
      ? valid
      : series.slice(0, Math.min(3, series.length)).map((entry) => entry.address);
  },
  { immediate: true },
);

watch(
  () => definition.value?.id,
  () => {
    paused.value = false;
    frozen.value = null;
    showTable.value = false;
    selectedAddresses.value = [];
  },
);

const selectedSeries = computed(() =>
  allSeries.value.filter((entry) => selectedAddresses.value.includes(entry.address)),
);

const chartSeries = computed<ChartSeries[]>(() =>
  selectedSeries.value.map((entry) => ({
    key: entry.address,
    label: entry.label,
    color: entry.color,
  })),
);

function latestValue(entry: SeriesEntry): string {
  for (let index = entry.values.length - 1; index >= 0; index -= 1) {
    const value = entry.values[index];
    if (value !== null && value !== undefined && Number.isFinite(value)) {
      return formatValue(value);
    }
  }
  return "—";
}

function formatValue(value: number): string {
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 3 }).format(value);
}

function toggleAddress(address: number): void {
  if (selectedAddresses.value.includes(address)) {
    if (selectedAddresses.value.length > 1) {
      selectedAddresses.value = selectedAddresses.value.filter((entry) => entry !== address);
    }
    return;
  }
  if (selectedAddresses.value.length < MAX_SERIES) {
    selectedAddresses.value = [...selectedAddresses.value, address];
  }
}

const liveData = computed<ChartData | null>(() => {
  const tl = timeline.value;
  if (!tl || !tl.timestamps.length || !selectedSeries.value.length) return null;
  const cutoff = Date.now() - windowSec.value * 1000;
  const start = tl.timestamps.findIndex((timestamp) => timestamp >= cutoff);
  if (start < 0) return null;
  const times = tl.timestamps.slice(start).map((timestamp) => timestamp / 1000);
  if (!times.length) return null;
  const series = selectedSeries.value.map((entry) => {
    const values = tl.series[entry.address] ?? [];
    return times.map((_, offset) => {
      const value = values[start + offset];
      return typeof value === "number" ? value : null;
    });
  });
  return [times, ...series];
});

const displayData = computed<ChartData | null>(() =>
  paused.value ? frozen.value : liveData.value,
);

function togglePause(): void {
  if (paused.value) {
    paused.value = false;
    frozen.value = null;
  } else {
    frozen.value = liveData.value;
    paused.value = true;
  }
}

const tableRows = computed(() => {
  const tl = timeline.value;
  if (!tl) return [];
  const start = Math.max(0, tl.timestamps.length - 20);
  return tl.timestamps
    .slice(start)
    .map((timestamp, offset) => ({
      timestamp,
      values: selectedSeries.value.map((entry) => {
        const raw = tl.series[entry.address] ?? [];
        const value = raw[start + offset];
        return typeof value === "number" ? formatValue(value) : "—";
      }),
    }))
    .reverse();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex h-8 shrink-0 items-center gap-2 border-b border-white/[0.04] px-3">
      <Activity class="size-3 text-signal" />
      <span class="text-[10.5px] font-medium">实时曲线</span>
      <span class="max-w-[180px] truncate text-[9.5px] text-muted-foreground/55">
        {{ definition?.name ?? "未选择定义" }}
      </span>
      <span
        v-if="paused"
        class="rounded border border-warn/25 bg-warn/10 px-1.5 py-0.5 text-[8.5px] text-warn"
      >
        已暂停
      </span>
      <span v-else-if="displayData" class="flex items-center gap-1 text-[8.5px] text-signal/80">
        <span class="status-dot status-dot-live scale-75" />
        LIVE
      </span>
      <span
        v-if="numericCount > MAX_SERIES"
        class="rounded border border-white/[0.06] px-1 py-0.5 text-[8px] text-muted-foreground/45"
      >
        仅显示前 {{ MAX_SERIES }} 条
      </span>

      <div class="ml-auto flex items-center gap-1.5">
        <div class="flex rounded-md border border-white/[0.06] bg-black/20 p-0.5">
          <button
            v-for="option in WINDOW_OPTIONS"
            :key="option.value"
            type="button"
            class="numeric rounded px-1.5 py-0.5 text-[9px] transition-colors"
            :class="
              windowSec === option.value
                ? 'bg-white/[0.08] text-foreground'
                : 'text-muted-foreground/50 hover:text-foreground'
            "
            @click="windowSec = option.value"
          >
            {{ option.label }}
          </button>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md text-muted-foreground/65 hover:text-foreground"
          :aria-label="paused ? '继续曲线' : '暂停曲线'"
          @click="togglePause"
        >
          <Play v-if="paused" class="size-3" />
          <Pause v-else class="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md"
          :class="showTable ? 'text-signal' : 'text-muted-foreground/65 hover:text-foreground'"
          aria-label="切换数据表视图"
          @click="showTable = !showTable"
        >
          <Table2 class="size-3" />
        </Button>
      </div>
    </div>

    <div v-if="allSeries.length" class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 p-2 pb-0">
        <div
          v-if="showTable"
          class="scrollbar-thin h-full overflow-auto rounded-lg border border-white/[0.05] bg-black/15"
        >
          <table class="w-full border-collapse text-[9.5px]">
            <thead class="sticky top-0 bg-[#151a22]">
              <tr class="border-b border-white/[0.06]">
                <th class="px-2 py-1.5 text-left font-medium text-muted-foreground/55">时间</th>
                <th
                  v-for="entry in selectedSeries"
                  :key="entry.address"
                  class="px-2 py-1.5 text-right font-medium text-muted-foreground/55"
                >
                  <span class="inline-flex items-center gap-1">
                    <span class="size-1.5 rounded-full" :style="{ background: entry.color }" />
                    {{ entry.label }}
                  </span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in tableRows"
                :key="row.timestamp"
                class="border-b border-white/[0.03]"
              >
                <td class="numeric px-2 py-1 text-muted-foreground/60">
                  {{ formatTime(row.timestamp) }}
                </td>
                <td
                  v-for="(value, index) in row.values"
                  :key="index"
                  class="numeric px-2 py-1 text-right text-foreground/85"
                >
                  {{ value }}
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="!tableRows.length"
            class="px-3 py-6 text-center text-[10px] text-muted-foreground/50"
          >
            窗口内暂无采样
          </p>
        </div>

        <div v-else-if="displayData" class="relative h-full min-h-[120px]">
          <RealtimeChart :data="displayData" :series="chartSeries" />
          <div
            v-if="paused"
            class="glass pointer-events-none absolute bottom-2 left-2 rounded-md px-2 py-1 text-[8.5px] text-warn/85"
          >
            已暂停 · 数据仍在后台缓冲
          </div>
        </div>

        <EmptyState
          v-else
          title="等待曲线数据…"
          description="轮询产生数值后会在此绘制趋势；HEX / BIN / ASCII 类型不参与曲线。"
          class="h-full"
        >
          <template #icon>
            <LineChart class="size-4" />
          </template>
        </EmptyState>
      </div>

      <div
        v-if="selectedSeries.length >= 2"
        class="flex shrink-0 flex-wrap items-center gap-1.5 border-t border-white/[0.04] px-3 py-1.5"
      >
        <button
          v-for="entry in allSeries"
          :key="entry.address"
          type="button"
          class="flex items-center gap-1.5 rounded-md border px-1.5 py-1 transition-colors"
          :class="
            selectedAddresses.includes(entry.address)
              ? 'border-white/[0.08] bg-white/[0.035]'
              : 'border-transparent opacity-45 hover:opacity-75'
          "
          @click="toggleAddress(entry.address)"
        >
          <span class="size-1.5 rounded-full" :style="{ background: entry.color }" />
          <span class="max-w-[110px] truncate text-[9px] text-muted-foreground/80">
            {{ entry.label }}
          </span>
          <span class="numeric text-[9px] text-foreground/85">{{ latestValue(entry) }}</span>
        </button>
      </div>
      <div
        v-else-if="selectedSeries.length === 1"
        class="flex shrink-0 items-center gap-2 border-t border-white/[0.04] px-3 py-1.5"
      >
        <span class="size-1.5 rounded-full" :style="{ background: selectedSeries[0]?.color }" />
        <span class="text-[9.5px] text-muted-foreground/75">{{ selectedSeries[0]?.label }}</span>
        <span class="numeric text-[10px] text-foreground/90">{{
          latestValue(selectedSeries[0]!)
        }}</span>
        <span class="ml-auto text-[8.5px] text-muted-foreground/40">单序列 · 无图例</span>
      </div>
    </div>

    <EmptyState
      v-else
      :title="definition ? '暂无可绘制的数值' : '选择一个读取定义'"
      :description="
        definition
          ? '当前定义没有数值类型采样，或尚未收到数据。'
          : '在左侧选择定义后即可查看实时趋势。'
      "
      class="m-auto"
    >
      <template #icon>
        <LineChart class="size-4" />
      </template>
    </EmptyState>
  </div>
</template>
