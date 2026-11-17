<script setup lang="ts">
import { SlidersHorizontal } from "@lucide/vue";
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
import {
  BYTE_ORDER_OPTIONS,
  DATA_TYPE_OPTIONS,
  resolvePoints,
  toDefinitionInput,
} from "@/lib/definition";
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ByteOrder, DataType, Definition, RegisterPoint } from "@/types/models";

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
const unit = ref("");
const scale = ref("1");
const offset = ref("0");
const saving = ref(false);

const point = computed<RegisterPoint | null>(() => {
  const definition = props.definition;
  if (!definition) return null;
  const existing = definition.points.find((entry) => entry.address === props.address);
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

const addressLabel = computed(() => formatAddress(props.address, settings.addressBase));

watch(
  () => [open.value, props.address, props.definition?.id] as const,
  () => {
    if (!open.value || !point.value) return;
    dataType.value = point.value.dataType;
    byteOrder.value = point.value.byteOrder;
    meaning.value = point.value.meaning;
    unit.value = point.value.unit;
    scale.value = String(point.value.scale);
    offset.value = String(point.value.offset);
  },
  { immediate: true },
);

async function save(): Promise<void> {
  const definition = props.definition;
  if (!definition || saving.value) return;
  const parsedScale = Number(scale.value);
  const parsedOffset = Number(offset.value);
  if (!Number.isFinite(parsedScale) || !Number.isFinite(parsedOffset)) {
    toast.error(t("pointEditor.invalidNumbers"));
    return;
  }
  const updated: RegisterPoint = {
    ...(point.value ?? {
      address: props.address,
      meaning: "",
      unit: "",
      scale: 1,
      offset: 0,
      alerts: [],
    }),
    address: props.address,
    dataType: dataType.value,
    byteOrder: byteOrder.value,
    meaning: meaning.value.trim(),
    unit: unit.value.trim(),
    scale: parsedScale,
    offset: parsedOffset,
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
    toast.success(t("pointEditor.updated"), {
      description: t("pointEditor.updatedDescription", {
        address: addressLabel.value,
        type: dataType.value.toUpperCase(),
        order: byteOrder.value.toUpperCase(),
      }),
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error(t("pointEditor.updateFailed"), {
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
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[480px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <SlidersHorizontal class="size-3.5" />
          </span>
          {{ t("pointEditor.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{
            t("pointEditor.description", {
              name: definition?.name ?? t("pointEditor.noDefinition"),
              address: addressLabel,
            })
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("pointEditor.dataType") }}
            </Label>
            <FormSelect v-model="dataType" :options="DATA_TYPE_OPTIONS" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("pointEditor.order") }}</Label>
            <FormSelect v-model="byteOrder" :options="BYTE_ORDER_OPTIONS" />
          </div>
          <div class="col-span-2 space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("pointEditor.meaning") }}</Label>
            <Input
              v-model="meaning"
              :placeholder="t('pointEditor.meaningPlaceholder')"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("pointEditor.unit") }}</Label>
            <Input
              v-model="unit"
              :placeholder="t('pointEditor.unitPlaceholder')"
              class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="grid grid-cols-2 gap-2">
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("pointEditor.scale") }}</Label>
              <Input
                v-model="scale"
                inputmode="decimal"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
            <div class="space-y-1.5">
              <Label class="text-[11px] text-muted-foreground">{{ t("pointEditor.offset") }}</Label>
              <Input
                v-model="offset"
                inputmode="decimal"
                class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
            </div>
          </div>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ t("pointEditor.hint") }}
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
          {{ saving ? t("pointEditor.saving") : t("pointEditor.save") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
