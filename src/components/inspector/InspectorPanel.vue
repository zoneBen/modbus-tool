<script setup lang="ts">
import {
  ArrowRight,
  Binary,
  Clock3,
  Gauge,
  Hash,
  PencilLine,
  SlidersHorizontal,
} from "@lucide/vue";
import { computed, ref } from "vue";
import EmptyState from "@/components/common/EmptyState.vue";
import DefinitionDialog from "@/components/definition/DefinitionDialog.vue";
import { Separator } from "@/components/ui/separator";
import {
  formatFunctionCode,
  formatInterval,
  formatLatency,
  formatRange,
  formatTime,
  formatWord,
} from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ByteOrder } from "@/types/models";

const sessions = useSessionsStore();
const settings = useSettingsStore();
const editOpen = ref(false);

const definition = computed(() => sessions.activeDefinition);
const session = computed(() => sessions.activeSession);
const isSlave = computed(() => session.value?.role === "slave");
const snapshot = computed(() => sessions.activeSnapshot);
const firstValue = computed(() => snapshot.value?.values[0] ?? null);

const presetLabel: Record<ByteOrder, string> = {
  abcd: "ABCD · 大端",
  dcba: "DCBA · 小端",
  badc: "BADC · 字节交换",
  cdab: "CDAB · 字交换",
};

const presetHint: Record<ByteOrder, string> = {
  abcd: "高字节在前，高字在前",
  dcba: "低字节在前，低字在前",
  badc: "字内字节交换",
  cdab: "32 位字序交换",
};

interface ByteBox {
  letter: string;
  hex: string;
}

const byteBoxes = computed<ByteBox[]>(() => {
  const words = firstValue.value?.words ?? [];
  if (!words.length || !definition.value) return [];
  const bytes = words.flatMap((word) => [(word >> 8) & 0xff, word & 0xff]);
  const canonical = ["A", "B", "C", "D"].map((letter, index) => ({
    letter,
    hex: bytes[index]?.toString(16).toUpperCase().padStart(2, "0") ?? "—",
  }));
  switch (definition.value.byteOrder) {
    case "badc":
      return [canonical[1], canonical[0], canonical[3], canonical[2]].filter(
        (box) => box.hex !== "—",
      );
    case "cdab":
      return [canonical[2], canonical[3], canonical[0], canonical[1]].filter(
        (box) => box.hex !== "—",
      );
    case "dcba":
      return [canonical[3], canonical[2], canonical[1], canonical[0]].filter(
        (box) => box.hex !== "—",
      );
    default:
      return canonical.filter((box) => box.hex !== "—");
  }
});

const fields = computed(() => {
  const def = definition.value;
  if (!def) return [];
  return [
    { label: "功能码", value: formatFunctionCode(def.functionCode), icon: Binary },
    { label: "从站 ID", value: def.slaveId.toString(), icon: Hash },
    {
      label: "地址范围",
      value: formatRange(def.startAddress, def.quantity, settings.addressBase),
      icon: Hash,
    },
    { label: "寄存器数", value: def.quantity.toString(), icon: Hash },
    { label: "数据类型", value: def.dataType.toUpperCase(), icon: Binary },
    { label: "字节序", value: presetLabel[def.byteOrder], icon: SlidersHorizontal },
    { label: "轮询周期", value: formatInterval(def.intervalMs), icon: Clock3 },
    {
      label: "别名",
      value: `${Object.keys(def.aliases).length} 项`,
      icon: Hash,
    },
  ];
});
</script>

<template>
  <aside class="hidden w-[306px] shrink-0 flex-col border-l border-hairline bg-overlay-1 xl:flex">
    <div class="flex h-9 shrink-0 items-center gap-2 border-b border-hairline px-4">
      <SlidersHorizontal class="size-3.5 text-muted-foreground" />
      <span class="label-caps">定义检查器</span>
      <button
        v-if="definition && !isSlave"
        type="button"
        class="ml-auto flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
        aria-label="编辑读取定义"
        @click="editOpen = true"
      >
        <PencilLine class="size-3" />
      </button>
      <span v-if="definition" class="numeric text-[11px] text-muted-foreground">
        {{ definition.id.slice(-6) }}
      </span>
    </div>

    <template v-if="definition">
      <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-4 py-4">
        <div>
          <p class="text-[13px] font-semibold">{{ definition.name }}</p>
          <p class="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground">
            <span>{{ session?.name }}</span>
            <span class="text-foreground/15">·</span>
            <span class="uppercase">{{ session?.framing }}</span>
            <span class="text-foreground/15">·</span>
            <span class="numeric">{{ session?.endpoint }}</span>
          </p>
        </div>

        <div class="mt-4 grid grid-cols-2 gap-1.5">
          <div
            v-for="field in fields"
            :key="field.label"
            class="rounded-lg border border-hairline bg-overlay-1 px-2.5 py-2"
          >
            <p class="text-[11px] uppercase tracking-wider text-muted-foreground">
              {{ field.label }}
            </p>
            <p class="numeric mt-1 truncate text-[11.5px] text-foreground">{{ field.value }}</p>
          </div>
        </div>

        <Separator class="my-4 bg-overlay-3" />

        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <span class="label-caps">{{ presetLabel[definition.byteOrder] }}</span>
            <span class="text-[11px] text-muted-foreground">{{
              presetHint[definition.byteOrder]
            }}</span>
          </div>
          <div class="flex items-center gap-1.5">
            <div
              v-for="box in byteBoxes"
              :key="box.letter"
              class="flex flex-1 flex-col items-center rounded-lg border border-hairline bg-well py-1.5"
            >
              <span class="text-[11px] text-muted-foreground">{{ box.letter }}</span>
              <span class="numeric text-[12px] text-foreground">{{ box.hex }}</span>
            </div>
          </div>
          <p class="text-[11px] leading-relaxed text-muted-foreground">
            四组预设仅映射字节序与字序，解析逻辑由 Rust 核心统一实现。
          </p>
        </div>

        <Separator class="my-4 bg-overlay-3" />

        <div class="space-y-2">
          <div class="flex items-center gap-1.5">
            <Gauge class="size-3 text-muted-foreground" />
            <span class="label-caps">实时采样</span>
          </div>
          <div
            class="rounded-xl border border-hairline bg-gradient-to-br from-overlay-2 to-transparent p-3"
          >
            <div class="flex items-end justify-between gap-2">
              <div class="min-w-0">
                <p class="truncate text-[11px] text-muted-foreground">
                  {{
                    firstValue
                      ? (definition.aliases[firstValue.address] ?? `寄存器 ${firstValue.address}`)
                      : "等待采样"
                  }}
                </p>
                <p
                  class="numeric mt-1 text-[22px] font-semibold leading-none tracking-tight text-foreground"
                >
                  {{ firstValue?.formatted ?? "—" }}
                </p>
              </div>
              <div class="shrink-0 text-right">
                <p class="numeric text-[11px] text-signal/85">
                  {{ formatLatency(snapshot?.latencyMs ?? 0) }}
                </p>
                <p class="numeric mt-1 text-[11px] text-muted-foreground">
                  {{ snapshot ? formatTime(snapshot.timestamp) : "—" }}
                </p>
              </div>
            </div>
            <div class="mt-3 flex items-center gap-2 border-t border-hairline pt-2">
              <span class="numeric text-[11px] text-muted-foreground">
                {{ firstValue?.words.length ? firstValue.words.map(formatWord).join(" ") : "—" }}
              </span>
              <ArrowRight class="size-3 text-muted-foreground/70" />
              <span class="numeric text-[11px] text-foreground">{{
                firstValue?.formatted ?? "—"
              }}</span>
            </div>
            <p
              v-if="snapshot?.error"
              class="mt-2 rounded-md border border-error/25 bg-error/10 px-2 py-1 text-[11px] leading-relaxed text-error"
            >
              {{ snapshot.error }}
            </p>
          </div>
        </div>

        <p class="mt-4 text-[11px] leading-relaxed text-muted-foreground">
          写操作通过寄存器表格上方的「写入值」按钮执行；双击寄存器行可打开趋势大图。
        </p>
      </div>
    </template>

    <EmptyState
      v-else
      title="选择一个读取定义"
      description="在左侧定义列表中选择一项，查看解析参数与实时采样。"
      class="m-auto"
    >
      <template #icon>
        <SlidersHorizontal class="size-4" />
      </template>
    </EmptyState>

    <DefinitionDialog v-model:open="editOpen" :definition="definition" />
  </aside>
</template>
