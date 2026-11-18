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
import { useI18n } from "vue-i18n";
import EmptyState from "@/components/common/EmptyState.vue";
import DefinitionDialog from "@/components/definition/DefinitionDialog.vue";
import { Separator } from "@/components/ui/separator";
import { parseCandidates } from "@/lib/decode-preview";
import {
  formatAddress,
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
const { t } = useI18n();
const editOpen = ref(false);

const definition = computed(() => sessions.activeDefinition);
const session = computed(() => sessions.activeSession);
const isSlave = computed(() => session.value?.role === "slave");
const snapshot = computed(() => sessions.activeSnapshot);
const firstValue = computed(() => snapshot.value?.values[0] ?? null);
const selectedAddress = computed(
  () => sessions.selectedAddress ?? firstValue.value?.address ?? null,
);
const selectedValue = computed(
  () =>
    snapshot.value?.values.find((value) => value.address === selectedAddress.value) ??
    firstValue.value,
);
const selectedMeaning = computed(() => {
  const address = selectedAddress.value;
  if (address === null) return t("inspector.waitingSample");
  return definition.value?.aliases[address] ?? t("inspector.register", { address });
});
const candidates = computed(() =>
  snapshot.value && selectedAddress.value !== null
    ? parseCandidates(snapshot.value.values, selectedAddress.value)
    : [],
);

const presetLabel = computed<Record<ByteOrder, string>>(() => ({
  abcd: t("inspector.order.abcd"),
  dcba: t("inspector.order.dcba"),
  badc: t("inspector.order.badc"),
  cdab: t("inspector.order.cdab"),
}));

const presetHint = computed<Record<ByteOrder, string>>(() => ({
  abcd: t("inspector.order.abcdHint"),
  dcba: t("inspector.order.dcbaHint"),
  badc: t("inspector.order.badcHint"),
  cdab: t("inspector.order.cdabHint"),
}));

interface ByteBox {
  letter: string;
  hex: string;
}

const byteBoxes = computed<ByteBox[]>(() => {
  const words = selectedValue.value?.words ?? [];
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
    {
      label: t("inspector.functionCode"),
      value: formatFunctionCode(def.functionCode),
      icon: Binary,
    },
    { label: t("inspector.slaveId"), value: def.slaveId.toString(), icon: Hash },
    {
      label: t("inspector.range"),
      value: formatRange(def.startAddress, def.quantity, settings.addressBase),
      icon: Hash,
    },
    { label: t("inspector.quantity"), value: def.quantity.toString(), icon: Hash },
    { label: t("inspector.dataType"), value: def.dataType.toUpperCase(), icon: Binary },
    {
      label: t("inspector.byteOrder"),
      value: presetLabel.value[def.byteOrder],
      icon: SlidersHorizontal,
    },
    { label: t("inspector.interval"), value: formatInterval(def.intervalMs), icon: Clock3 },
    {
      label: t("inspector.aliases"),
      value: t("inspector.aliasCount", { count: Object.keys(def.aliases).length }),
      icon: Hash,
    },
  ];
});
</script>

<template>
  <aside class="hidden w-[306px] shrink-0 flex-col border-l border-hairline bg-overlay-1 xl:flex">
    <div class="flex h-9 shrink-0 items-center gap-2 border-b border-hairline px-4">
      <SlidersHorizontal class="size-3.5 text-muted-foreground" />
      <span class="label-caps">{{ t("inspector.title") }}</span>
      <button
        v-if="definition && !isSlave"
        type="button"
        class="ml-auto flex size-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
        :aria-label="t('inspector.editDefinition')"
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
            {{ t("inspector.presetsHint") }}
          </p>
        </div>

        <Separator class="my-4 bg-overlay-3" />

        <div class="space-y-2">
          <div class="flex items-center justify-between gap-2">
            <span class="label-caps">{{ t("inspector.parsePreview") }}</span>
            <span class="numeric text-[10.5px] text-muted-foreground">
              @{{
                selectedAddress === null
                  ? "—"
                  : formatAddress(selectedAddress, settings.addressBase)
              }}
            </span>
          </div>
          <p class="text-[11px] leading-relaxed text-muted-foreground">
            {{ t("inspector.parseHint") }}
          </p>
          <div
            v-if="candidates.length"
            class="scrollbar-thin max-h-[236px] space-y-1 overflow-y-auto"
          >
            <div
              v-for="candidate in candidates"
              :key="`${candidate.dataType}-${candidate.byteOrder}`"
              class="flex items-center gap-2 rounded-lg border border-hairline bg-overlay-1 px-2 py-1.5"
            >
              <span
                class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 text-[10px] font-medium text-signal-dim"
              >
                {{ candidate.label }}
              </span>
              <span class="numeric text-[10px] text-muted-foreground">
                {{ candidate.byteOrder.toUpperCase() }}
              </span>
              <span class="numeric ml-auto truncate text-[11.5px] text-foreground">
                {{ candidate.value }}
              </span>
            </div>
          </div>
          <p
            v-else
            class="rounded-lg border border-dashed border-hairline px-2.5 py-2 text-[11px] text-muted-foreground"
          >
            {{ t("inspector.waitingSelected") }}
          </p>
        </div>

        <Separator class="my-4 bg-overlay-3" />

        <div class="space-y-2">
          <div class="flex items-center gap-1.5">
            <Gauge class="size-3 text-muted-foreground" />
            <span class="label-caps">{{ t("inspector.liveSample") }}</span>
          </div>
          <div
            class="rounded-xl border border-hairline bg-gradient-to-br from-overlay-2 to-transparent p-3"
          >
            <div class="flex items-end justify-between gap-2">
              <div class="min-w-0">
                <p class="truncate text-[11px] text-muted-foreground">
                  {{ selectedValue ? selectedMeaning : t("inspector.waitingSample") }}
                </p>
                <p
                  class="numeric mt-1 text-[22px] font-semibold leading-none tracking-tight text-foreground"
                >
                  {{ selectedValue?.formatted ?? "—" }}
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
                {{
                  selectedValue?.words.length ? selectedValue.words.map(formatWord).join(" ") : "—"
                }}
              </span>
              <ArrowRight class="size-3 text-muted-foreground/70" />
              <span class="numeric text-[11px] text-foreground">{{
                selectedValue?.formatted ?? "—"
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
          {{ t("inspector.writeHint") }}
        </p>
      </div>
    </template>

    <EmptyState
      v-else
      :title="t('inspector.selectTitle')"
      :description="t('inspector.selectHint')"
      class="m-auto"
    >
      <template #icon>
        <SlidersHorizontal class="size-4" />
      </template>
    </EmptyState>

    <DefinitionDialog v-model:open="editOpen" :definition="definition" />
  </aside>
</template>
