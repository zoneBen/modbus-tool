<script setup lang="ts">
import { Activity } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import FormSelect from "@/components/common/FormSelect.vue";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  BYTE_ORDER_OPTIONS,
  DATA_TYPE_OPTIONS,
  resolvePoints,
  toDefinitionInput,
} from "@/lib/definition";
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type {
  ByteOrder,
  DataType,
  Definition,
  RegisterPoint,
  SimulatorMode,
  SimulatorPointConfig,
} from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
  address: number;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();
const dataType = ref<DataType>("u16");
const byteOrder = ref<ByteOrder>("abcd");
const meaning = ref("");
const mode = ref<SimulatorMode>("fixed");
const refreshMs = ref(1000);
const min = ref(0);
const max = ref(100);
const enabled = ref(false);
const saving = ref(false);

const numericTypes = DATA_TYPE_OPTIONS.filter((option) =>
  ["bool", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64"].includes(
    option.value,
  ),
);

const modeOptions = computed<Array<{ label: string; value: SimulatorMode }>>(() => [
  { label: t("simulatorPoint.modes.fixed"), value: "fixed" },
  { label: t("simulatorPoint.modes.increment"), value: "increment" },
  { label: t("simulatorPoint.modes.decrement"), value: "decrement" },
  { label: t("simulatorPoint.modes.random"), value: "random" },
]);

const addressLabel = computed(() => formatAddress(props.address, settings.addressBase));
const point = computed<RegisterPoint | null>(() => {
  const definition = props.definition;
  if (!definition) return null;
  const points = definition.points.length ? definition.points : resolvePoints(definition);
  const existing = points.find((entry) => entry.address === props.address);
  if (existing) return existing;
  return {
    address: props.address,
    dataType: definition.dataType,
    byteOrder: definition.byteOrder,
    meaning: definition.aliases[props.address] ?? "",
    unit: "",
    scale: 1,
    offset: 0,
    alerts: [],
  };
});

watch(
  () => [open.value, props.address, props.definition?.id] as const,
  () => {
    if (!open.value || !point.value) return;
    dataType.value = point.value.dataType;
    byteOrder.value = point.value.byteOrder;
    meaning.value = point.value.meaning;
    const simulator = point.value.simulator ?? {
      mode: "fixed",
      refreshMs: 1000,
      min: 0,
      max: 100,
      enabled: false,
    };
    mode.value = simulator.mode;
    refreshMs.value = simulator.refreshMs;
    min.value = simulator.min;
    max.value = simulator.max;
    enabled.value = simulator.enabled;
  },
  { immediate: true },
);

async function save(): Promise<void> {
  const definition = props.definition;
  if (!definition || saving.value) return;
  if (!Number.isFinite(min.value) || !Number.isFinite(max.value)) {
    toast.error(t("simulatorPoint.invalidRange"));
    return;
  }
  if (min.value > max.value) {
    toast.error(t("simulatorPoint.minGreaterMax"));
    return;
  }
  const simulator: SimulatorPointConfig = {
    mode: mode.value,
    refreshMs: Math.max(20, Math.trunc(Number(refreshMs.value) || 1000)),
    min: Number(min.value),
    max: Number(max.value),
    enabled: enabled.value,
  };
  const basePoint = point.value;
  if (!basePoint) return;
  const updated: RegisterPoint = {
    ...basePoint,
    dataType: dataType.value,
    byteOrder: byteOrder.value,
    meaning: meaning.value.trim(),
    simulator,
  };
  const basePoints = definition.points.length ? definition.points : resolvePoints(definition);
  const points = basePoints.some((entry) => entry.address === props.address)
    ? basePoints.map((entry) => (entry.address === props.address ? updated : entry))
    : [...basePoints, updated].sort((left, right) => left.address - right.address);
  const aliases = { ...definition.aliases };
  if (updated.meaning) aliases[props.address] = updated.meaning;
  else delete aliases[props.address];
  saving.value = true;
  try {
    await sessions.updateDefinition(definition.id, {
      ...toDefinitionInput(definition),
      aliases,
      points,
    });
    toast.success(t("simulatorPoint.updated"), {
      description: t("simulatorPoint.updatedDescription"),
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error(t("simulatorPoint.updateFailed"), {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[560px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-sim/25 bg-sim/10 text-sim"
          >
            <Activity class="size-3.5" />
          </span>
          {{ t("simulatorPoint.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{
            t("simulatorPoint.description", {
              name: definition?.name ?? t("simulatorPoint.noDefinition"),
              address: addressLabel,
            })
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="scrollbar-thin max-h-[65vh] space-y-4 overflow-y-auto px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.meaning") }}
            </Label>
            <Input
              v-model="meaning"
              :placeholder="t('simulatorPoint.meaningPlaceholder')"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.dataType") }}
            </Label>
            <FormSelect v-model="dataType" :options="numericTypes" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.order") }}
            </Label>
            <FormSelect v-model="byteOrder" :options="BYTE_ORDER_OPTIONS" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.mode") }}
            </Label>
            <FormSelect v-model="mode" :options="modeOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.refresh") }}
            </Label>
            <Input
              v-model.number="refreshMs"
              type="number"
              min="20"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="flex items-end justify-between gap-2 pb-1">
            <span class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.startRefresh") }}
            </span>
            <Switch v-model="enabled" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.min") }}
            </Label>
            <Input
              v-model.number="min"
              type="number"
              step="any"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("simulatorPoint.max") }}
            </Label>
            <Input
              v-model.number="max"
              type="number"
              step="any"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ t("simulatorPoint.hint") }}
          {{ t("simulatorPoint.hintSecond") }}
        </p>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            {{ t("common.cancel") }}
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[96px] text-[12px]" :disabled="saving" @click="save">
          {{ saving ? t("simulatorPoint.saving") : t("simulatorPoint.save") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
