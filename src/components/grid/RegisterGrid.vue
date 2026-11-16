<script setup lang="ts">
import { RefreshCw, Table2 } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
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
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const selectedAddress = ref<number | null>(null);
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

const gridClass = computed(() =>
  settings.showRawColumn
    ? "grid-cols-[72px_minmax(130px,1.2fr)_110px_minmax(110px,0.9fr)_92px_58px_26px]"
    : "grid-cols-[72px_minmax(130px,1.2fr)_minmax(110px,0.9fr)_92px_58px_26px]",
);

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
</script>

<template>
  <section class="relative flex h-full min-h-0 min-w-0 flex-col">
    <template v-if="definition">
      <div
        class="flex h-9 shrink-0 items-center gap-2 border-b border-white/[0.05] px-4 text-[10px] text-muted-foreground/60"
      >
        <span class="label-caps">寄存器视图</span>
        <span class="text-white/15">/</span>
        <span class="numeric">{{ rows.length }} 值</span>
        <span class="numeric text-muted-foreground/40">· {{ definition.quantity }} 寄存器</span>
        <span class="ml-auto flex items-center gap-2">
          <span class="status-dot" :class="snapshot?.error ? 'text-error' : 'text-signal'" />
          <span>{{ snapshot?.error ? "读取异常" : snapshot ? "实时" : "等待采样" }}</span>
          <span v-if="snapshot" class="numeric text-muted-foreground/50">
            {{ formatLatency(snapshot.latencyMs) }}
          </span>
        </span>
      </div>

      <div class="min-h-0 flex-1 overflow-auto scrollbar-thin">
        <div class="min-w-[620px]">
          <div
            class="sticky top-0 z-10 grid items-center border-b border-white/[0.06] bg-background/80 px-4 py-1.5 backdrop-blur-md"
            :class="gridClass"
          >
            <span class="label-caps">地址</span>
            <span class="label-caps">别名 / 点位</span>
            <span v-if="settings.showRawColumn" class="label-caps">原始 HEX</span>
            <span class="label-caps text-right">数值</span>
            <span class="label-caps text-center">趋势</span>
            <span class="label-caps text-center">格式</span>
            <span class="label-caps text-center">状态</span>
          </div>

          <template v-if="rows.length">
            <div
              v-for="row in rows"
              :key="row.address"
              class="row-stripe group relative grid cursor-default items-center border-b border-white/[0.035] px-4 transition-colors hover:bg-white/[0.035]"
              :class="[
                gridClass,
                rowHeightClass,
                selectedAddress === row.address ? 'bg-white/[0.05]' : '',
              ]"
              @click="selectedAddress = row.address"
              @dblclick="openWrite(row)"
            >
              <span
                v-if="selectedAddress === row.address"
                class="absolute inset-y-1 left-0 w-[2px] rounded-full bg-signal/70"
              />
              <span class="numeric text-[11px] text-muted-foreground/75">
                {{ formatAddress(row.address, settings.addressBase) }}
              </span>
              <div class="flex min-w-0 items-center gap-2">
                <span
                  class="size-1 shrink-0 rounded-full"
                  :class="
                    definition.dataType === 'f32' || definition.dataType === 'f64'
                      ? 'bg-rx/70'
                      : 'bg-signal/60'
                  "
                />
                <span
                  class="truncate text-[11.5px]"
                  :class="
                    definition.aliases[row.address]
                      ? 'text-foreground/90'
                      : 'text-muted-foreground/70'
                  "
                >
                  {{ row.alias }}
                </span>
              </div>
              <span
                v-if="settings.showRawColumn"
                class="numeric truncate text-[10px] text-muted-foreground/70"
              >
                {{ formatWord(row.words[0] ?? 0) }}
                <span v-if="row.words.length > 1" class="text-muted-foreground/35">
                  {{ row.words.slice(1).map(formatWord).join(" ") }}
                </span>
              </span>
              <div class="flex justify-end">
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
              <div class="flex justify-center">
                <Sparkline :points="row.history" :tone="sparkTone" :width="84" :height="22" />
              </div>
              <div class="flex justify-center">
                <span
                  class="numeric rounded border px-1 py-0.5 text-[8.5px] font-medium uppercase tracking-wide"
                  :class="
                    definition.dataType === 'f32' || definition.dataType === 'f64'
                      ? 'border-rx/20 bg-rx/10 text-rx/90'
                      : 'border-white/[0.06] bg-white/[0.03] text-muted-foreground/70'
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
            <p class="text-[11.5px] text-muted-foreground">等待首次采样…</p>
            <p class="text-[10px] text-muted-foreground/60">
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
  </section>
</template>
