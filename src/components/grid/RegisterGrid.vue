<script setup lang="ts">
import { PencilLine, RefreshCw, Table2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import AliasDialog from "@/components/definition/AliasDialog.vue";
import FormatDialog from "@/components/definition/FormatDialog.vue";
import SlaveValueDialog from "@/components/definition/SlaveValueDialog.vue";
import Sparkline from "@/components/grid/Sparkline.vue";
import { Button } from "@/components/ui/button";
import { formatAddress, formatHexBytes, formatLatency, formatWord, historyKey } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Definition, RegisterValue } from "@/types/models";
import type { WriteTarget } from "@/types/ui";

const emit = defineEmits<{
  (event: "write", target: WriteTarget): void;
  (event: "new-session"): void;
  (event: "open-trend"): void;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const selectedAddress = ref<number | null>(null);
const aliasOpen = ref(false);
const aliasRow = ref<GridRow | null>(null);
const formatOpen = ref(false);
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
  format: string;
  history: number[];
}

const rows = computed<GridRow[]>(() => {
  const def = definition.value;
  const snap = snapshot.value;
  if (!def || !snap) return [];
  return snap.values.map((value) => ({
    ...value,
    alias: def.aliases[value.address] ?? `寄存器 ${value.address}`,
    raw: formatHexBytes(value.words.flatMap((word) => [(word >> 8) & 0xff, word & 0xff])),
    format: def.dataType.toUpperCase(),
    history: sessions.history[historyKey(def.id, value.address)] ?? [],
  }));
});

const sparkTone = computed(() => {
  if (session.value?.role === "slave") return "var(--sim)";
  if (session.value?.transport === "serial") return "var(--rx)";
  return "var(--signal)";
});

const gridClass = computed(() => {
  if (isSlave.value) {
    return settings.showRawColumn
      ? "grid-cols-[72px_minmax(130px,1.2fr)_110px_minmax(110px,0.9fr)_58px_26px]"
      : "grid-cols-[72px_minmax(130px,1.2fr)_minmax(110px,0.9fr)_58px_26px]";
  }
  return settings.showRawColumn
    ? "grid-cols-[72px_minmax(130px,1.2fr)_110px_minmax(110px,0.9fr)_92px_58px_26px]"
    : "grid-cols-[72px_minmax(130px,1.2fr)_minmax(110px,0.9fr)_92px_58px_26px]";
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
    selectedAddress.value = null;
  },
);

function openWrite(row: GridRow): void {
  if (!definition.value) return;
  if (isSlave.value) {
    toast.info("从站映射编辑将在 P7 阶段启用。");
    return;
  }
  emit("write", {
    address: row.address,
    alias: row.alias,
    current: row.formatted,
    dataType: definition.value.dataType,
  });
}

const selectedRow = computed(
  () => rows.value.find((row) => row.address === selectedAddress.value) ?? null,
);

function openSelectedWrite(): void {
  if (selectedRow.value) openWrite(selectedRow.value);
}

function openTrend(): void {
  if (definition.value) emit("open-trend");
}

function openAlias(row: GridRow): void {
  aliasRow.value = row;
  aliasOpen.value = true;
}

function openFormat(): void {
  if (definition.value && !isSlave.value) formatOpen.value = true;
}

function openSlaveValue(row: GridRow): void {
  slaveValueRow.value = row;
  slaveValueOpen.value = true;
}

function openValue(row: GridRow): void {
  if (isSlave.value) openSlaveValue(row);
  else openTrend();
}

function openFormatCell(row: GridRow): void {
  if (isSlave.value) openSlaveValue(row);
  else openFormat();
}
</script>

<template>
  <section class="relative flex h-full min-h-0 min-w-0 flex-col">
    <template v-if="definition">
      <div
        class="flex h-9 shrink-0 items-center gap-2 border-b border-hairline px-4 text-[11px] text-muted-foreground"
      >
        <span class="label-caps">寄存器视图</span>
        <span class="text-foreground/15">/</span>
        <span class="numeric">{{ rows.length }} 值</span>
        <span class="numeric text-muted-foreground">· {{ definition.quantity }} 寄存器</span>
        <span class="ml-auto flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            class="h-7 gap-1 border-hairline-strong bg-overlay-1 px-2 text-[11px]"
            :disabled="!selectedRow || isSlave"
            aria-label="写入选中寄存器"
            title="先选择一行，再打开写入弹窗"
            @click="openSelectedWrite"
          >
            <PencilLine class="size-3" />
            写入值
          </Button>
          <span class="status-dot" :class="snapshot?.error ? 'text-error' : 'text-signal'" />
          <span>{{ snapshot?.error ? "读取异常" : snapshot ? "实时" : "等待采样" }}</span>
          <span v-if="snapshot" class="numeric text-muted-foreground">
            {{ formatLatency(snapshot.latencyMs) }}
          </span>
        </span>
      </div>

      <div class="min-h-0 flex-1 overflow-auto scrollbar-thin">
        <div class="min-w-[620px]">
          <div
            class="sticky top-0 z-10 grid items-center border-b border-hairline bg-background/80 px-4 py-1.5 backdrop-blur-md"
            :class="gridClass"
          >
            <span class="label-caps">地址</span>
            <span class="label-caps">别名 / 点位</span>
            <span v-if="settings.showRawColumn" class="label-caps">原始 HEX</span>
            <span class="label-caps text-right">数值</span>
            <span v-if="!isSlave" class="label-caps text-center">趋势</span>
            <span class="label-caps text-center">格式</span>
            <span class="label-caps text-center">状态</span>
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
              @click="selectedAddress = row.address"
            >
              <span
                v-if="selectedAddress === row.address"
                class="absolute inset-y-1 left-0 w-[2px] rounded-full bg-signal/70"
              />
              <span class="numeric text-[12px] text-muted-foreground">
                {{ formatAddress(row.address, settings.addressBase) }}
              </span>
              <div
                class="flex min-w-0 cursor-text items-center gap-2"
                title="双击编辑别名"
                @dblclick.stop="openAlias(row)"
              >
                <span
                  class="size-1 shrink-0 rounded-full"
                  :class="
                    definition.dataType === 'f32' || definition.dataType === 'f64'
                      ? 'bg-rx/70'
                      : 'bg-signal/60'
                  "
                />
                <span
                  class="truncate text-[12px]"
                  :class="
                    definition.aliases[row.address] ? 'text-foreground' : 'text-muted-foreground'
                  "
                >
                  {{ row.alias }}
                </span>
              </div>
              <span
                v-if="settings.showRawColumn"
                class="numeric cursor-pointer truncate text-[11px] text-muted-foreground"
                :title="isSlave ? '双击修改模拟值' : '双击打开趋势大图'"
                @dblclick.stop="openValue(row)"
              >
                {{ row.words.length ? formatWord(row.words[0] ?? 0) : "—" }}
                <span v-if="row.words.length > 1" class="text-muted-foreground/70">
                  {{ row.words.slice(1).map(formatWord).join(" ") }}
                </span>
              </span>
              <div
                class="flex cursor-pointer justify-end"
                :title="isSlave ? '双击修改模拟值' : '双击打开趋势大图'"
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
                </span>
              </div>
              <div
                v-if="!isSlave"
                class="flex cursor-pointer justify-center"
                title="双击打开趋势大图"
                @dblclick.stop="openTrend"
              >
                <Sparkline :points="row.history" :tone="sparkTone" :width="84" :height="22" />
              </div>
              <div
                class="flex cursor-pointer justify-center"
                :title="isSlave ? '双击修改模拟值与格式' : '双击修改数据类型 / 字节序'"
                @dblclick.stop="openFormatCell(row)"
              >
                <span
                  class="numeric rounded border px-1 py-0.5 text-[11px] font-medium uppercase tracking-wide"
                  :class="
                    definition.dataType === 'f32' || definition.dataType === 'f64'
                      ? 'border-rx/20 bg-rx/10 text-rx/90'
                      : 'border-hairline bg-overlay-1 text-muted-foreground'
                  "
                >
                  {{ row.format }}
                </span>
              </div>
              <div class="flex justify-center">
                <span
                  class="status-dot scale-75"
                  :class="row.stale || row.error ? 'text-warn' : 'text-signal/70'"
                />
              </div>
            </div>
          </template>

          <div v-else class="flex h-52 flex-col items-center justify-center gap-2 text-center">
            <span class="status-dot status-dot-live text-signal" />
            <p class="text-[12px] text-muted-foreground">等待首次采样…</p>
            <p class="text-[11px] text-muted-foreground">
              轮询周期 {{ definition.intervalMs }} ms · 从站 {{ definition.slaveId }}
            </p>
          </div>
        </div>
      </div>
    </template>

    <EmptyState
      v-else-if="sessions.bootError"
      title="后台尚未就绪"
      :description="sessions.bootError"
      class="m-auto"
    >
      <template #icon>
        <RefreshCw class="size-5" />
      </template>
      <Button variant="outline" size="sm" class="mt-1" @click="sessions.boot()"
        >重试连接后台</Button
      >
    </EmptyState>

    <EmptyState
      v-else-if="sessions.activeSession && sessions.activeDefinitions.length === 0"
      title="还没有读取定义"
      :description="`在左侧「读取定义」旁点击 +，为「${sessions.activeSession.name}」配置功能码与地址后开始轮询。`"
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
    <FormatDialog v-model:open="formatOpen" :definition="definition" />
    <SlaveValueDialog
      v-model:open="slaveValueOpen"
      :definition="definition"
      :address="slaveValueRow?.address ?? 0"
      :current="slaveValueRow?.formatted ?? ''"
    />
  </section>
</template>
