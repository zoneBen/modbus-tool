<script setup lang="ts">
import { Activity, LineChart, Pause, Play, Table2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import EmptyState from "@/components/common/EmptyState.vue";
import RealtimeChart from "@/components/chart/RealtimeChart.vue";
import { Button } from "@/components/ui/button";
import { formatAddress, formatTime } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ChartData, ChartSeries } from "@/types/chart";

// Both palettes pass the dataviz validator on their own chart surface
// (light: #f6f8fb, dark: #1a1f27); slots are assigned in fixed order.
const DARK_PALETTE = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#0ca30c"];
const LIGHT_PALETTE = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300"];
const MAX_SERIES = 6;
const WINDOW_OPTIONS = [
  { label: "1m", value: 60 },
  { label: "5m", value: 300 },
  { label: "15m", value: 900 },
  { label: "30m", value: 1800 },
];

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const windowSec = ref(300);
const paused = ref(false);
const frozen = ref<ChartData | null>(null);
const showTable = ref(false);
const selectedAddresses = ref<number[]>([]);

const definition = computed(() => sessions.activeDefinition);
const timeline = computed(() =>
  definition.value ? sessions.timelines[definition.value.id] : undefined,
);
const focusAddress = computed(() => sessions.chartFocusAddress);
const palette = computed(() => (settings.resolvedTheme === "dark" ? DARK_PALETTE : LIGHT_PALETTE));

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
  const ordered = Object.entries(tl.series)
    .map(([address, values]) => ({ address: Number(address), values }))
    .filter((entry) => entry.values.some((value) => value !== null))
    .sort((left, right) => left.address - right.address);
  if (!ordered.length) return [];
  const focusIndex = ordered.findIndex((entry) => entry.address === focusAddress.value);
  const start =
    focusIndex >= 0
      ? Math.max(0, Math.min(focusIndex - Math.floor(MAX_SERIES / 2), ordered.length - MAX_SERIES))
      : 0;
  return ordered.slice(start, start + MAX_SERIES).map((entry, index) => ({
    ...entry,
    label:
      def.aliases[entry.address] ??
      t("chart.address", {
        address: formatAddress(entry.address, settings.addressBase),
      }),
    color: palette.value[index] ?? palette.value[0],
  }));
});

let lastFocusAddress: number | null | undefined;
let lastFocusDefinition: string | null | undefined;

watch(
  [allSeries, focusAddress, () => definition.value?.id] as const,
  ([series, focus, definitionId]) => {
    const focusChanged = focus !== lastFocusAddress || definitionId !== lastFocusDefinition;
    lastFocusAddress = focus;
    lastFocusDefinition = definitionId;
    const hadSelection = selectedAddresses.value.some((address) =>
      series.some((entry) => entry.address === address),
    );
    if (focus !== null && (focusChanged || !hadSelection)) {
      const index = series.findIndex((entry) => entry.address === focus);
      if (index >= 0) {
        selectedAddresses.value = [series[index - 1], series[index], series[index + 1]]
          .filter((entry): entry is SeriesEntry => Boolean(entry))
          .map((entry) => entry.address);
        return;
      }
    }
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
    <div class="flex h-8 shrink-0 items-center gap-2 border-b border-hairline px-3">
      <Activity class="size-3 text-signal" />
      <span class="text-[11.5px] font-medium">{{ t("chart.realtime") }}</span>
      <span class="max-w-[180px] truncate text-[11.5px] text-muted-foreground">
        {{ definition?.name ?? t("chart.noDefinition") }}
      </span>
      <span
        v-if="paused"
        class="rounded border border-warn/25 bg-warn/10 px-1.5 py-0.5 text-[11px] text-warn"
      >
        {{ t("chart.paused") }}
      </span>
      <span v-else-if="displayData" class="flex items-center gap-1 text-[11px] text-signal">
        <span class="status-dot status-dot-live scale-75" />
        LIVE
      </span>
      <span
        v-if="numericCount > MAX_SERIES"
        class="rounded border border-hairline px-1 py-0.5 text-[11px] text-muted-foreground"
      >
        {{ t("chart.nearbySeries", { count: MAX_SERIES }) }}
      </span>

      <div class="ml-auto flex items-center gap-1.5">
        <div class="flex rounded-md border border-hairline bg-well p-0.5">
          <button
            v-for="option in WINDOW_OPTIONS"
            :key="option.value"
            type="button"
            class="numeric rounded px-1.5 py-0.5 text-[11px] transition-colors"
            :class="
              windowSec === option.value
                ? 'bg-overlay-3 text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            "
            @click="windowSec = option.value"
          >
            {{ option.label }}
          </button>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md text-muted-foreground hover:text-foreground"
          :aria-label="paused ? t('chart.resume') : t('chart.pause')"
          @click="togglePause"
        >
          <Play v-if="paused" class="size-3" />
          <Pause v-else class="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md"
          :class="showTable ? 'text-signal' : 'text-muted-foreground hover:text-foreground'"
          :aria-label="t('chart.switchTable')"
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
          class="scrollbar-thin h-full overflow-auto rounded-lg border border-hairline bg-well-soft"
        >
          <table class="w-full border-collapse text-[11.5px]">
            <thead class="sticky top-0 bg-surface">
              <tr class="border-b border-hairline">
                <th class="px-2 py-1.5 text-left font-medium text-muted-foreground">
                  {{ t("chart.time") }}
                </th>
                <th
                  v-for="entry in selectedSeries"
                  :key="entry.address"
                  class="px-2 py-1.5 text-right font-medium text-muted-foreground"
                >
                  <span class="inline-flex items-center gap-1">
                    <span class="size-1.5 rounded-full" :style="{ background: entry.color }" />
                    {{ entry.label }}
                  </span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in tableRows" :key="row.timestamp" class="border-b border-hairline">
                <td class="numeric px-2 py-1 text-muted-foreground">
                  {{ formatTime(row.timestamp) }}
                </td>
                <td
                  v-for="(value, index) in row.values"
                  :key="index"
                  class="numeric px-2 py-1 text-right text-foreground"
                >
                  {{ value }}
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="!tableRows.length"
            class="px-3 py-6 text-center text-[11px] text-muted-foreground"
          >
            {{ t("chart.noSamples") }}
          </p>
        </div>

        <div v-else-if="displayData" class="relative h-full min-h-[120px]">
          <RealtimeChart :data="displayData" :series="chartSeries" />
          <div
            v-if="paused"
            class="glass pointer-events-none absolute bottom-2 left-2 rounded-md px-2 py-1 text-[11px] text-warn"
          >
            {{ t("chart.pausedHint") }}
          </div>
        </div>

        <EmptyState
          v-else
          :title="t('chart.waitingTitle')"
          :description="t('chart.waitingHint')"
          class="h-full"
        >
          <template #icon>
            <LineChart class="size-4" />
          </template>
        </EmptyState>
      </div>

      <div
        v-if="selectedSeries.length >= 2"
        class="flex shrink-0 flex-wrap items-center gap-1.5 border-t border-hairline px-3 py-1.5"
      >
        <button
          v-for="entry in allSeries"
          :key="entry.address"
          type="button"
          class="flex items-center gap-1.5 rounded-md border px-1.5 py-1 transition-colors"
          :class="
            selectedAddresses.includes(entry.address)
              ? 'border-hairline-strong bg-overlay-2'
              : 'border-transparent opacity-60 hover:opacity-90'
          "
          @click="toggleAddress(entry.address)"
        >
          <span class="size-1.5 rounded-full" :style="{ background: entry.color }" />
          <span class="max-w-[110px] truncate text-[11px] text-muted-foreground">
            {{ entry.label }}
          </span>
          <span class="numeric text-[11px] text-foreground">{{ latestValue(entry) }}</span>
        </button>
      </div>
      <div
        v-else-if="selectedSeries.length === 1"
        class="flex shrink-0 items-center gap-2 border-t border-hairline px-3 py-1.5"
      >
        <span class="size-1.5 rounded-full" :style="{ background: selectedSeries[0]?.color }" />
        <span class="text-[11.5px] text-muted-foreground">{{ selectedSeries[0]?.label }}</span>
        <span class="numeric text-[11px] text-foreground">{{
          latestValue(selectedSeries[0]!)
        }}</span>
        <span class="ml-auto text-[11px] text-muted-foreground">
          {{ t("chart.singleSeries") }}
        </span>
      </div>
    </div>

    <EmptyState
      v-else
      :title="definition ? t('chart.noNumeric') : t('chart.selectDefinition')"
      :description="definition ? t('chart.noNumericHint') : t('chart.selectHint')"
      class="m-auto"
    >
      <template #icon>
        <LineChart class="size-4" />
      </template>
    </EmptyState>
  </div>
</template>
