<script setup lang="ts">
import { CircleAlert, Plus, SlidersHorizontal, X } from "@lucide/vue";
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
import { dataTypeSpan } from "@/lib/definition";
import { useSessionsStore } from "@/stores/sessions";
import type {
  ByteOrder,
  DataType,
  Definition,
  DefinitionInput,
  FunctionCode,
} from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
}>();

const sessions = useSessionsStore();
const { t } = useI18n();

const name = ref("");
const functionCode = ref("3");
const slaveId = ref("1");
const startAddress = ref("0");
const quantity = ref("16");
const dataType = ref<DataType>("u16");
const byteOrder = ref<ByteOrder>("abcd");
const intervalMs = ref("1000");
const enabled = ref(true);
const saving = ref(false);
const errorMessage = ref("");

interface AliasRow {
  address: string;
  name: string;
}

const aliasRows = ref<AliasRow[]>([]);

const functionOptions = computed(() => [
  { label: t("definitionEditor.functions.fc1"), value: "1" },
  { label: t("definitionEditor.functions.fc2"), value: "2" },
  { label: t("definitionEditor.functions.fc3"), value: "3" },
  { label: t("definitionEditor.functions.fc4"), value: "4" },
]);

const dataTypeOptions: Array<{ label: string; value: DataType }> = [
  { label: "BOOL", value: "bool" },
  { label: "INT8", value: "i8" },
  { label: "UINT8", value: "u8" },
  { label: "INT16", value: "i16" },
  { label: "UINT16", value: "u16" },
  { label: "INT32", value: "i32" },
  { label: "UINT32", value: "u32" },
  { label: "INT64", value: "i64" },
  { label: "UINT64", value: "u64" },
  { label: "FLOAT32", value: "f32" },
  { label: "FLOAT64", value: "f64" },
  { label: "HEX", value: "hex" },
  { label: "BIN", value: "bin" },
  { label: "ASCII", value: "ascii" },
];

const byteOrderOptions = computed<Array<{ label: string; value: ByteOrder }>>(() => [
  { label: t("inspector.order.abcd"), value: "abcd" },
  { label: t("inspector.order.dcba"), value: "dcba" },
  { label: t("inspector.order.badc"), value: "badc" },
  { label: t("inspector.order.cdab"), value: "cdab" },
]);

const isBitFunction = computed(() => functionCode.value === "1" || functionCode.value === "2");

const quantityLimit = computed(() => (isBitFunction.value ? 2000 : 125));

const dataTypeHint = computed(() => {
  if (isBitFunction.value) return t("definitionEditor.hints.bit");
  const span = dataType.value === "f32" || dataType.value === "i32" ? 2 : 1;
  return span > 1 ? t("definitionEditor.hints.multi") : t("definitionEditor.hints.single");
});

watch(
  () => [open.value, props.definition] as const,
  ([isOpen]) => {
    if (!isOpen) return;
    errorMessage.value = "";
    const definition = props.definition;
    if (definition) {
      name.value = definition.name;
      functionCode.value = String(definition.functionCode);
      slaveId.value = String(definition.slaveId);
      startAddress.value = String(definition.startAddress);
      quantity.value = String(definition.quantity);
      dataType.value = definition.dataType;
      byteOrder.value = definition.byteOrder;
      intervalMs.value = String(definition.intervalMs);
      enabled.value = definition.enabled;
      aliasRows.value = Object.entries(definition.aliases)
        .map(([address, alias]) => ({ address, name: alias }))
        .sort((left, right) => Number(left.address) - Number(right.address));
    } else {
      name.value = t("definitionEditor.defaultName");
      functionCode.value = "3";
      slaveId.value = "1";
      startAddress.value = "0";
      quantity.value = "16";
      dataType.value = "u16";
      byteOrder.value = "abcd";
      intervalMs.value = "1000";
      enabled.value = true;
      aliasRows.value = [];
    }
  },
  { immediate: true },
);

function addAliasRow(): void {
  const start = Math.max(0, Number(startAddress.value) || 0);
  const nextAddress = start + aliasRows.value.length;
  aliasRows.value = [...aliasRows.value, { address: String(nextAddress), name: "" }];
}

function removeAliasRow(index: number): void {
  aliasRows.value = aliasRows.value.filter((_, rowIndex) => rowIndex !== index);
}

watch(functionCode, (value) => {
  if (value === "1" || value === "2") {
    dataType.value = "bool";
  } else if (dataType.value === "bool") {
    dataType.value = "u16";
  }
});

function fail(message: string): void {
  errorMessage.value = message;
  toast.error(message);
}

async function submit(): Promise<void> {
  if (saving.value) return;
  errorMessage.value = "";
  const definitionName = name.value.trim();
  if (!definitionName) {
    fail(t("definitionEditor.errors.name"));
    return;
  }
  const slave = Number(slaveId.value);
  if (!Number.isInteger(slave) || slave < 1 || slave > 247) {
    fail(t("definitionEditor.errors.slaveId"));
    return;
  }
  const start = Number(startAddress.value);
  if (!Number.isInteger(start) || start < 0 || start > 65535) {
    fail(t("definitionEditor.errors.start"));
    return;
  }
  const count = Number(quantity.value);
  if (!Number.isInteger(count) || count < 1 || count > quantityLimit.value) {
    fail(t("definitionEditor.errors.quantity", { limit: quantityLimit.value }));
    return;
  }
  if (start + count > 65536) {
    fail(t("definitionEditor.errors.range"));
    return;
  }
  const interval = Number(intervalMs.value);
  if (!Number.isInteger(interval) || interval < 10 || interval > 600_000) {
    fail(t("definitionEditor.errors.interval"));
    return;
  }
  const aliases: Record<number, string> = {};
  for (const row of aliasRows.value) {
    const address = Number(row.address);
    const alias = row.name.trim();
    if (!Number.isInteger(address) || address < start || address >= start + count) continue;
    if (alias) aliases[address] = alias;
  }
  const end = start + count;
  const points = (props.definition?.points ?? [])
    .filter(
      (point) =>
        point.address >= start &&
        point.address < end &&
        point.address + dataTypeSpan(point.dataType) <= end,
    )
    .map((point) => ({
      ...point,
      meaning: aliases[point.address] ?? "",
      alerts: point.alerts.map((rule) => ({ ...rule })),
      simulator: point.simulator ? { ...point.simulator } : undefined,
    }));
  const input: DefinitionInput = {
    name: definitionName,
    functionCode: Number(functionCode.value) as FunctionCode,
    slaveId: slave,
    startAddress: start,
    quantity: count,
    dataType: isBitFunction.value ? "bool" : dataType.value,
    byteOrder: byteOrder.value,
    intervalMs: interval,
    enabled: enabled.value,
    aliases,
    points,
  };
  saving.value = true;
  try {
    if (props.definition) {
      await sessions.updateDefinition(props.definition.id, input);
      toast.success(t("definitionEditor.savedUpdate"));
    } else {
      await sessions.createDefinition(input);
      toast.success(t("definitionEditor.savedCreate"));
    }
    open.value = false;
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    errorMessage.value = message;
    toast.error(t("definitionEditor.saveFailed"), { description: message });
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[600px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <SlidersHorizontal class="size-3.5" />
          </span>
          {{ definition ? t("definitionEditor.titleEdit") : t("definitionEditor.titleNew") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ t("definitionEditor.hint") }}
        </DialogDescription>
      </DialogHeader>

      <div
        v-if="errorMessage"
        class="flex shrink-0 items-start gap-2 border-b border-error/25 bg-error/10 px-5 py-2.5 text-[11px] leading-relaxed text-error"
      >
        <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
        <span class="break-words">{{ errorMessage }}</span>
      </div>

      <div class="scrollbar-thin max-h-[62vh] space-y-4 overflow-y-auto px-5 py-4">
        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("definitionEditor.name") }}</Label>
          <Input v-model="name" class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]" />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.functionCode")
            }}</Label>
            <FormSelect v-model="functionCode" :options="functionOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.slaveId")
            }}</Label>
            <Input
              v-model="slaveId"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.startAddress")
            }}</Label>
            <Input
              v-model="startAddress"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("definitionEditor.quantity") }}
              <span class="text-muted-foreground">{{
                t("definitionEditor.quantityMax", { limit: quantityLimit })
              }}</span>
            </Label>
            <Input
              v-model="quantity"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.dataType")
            }}</Label>
            <FormSelect v-model="dataType" :options="dataTypeOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.order")
            }}</Label>
            <FormSelect v-model="byteOrder" :options="byteOrderOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.interval")
            }}</Label>
            <Input
              v-model="intervalMs"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("definitionEditor.enabled")
            }}</Label>
            <div
              class="flex h-9 items-center justify-between rounded-lg border border-hairline-strong bg-well px-3"
            >
              <span class="text-[11px] text-muted-foreground">
                {{ enabled ? t("definitionEditor.enabledOn") : t("definitionEditor.enabledOff") }}
              </span>
              <Switch v-model="enabled" />
            </div>
          </div>
        </div>

        <div class="space-y-2 rounded-xl border border-hairline bg-overlay-1 p-3">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="text-[12px] font-medium text-foreground">
                {{ t("definitionEditor.aliasesTitle") }}
              </p>
              <p class="mt-0.5 text-[11px] text-muted-foreground">
                {{ t("definitionEditor.aliasesHint") }}
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              class="h-7 shrink-0 gap-1 border-hairline-strong bg-overlay-2 text-[11.5px]"
              @click="addAliasRow"
            >
              <Plus class="size-3" />
              {{ t("definitionEditor.addAlias") }}
            </Button>
          </div>
          <div v-if="aliasRows.length" class="space-y-1.5">
            <div v-for="(row, index) in aliasRows" :key="index" class="flex items-center gap-2">
              <Input
                v-model="row.address"
                inputmode="numeric"
                :placeholder="t('definitionEditor.addressPlaceholder')"
                class="numeric h-8 w-[86px] shrink-0 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <Input
                v-model="row.name"
                :placeholder="t('definitionEditor.aliasPlaceholder')"
                class="h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <button
                type="button"
                class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                :aria-label="t('definitionEditor.deleteAlias')"
                @click="removeAliasRow(index)"
              >
                <X class="size-3.5" />
              </button>
            </div>
          </div>
          <p
            v-else
            class="rounded-lg border border-dashed border-hairline px-3 py-2 text-center text-[11px] text-muted-foreground"
          >
            {{ t("definitionEditor.aliasesEmpty") }}
          </p>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ t("definitionEditor.footerHint", { hint: dataTypeHint }) }}
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
        <Button size="sm" class="h-8 min-w-[84px] text-[12px]" :disabled="saving" @click="submit">
          {{ saving ? t("definitionEditor.saving") : t("definitionEditor.save") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
