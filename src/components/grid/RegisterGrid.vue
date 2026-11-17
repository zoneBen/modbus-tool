<script setup lang="ts">
import { BellRing, PencilLine, Plus, RefreshCw, Table2, Trash2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import AlertRulesDialog from "@/components/definition/AlertRulesDialog.vue";
import AliasDialog from "@/components/definition/AliasDialog.vue";
import PointDialog from "@/components/definition/PointDialog.vue";
import SimulatorPointDialog from "@/components/definition/SimulatorPointDialog.vue";
import SlaveValueDialog from "@/components/definition/SlaveValueDialog.vue";
import Sparkline from "@/components/grid/Sparkline.vue";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { resolvePoints, toDefinitionInput } from "@/lib/definition";
import { formatAddress, formatHexBytes, formatLatency, formatWord, historyKey } from "@/lib/format";
import { cloneMappingPoint, mappingPoints, mappingRange } from "@/lib/simulator-mapping";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Definition, RegisterPoint, RegisterValue } from "@/types/models";
import type { WriteTarget } from "@/types/ui";

const emit = defineEmits<{
  (event: "write", target: WriteTarget): void;
  (event: "new-session"): void;
  (event: "open-trend", address?: number): void;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const selectedAddress = computed(() => sessions.selectedAddress);
const aliasOpen = ref(false);
const aliasRow = ref<GridRow | null>(null);
const pointOpen = ref(false);
const pointAddress = ref(0);
const alertOpen = ref(false);
const alertAddresses = ref<number[]>([]);
const selectedForAlerts = ref<number[]>([]);
const simulatorOpen = ref(false);
const simulatorAddress = ref<number | null>(null);
const mappingDeleteOpen = ref(false);
const mappingDeleteRow = ref<GridRow | null>(null);
const deletingMapping = ref(false);
const slaveValueOpen = ref(false);
const slaveValueRow = ref<GridRow | null>(null);
const flashMap = ref<Record<number, boolean>>({});
let previousValues = new Map<number, string>();

const definition = computed<Definition | null>(() => sessions.activeDefinition);
const snapshot = computed(() => sessions.activeSnapshot);
const session = computed(() => sessions.activeSession);
const isSlave = computed(() => session.value?.role === "slave");

interface GridRow extends RegisterValue {
  alias: string;
  raw: string;
  history: number[];
  point: RegisterPoint;
}

function buildRow(def: Definition, point: RegisterPoint, value?: RegisterValue): GridRow {
  const current: RegisterValue = value ?? {
    address: point.address,
    words: [],
    formatted: "—",
    numeric: null,
    stale: false,
  };
  return {
    ...current,
    alias:
      point.meaning || def.aliases[point.address] || t("grid.register", { address: point.address }),
    raw: formatHexBytes(current.words.flatMap((word) => [(word >> 8) & 0xff, word & 0xff])),
    history: sessions.history[historyKey(def.id, point.address)] ?? [],
    point,
  };
}

const rows = computed<GridRow[]>(() => {
  const def = definition.value;
  if (!def) return [];
  if (isSlave.value) {
    const values = new Map((snapshot.value?.values ?? []).map((value) => [value.address, value]));
    return mappingPoints(def).map((point) => buildRow(def, point, values.get(point.address)));
  }
  const snap = snapshot.value;
  if (!snap) return [];
  const points = new Map(resolvePoints(def).map((point) => [point.address, point]));
  return snap.values.map((value) => {
    const point =
      points.get(value.address) ??
      ({
        address: value.address,
        dataType: def.dataType,
        byteOrder: def.byteOrder,
        meaning: def.aliases[value.address] ?? "",
        unit: "",
        scale: 1,
        offset: 0,
        alerts: [],
      } satisfies RegisterPoint);
    return buildRow(def, point, value);
  });
});

const allSelected = computed(
  () =>
    rows.value.length > 0 &&
    rows.value.every((row) => selectedForAlerts.value.includes(row.address)),
);

const sparkTone = computed(() => {
  if (session.value?.role === "slave") return "var(--sim)";
  if (session.value?.transport === "serial") return "var(--rx)";
  return "var(--signal)";
});

const gridClass = computed(() => {
  if (isSlave.value) {
    return settings.showRawColumn
      ? "grid-cols-[64px_minmax(108px,1.2fr)_84px_minmax(82px,0.8fr)_52px_50px_54px_46px_44px_44px_36px_52px]"
      : "grid-cols-[64px_minmax(108px,1.2fr)_minmax(82px,0.8fr)_52px_50px_54px_46px_44px_44px_36px_52px]";
  }
  return settings.showRawColumn
    ? "grid-cols-[28px_72px_minmax(130px,1.2fr)_110px_minmax(100px,0.9fr)_84px_68px_64px_48px_52px_54px_66px_26px]"
    : "grid-cols-[28px_72px_minmax(130px,1.2fr)_minmax(100px,0.9fr)_84px_68px_64px_48px_52px_54px_66px_26px]";
});

const rowHeightClass = computed(() => (settings.compactRows ? "h-8" : "h-[34px]"));

watch(
  () => snapshot.value?.timestamp,
  () => {
    const snap = snapshot.value;
    if (!snap) return;
    const nextFlash = { ...flashMap.value };
    const nextPrevious = new Map<number, string>();
    for (const value of snap.values) {
      const previous = previousValues.get(value.address);
      if (previous !== undefined && previous !== value.formatted) {
        nextFlash[value.address] = true;
        window.setTimeout(() => {
          const cleared = { ...flashMap.value };
          delete cleared[value.address];
          flashMap.value = cleared;
        }, 620);
      }
      nextPrevious.set(value.address, value.formatted);
    }
    previousValues = nextPrevious;
    flashMap.value = nextFlash;
  },
);

watch(
  () => definition.value?.id,
  () => {
    previousValues = new Map();
    flashMap.value = {};
    selectedForAlerts.value = [];
    sessions.selectAddress(null);
  },
);

watch(
  () => definition.value?.points,
  () => {
    const available = new Set(rows.value.map((row) => row.address));
    selectedForAlerts.value = selectedForAlerts.value.filter((address) => available.has(address));
  },
  { deep: true },
);

function openWrite(row: GridRow): void {
  if (!definition.value) return;
  if (isSlave.value) {
    openSlaveValue(row);
    return;
  }
  emit("write", {
    address: row.address,
    alias: row.alias,
    current: row.formatted,
    dataType: row.point.dataType,
  });
}

const selectedRow = computed(
  () => rows.value.find((row) => row.address === selectedAddress.value) ?? null,
);

function openSelectedWrite(): void {
  if (selectedRow.value) openWrite(selectedRow.value);
}

function openTrend(address?: number): void {
  if (!definition.value) return;
  const target = address ?? selectedAddress.value ?? undefined;
  if (target !== undefined) sessions.focusChart(target);
  emit("open-trend", target);
}

function openAlias(row: GridRow): void {
  aliasRow.value = row;
  aliasOpen.value = true;
}

function openMeaning(row: GridRow): void {
  if (isSlave.value) openPoint(row);
  else openAlias(row);
}

function openPoint(row: GridRow): void {
  if (!definition.value) return;
  if (isSlave.value) {
    simulatorAddress.value = row.address;
    simulatorOpen.value = true;
    return;
  }
  pointAddress.value = row.address;
  pointOpen.value = true;
}

function openSimulatorAdd(): void {
  if (!definition.value || !isSlave.value) return;
  simulatorAddress.value = null;
  simulatorOpen.value = true;
}

function openMappingDelete(row: GridRow): void {
  mappingDeleteRow.value = row;
  mappingDeleteOpen.value = true;
}

function closeMappingDelete(): void {
  if (deletingMapping.value) return;
  mappingDeleteOpen.value = false;
  mappingDeleteRow.value = null;
}

async function confirmMappingDelete(): Promise<void> {
  const def = definition.value;
  const row = mappingDeleteRow.value;
  if (!def || !row || deletingMapping.value) return;
  const points = mappingPoints(def)
    .filter((point) => point.address !== row.address)
    .map(cloneMappingPoint);
  const range = mappingRange(points);
  const aliases: Record<number, string> = {};
  for (const point of points) {
    if (point.meaning.trim()) aliases[point.address] = point.meaning.trim();
  }
  deletingMapping.value = true;
  try {
    await sessions.updateDefinition(def.id, {
      ...toDefinitionInput(def),
      startAddress: range.startAddress,
      quantity: range.quantity,
      aliases,
      points,
    });
    toast.success(t("grid.mappingDeleted"));
    mappingDeleteOpen.value = false;
    mappingDeleteRow.value = null;
  } catch (error: unknown) {
    toast.error(t("grid.mappingDeleteFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    deletingMapping.value = false;
  }
}

const simulatorModeLabel = computed<Record<string, string>>(() => ({
  fixed: t("grid.simulatorModes.fixed"),
  increment: t("grid.simulatorModes.increment"),
  decrement: t("grid.simulatorModes.decrement"),
  random: t("grid.simulatorModes.random"),
}));

function openAlerts(addresses: number[]): void {
  if (!definition.value || isSlave.value || !addresses.length) return;
  alertAddresses.value = addresses;
  alertOpen.value = true;
}

function openBulkAlerts(): void {
  openAlerts(selectedForAlerts.value);
}

function toggleSelected(address: number): void {
  selectedForAlerts.value = selectedForAlerts.value.includes(address)
    ? selectedForAlerts.value.filter((entry) => entry !== address)
    : [...selectedForAlerts.value, address];
}

function toggleSelectAll(): void {
  selectedForAlerts.value = allSelected.value ? [] : rows.value.map((row) => row.address);
}

function openSlaveValue(row: GridRow): void {
  slaveValueRow.value = row;
  slaveValueOpen.value = true;
}

function openValue(row: GridRow): void {
  if (isSlave.value) openSlaveValue(row);
  else openTrend(row.address);
}
</script>

<template>
  <section class="relative flex h-full min-h-0 min-w-0 flex-col">
    <template v-if="definition">
      <div
        class="flex h-9 shrink-0 items-center gap-2 border-b border-hairline px-4 text-[11px] text-muted-foreground"
      >
        <span class="label-caps">{{ t("grid.title") }}</span>
        <span class="text-foreground/15">/</span>
        <span class="numeric">
          {{
            isSlave
              ? t("grid.mappingCount", { count: rows.length })
              : t("grid.values", { count: rows.length })
          }}
        </span>
        <span v-if="!isSlave" class="numeric text-muted-foreground">
          · {{ t("grid.registers", { count: definition.quantity }) }}
        </span>
        <span class="ml-auto flex items-center gap-2">
          <Button
            v-if="!isSlave"
            variant="outline"
            size="sm"
            class="h-7 gap-1 border-hairline-strong bg-overlay-1 px-2 text-[11px]"
            :disabled="!selectedRow"
            :aria-label="t('grid.writeSelected')"
            :title="t('grid.writeSelectedHint')"
            @click="openSelectedWrite"
          >
            <PencilLine class="size-3" />
            {{ t("grid.write") }}
          </Button>
          <Button
            v-else
            size="sm"
            class="h-7 gap-1 border-sim/25 bg-sim/10 px-2 text-[11px] text-sim hover:bg-sim/15"
            @click="openSimulatorAdd"
          >
            <Plus class="size-3" />
            {{ t("grid.addMapping") }}
          </Button>
          <span class="status-dot" :class="snapshot?.error ? 'text-error' : 'text-signal'" />
          <span>
            {{
              snapshot?.error
                ? t("grid.readError")
                : snapshot
                  ? t("grid.live")
                  : t("grid.waitingSamples")
            }}
          </span>
          <span v-if="snapshot" class="numeric text-muted-foreground">
            {{ formatLatency(snapshot.latencyMs) }}
          </span>
        </span>
      </div>

      <div
        v-if="!isSlave"
        class="flex h-8 shrink-0 items-center gap-3 border-b border-hairline bg-overlay-1/35 px-4 text-[11px] text-muted-foreground"
      >
        <label class="flex cursor-pointer items-center gap-1.5">
          <input
            type="checkbox"
            class="size-3 accent-[var(--signal)]"
            :checked="allSelected"
            @change="toggleSelectAll"
          />
          {{ t("grid.selectAll") }}
        </label>
        <span class="numeric">{{
          t("grid.selectedCount", { count: selectedForAlerts.length })
        }}</span>
        <Button
          variant="outline"
          size="sm"
          class="h-6 gap-1 border-warn/25 bg-warn/10 px-2 text-[10.5px] text-warn hover:bg-warn/15"
          :disabled="!selectedForAlerts.length"
          @click="openBulkAlerts"
        >
          <BellRing class="size-3" />
          {{ t("grid.bulkAlerts") }}
        </Button>
        <span class="ml-auto text-[10.5px] text-muted-foreground/75">
          {{ t("grid.bulkHint") }}
        </span>
      </div>

      <div class="min-h-0 flex-1 overflow-auto scrollbar-thin">
        <div :class="isSlave ? 'min-w-[760px]' : 'min-w-[1220px]'">
          <div
            class="sticky top-0 z-10 grid items-center border-b border-hairline bg-background/80 px-4 py-1.5 backdrop-blur-md"
            :class="gridClass"
          >
            <span v-if="!isSlave" class="label-caps text-center">
              {{ t("grid.columns.select") }}
            </span>
            <span class="label-caps">{{ t("grid.columns.address") }}</span>
            <span class="label-caps">{{ t("grid.columns.meaning") }}</span>
            <span v-if="settings.showRawColumn" class="label-caps">
              {{ t("grid.columns.raw") }}
            </span>
            <span class="label-caps text-right">{{ t("grid.columns.value") }}</span>
            <span v-if="!isSlave" class="label-caps text-center">
              {{ t("grid.columns.trend") }}
            </span>
            <span class="label-caps text-center">{{ t("grid.columns.type") }}</span>
            <span class="label-caps text-center">{{ t("grid.columns.order") }}</span>
            <template v-if="!isSlave">
              <span class="label-caps text-center">{{ t("grid.columns.unit") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.scale") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.offset") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.alerts") }}</span>
            </template>
            <template v-else>
              <span class="label-caps text-center">{{ t("grid.columns.mode") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.refresh") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.min") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.max") }}</span>
              <span class="label-caps text-center">{{ t("grid.columns.start") }}</span>
            </template>
            <span class="label-caps text-center">
              {{ isSlave ? t("grid.columns.actions") : t("grid.columns.status") }}
            </span>
          </div>

          <template v-if="rows.length">
            <div
              v-for="row in rows"
              :key="row.address"
              class="row-stripe group relative grid cursor-default items-center border-b border-hairline px-4 transition-colors hover:bg-overlay-2"
              :class="[
                gridClass,
                rowHeightClass,
                selectedAddress === row.address ? 'bg-overlay-2' : '',
              ]"
              @click="sessions.selectAddress(row.address)"
            >
              <span
                v-if="selectedAddress === row.address"
                class="absolute inset-y-1 left-0 w-[2px] rounded-full bg-signal/70"
              />
              <div v-if="!isSlave" class="flex justify-center" @click.stop>
                <input
                  type="checkbox"
                  class="size-3 accent-[var(--warn)]"
                  :checked="selectedForAlerts.includes(row.address)"
                  @change="toggleSelected(row.address)"
                />
              </div>
              <span class="numeric text-[12px] text-muted-foreground">
                {{ formatAddress(row.address, settings.addressBase) }}
              </span>
              <div
                class="flex min-w-0 cursor-text items-center gap-2"
                :title="isSlave ? t('grid.editMapping') : t('grid.tips.meaning')"
                @dblclick.stop="openMeaning(row)"
              >
                <span
                  class="size-1 shrink-0 rounded-full"
                  :class="
                    row.point.dataType === 'f32' || row.point.dataType === 'f64'
                      ? 'bg-rx/70'
                      : 'bg-signal/60'
                  "
                />
                <span
                  class="truncate text-[12px]"
                  :class="row.point.meaning ? 'text-foreground' : 'text-muted-foreground'"
                >
                  {{ row.alias }}
                </span>
              </div>
              <span
                v-if="settings.showRawColumn"
                class="numeric cursor-pointer truncate text-[11px] text-muted-foreground"
                :title="isSlave ? t('grid.tips.simulatorValue') : t('grid.tips.trend')"
                @dblclick.stop="openValue(row)"
              >
                {{ row.words.length ? formatWord(row.words[0] ?? 0) : "—" }}
                <span v-if="row.words.length > 1" class="text-muted-foreground/70">
                  {{ row.words.slice(1).map(formatWord).join(" ") }}
                </span>
              </span>
              <div
                class="flex cursor-pointer justify-end"
                :title="isSlave ? t('grid.tips.simulatorValue') : t('grid.tips.trend')"
                @dblclick.stop="openValue(row)"
              >
                <span
                  class="numeric rounded px-1.5 py-0.5 text-[14px] font-medium tracking-tight"
                  :class="[
                    flashMap[row.address] ? 'value-flash' : '',
                    row.stale ? 'text-warn' : 'text-foreground',
                  ]"
                >
                  {{ row.formatted }}
                  <span v-if="row.point.unit" class="text-[10px] text-muted-foreground">
                    {{ row.point.unit }}
                  </span>
                </span>
              </div>
              <div
                v-if="!isSlave"
                class="flex cursor-pointer justify-center"
                :title="t('grid.tips.trend')"
                @dblclick.stop="openTrend(row.address)"
              >
                <Sparkline :points="row.history" :tone="sparkTone" :width="76" :height="22" />
              </div>
              <button
                type="button"
                class="numeric mx-auto rounded border px-1 py-0.5 text-[10.5px] font-medium uppercase tracking-wide transition-colors"
                :class="
                  row.point.dataType === 'f32' || row.point.dataType === 'f64'
                    ? 'border-rx/20 bg-rx/10 text-rx/90 hover:border-rx/40'
                    : 'border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground'
                "
                :title="t('grid.tips.type')"
                @dblclick.stop="openPoint(row)"
                @click.stop="openPoint(row)"
              >
                {{ row.point.dataType.toUpperCase() }}
              </button>
              <button
                type="button"
                class="numeric mx-auto text-[10.5px] uppercase text-muted-foreground transition-colors hover:text-foreground"
                :title="t('grid.tips.order')"
                @click.stop="openPoint(row)"
              >
                {{ row.point.byteOrder.toUpperCase() }}
              </button>
              <template v-if="!isSlave">
                <button
                  type="button"
                  class="mx-auto max-w-full truncate text-[11px] text-muted-foreground transition-colors hover:text-foreground"
                  :title="row.point.unit || t('grid.tips.unit')"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.unit || "—" }}
                </button>
                <button
                  type="button"
                  class="numeric mx-auto text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
                  :title="t('grid.tips.scale')"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.scale }}
                </button>
                <button
                  type="button"
                  class="numeric mx-auto text-[10.5px] text-muted-foreground transition-colors hover:text-foreground"
                  :title="t('grid.tips.offset')"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.offset }}
                </button>
                <div class="flex justify-center">
                  <button
                    type="button"
                    class="flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10.5px] transition-colors"
                    :class="
                      row.point.alerts.length
                        ? 'border-warn/25 bg-warn/10 text-warn hover:bg-warn/15'
                        : 'border-hairline bg-overlay-1 text-muted-foreground hover:text-foreground'
                    "
                    :title="t('grid.tips.alerts', { count: row.point.alerts.length })"
                    @click.stop="openAlerts([row.address])"
                  >
                    <BellRing class="size-3" />
                    <span class="numeric">{{ row.point.alerts.length }}</span>
                  </button>
                </div>
              </template>
              <template v-else>
                <button
                  type="button"
                  class="numeric mx-auto text-[10px] text-muted-foreground transition-colors hover:text-foreground"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.simulator ? simulatorModeLabel[row.point.simulator.mode] : "—" }}
                </button>
                <button
                  type="button"
                  class="numeric mx-auto text-[10px] text-muted-foreground transition-colors hover:text-foreground"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.simulator ? `${row.point.simulator.refreshMs}` : "—" }}
                </button>
                <button
                  type="button"
                  class="numeric mx-auto text-[10px] text-muted-foreground transition-colors hover:text-foreground"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.simulator ? row.point.simulator.min : "—" }}
                </button>
                <button
                  type="button"
                  class="numeric mx-auto text-[10px] text-muted-foreground transition-colors hover:text-foreground"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.simulator ? row.point.simulator.max : "—" }}
                </button>
                <button
                  type="button"
                  class="mx-auto text-[10px] transition-colors"
                  :class="row.point.simulator?.enabled ? 'text-sim' : 'text-muted-foreground'"
                  @click.stop="openPoint(row)"
                >
                  {{ row.point.simulator?.enabled ? t("grid.on") : t("grid.off") }}
                </button>
              </template>
              <div v-if="!isSlave" class="flex justify-center">
                <span
                  class="status-dot scale-75"
                  :class="row.stale || row.error ? 'text-warn' : 'text-signal/70'"
                />
              </div>
              <div v-else class="flex justify-center gap-1" @click.stop>
                <button
                  type="button"
                  class="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
                  :title="t('grid.editMapping')"
                  :aria-label="t('grid.editMapping')"
                  @click="openPoint(row)"
                >
                  <PencilLine class="size-3" />
                </button>
                <button
                  type="button"
                  class="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                  :title="t('grid.deleteMapping')"
                  :aria-label="t('grid.deleteMapping')"
                  @click="openMappingDelete(row)"
                >
                  <Trash2 class="size-3" />
                </button>
              </div>
            </div>
          </template>

          <div v-else class="flex h-52 flex-col items-center justify-center gap-2 text-center">
            <template v-if="isSlave">
              <span
                class="flex size-9 items-center justify-center rounded-xl border border-sim/25 bg-sim/10 text-sim"
              >
                <Plus class="size-4" />
              </span>
              <p class="text-[12px] text-muted-foreground">{{ t("grid.mappingEmpty") }}</p>
              <p class="max-w-sm text-[11px] leading-relaxed text-muted-foreground/80">
                {{ t("grid.mappingEmptyHint") }}
              </p>
              <Button size="sm" class="mt-1 gap-1 text-[11.5px]" @click="openSimulatorAdd">
                <Plus class="size-3" />
                {{ t("grid.addMapping") }}
              </Button>
            </template>
            <template v-else>
              <span class="status-dot status-dot-live text-signal" />
              <p class="text-[12px] text-muted-foreground">{{ t("grid.waitingFirst") }}</p>
              <p class="text-[11px] text-muted-foreground">
                {{
                  t("grid.pollingHint", {
                    interval: definition.intervalMs,
                    slave: definition.slaveId,
                  })
                }}
              </p>
            </template>
          </div>
        </div>
      </div>
    </template>

    <EmptyState
      v-else-if="sessions.bootError"
      :title="t('grid.backendUnavailable')"
      :description="sessions.bootError"
      class="m-auto"
    >
      <template #icon>
        <RefreshCw class="size-5" />
      </template>
      <Button variant="outline" size="sm" class="mt-1" @click="sessions.boot()">
        {{ t("grid.retryBackend") }}
      </Button>
    </EmptyState>

    <EmptyState
      v-else-if="sessions.activeSession && sessions.activeDefinitions.length === 0"
      :title="t('grid.noDefinitions')"
      :description="t('grid.noDefinitionsHint', { name: sessions.activeSession.name })"
      class="m-auto"
    >
      <template #icon>
        <Table2 class="size-5" />
      </template>
    </EmptyState>

    <EmptyState
      v-else
      :title="t('workspace.noSession')"
      :description="t('workspace.noSessionHint')"
    >
      <template #icon>
        <Table2 class="size-5" />
      </template>
      <Button size="sm" class="mt-1" @click="emit('new-session')">
        {{ t("workspace.newSession") }}
      </Button>
    </EmptyState>

    <AliasDialog
      v-model:open="aliasOpen"
      :definition="definition"
      :address="aliasRow?.address ?? 0"
      :current-alias="aliasRow?.alias ?? ''"
    />
    <PointDialog v-model:open="pointOpen" :definition="definition" :address="pointAddress" />
    <AlertRulesDialog
      v-model:open="alertOpen"
      :definition="definition"
      :addresses="alertAddresses"
    />
    <SimulatorPointDialog
      v-model:open="simulatorOpen"
      :definition="definition"
      :address="simulatorAddress"
    />
    <SlaveValueDialog
      v-model:open="slaveValueOpen"
      :definition="definition"
      :address="slaveValueRow?.address ?? 0"
      :current="slaveValueRow?.formatted ?? ''"
    />

    <AlertDialog
      :open="mappingDeleteOpen"
      @update:open="
        (open) => {
          if (!open) closeMappingDelete();
        }
      "
    >
      <AlertDialogContent
        class="border-glass-border bg-[color:var(--glass-strong)] backdrop-blur-2xl"
      >
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t("grid.deleteMappingTitle") }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{
              t("grid.deleteMappingDescription", {
                name:
                  mappingDeleteRow?.alias ??
                  t("grid.register", { address: mappingDeleteRow?.address ?? 0 }),
              })
            }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel :disabled="deletingMapping">
            {{ t("common.cancel") }}
          </AlertDialogCancel>
          <Button
            variant="destructive"
            class="text-white"
            :disabled="deletingMapping"
            @click="confirmMappingDelete"
          >
            {{ deletingMapping ? t("common.loading") : t("grid.deleteMapping") }}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </section>
</template>
