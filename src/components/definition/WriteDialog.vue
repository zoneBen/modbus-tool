<script setup lang="ts">
import { CircleAlert, CircleCheck, PencilLine, Send } from "@lucide/vue";
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
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { CustomWriteInput, WriteFunctionCode } from "@/types/models";
import type { WriteTarget } from "@/types/ui";

type WriteMode = "auto" | "5" | "6" | "15" | "16";
type Feedback = { tone: "success" | "error"; message: string };

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  target: WriteTarget | null;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const slaveId = ref("1");
const mode = ref<WriteMode>("6");
const address = ref("0");
const value = ref("");
const busy = ref(false);
const feedback = ref<Feedback | null>(null);

const definition = computed(() => sessions.activeDefinition);
const isFloat = computed(
  () => props.target?.dataType === "f32" || props.target?.dataType === "f64",
);
const isBitRead = computed(
  () => definition.value?.functionCode === 1 || definition.value?.functionCode === 2,
);

const functionOptions = computed<Array<{ label: string; value: WriteMode }>>(() => [
  { label: t("writeDialog.functions.auto"), value: "auto" },
  { label: t("writeDialog.functions.fc05"), value: "5" },
  { label: t("writeDialog.functions.fc06"), value: "6" },
  { label: t("writeDialog.functions.fc0f"), value: "15" },
  { label: t("writeDialog.functions.fc10"), value: "16" },
]);

const values = computed(() => value.value.split(/[\s,;]+/).filter(Boolean));
const quantity = computed(() => values.value.length);

const isMultiple = computed(() => mode.value === "15" || mode.value === "16");
const slaveEditable = computed(() => mode.value !== "auto");

const functionLabel = computed(() => {
  if (mode.value === "auto") return t("writeDialog.functions.autoLabel");
  return `FC${mode.value.padStart(2, "0")}`;
});

const valueHint = computed(() => {
  switch (mode.value) {
    case "auto":
      return isFloat.value ? t("writeDialog.hints.autoFloat") : t("writeDialog.hints.auto");
    case "5":
      return t("writeDialog.hints.coil");
    case "6":
      return t("writeDialog.hints.register");
    case "15":
      return t("writeDialog.hints.coils");
    case "16":
      return t("writeDialog.hints.registers");
    default:
      return "";
  }
});

watch(
  () => [open.value, props.target] as const,
  () => {
    if (!open.value || !props.target) return;
    const target = props.target;
    slaveId.value = String(definition.value?.slaveId ?? 1);
    address.value = String(target.address);
    value.value = target.current === "—" ? "" : target.current.replace(/,/g, "");
    feedback.value = null;
    busy.value = false;
    mode.value = isFloat.value ? "auto" : isBitRead.value ? "5" : "6";
  },
  { immediate: true },
);

function fail(message: string): void {
  feedback.value = { tone: "error", message: t("writeDialog.failedMessage", { message }) };
  toast.error(t("writeDialog.failed"), { description: message });
}

async function submit(): Promise<void> {
  if (!props.target || busy.value) return;
  const raw = value.value.trim();
  if (!raw) {
    fail(t("writeDialog.errors.valueRequired"));
    return;
  }
  const parsedValues = values.value;
  if (!parsedValues.length) {
    fail(t("writeDialog.errors.valueRequired"));
    return;
  }
  const parsedAddress = Number(address.value);
  if (!Number.isInteger(parsedAddress) || parsedAddress < 0 || parsedAddress > 0xffff) {
    fail(t("writeDialog.errors.address"));
    return;
  }
  if (mode.value === "auto" && parsedValues.length !== 1) {
    fail(t("writeDialog.errors.autoSingle"));
    return;
  }
  if ((mode.value === "5" || mode.value === "6") && parsedValues.length !== 1) {
    fail(t("writeDialog.errors.singleValue", { function: functionLabel.value }));
    return;
  }
  if (mode.value === "15" && parsedValues.length > 1968) {
    fail(t("writeDialog.errors.coilLimit"));
    return;
  }
  if (mode.value === "16" && parsedValues.length > 123) {
    fail(t("writeDialog.errors.registerLimit"));
    return;
  }
  const parsedSlave = Number(slaveId.value);
  if (!Number.isInteger(parsedSlave) || parsedSlave < 1 || parsedSlave > 247) {
    fail(t("writeDialog.errors.slaveId"));
    return;
  }

  busy.value = true;
  feedback.value = null;
  try {
    if (mode.value === "auto") {
      await sessions.writeRegister(parsedAddress, parsedValues[0] ?? "");
    } else {
      const input: CustomWriteInput = {
        slaveId: parsedSlave,
        functionCode: Number(mode.value) as WriteFunctionCode,
        address: parsedAddress,
        values: parsedValues,
      };
      await sessions.writeCustom(input);
    }
    const summary = t("writeDialog.summary", {
      function: functionLabel.value,
      slave: slaveEditable.value ? parsedSlave : (definition.value?.slaveId ?? parsedSlave),
      address: formatAddress(parsedAddress, settings.addressBase),
      values: parsedValues.join(", "),
    });
    feedback.value = { tone: "success", message: t("writeDialog.successMessage", { summary }) };
    toast.success(t("writeDialog.success"), { description: summary });
  } catch (error: unknown) {
    fail(error instanceof Error ? error.message : String(error));
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[520px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <PencilLine class="size-3.5" />
          </span>
          {{ t("writeDialog.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ t("writeDialog.hint") }}
        </DialogDescription>
      </DialogHeader>

      <div v-if="target" class="scrollbar-thin max-h-[68vh] space-y-4 overflow-y-auto px-5 py-4">
        <div class="grid grid-cols-2 gap-2 rounded-xl border border-hairline bg-overlay-1 p-3">
          <div class="min-w-0">
            <p class="text-[11px] text-muted-foreground">{{ t("writeDialog.targetDefinition") }}</p>
            <p class="mt-0.5 truncate text-[12px] text-foreground">
              {{ definition?.name ?? "—" }}
            </p>
          </div>
          <div class="min-w-0 text-right">
            <p class="text-[11px] text-muted-foreground">{{ t("writeDialog.targetAddress") }}</p>
            <p class="numeric mt-0.5 text-[12px] text-foreground">
              {{ formatAddress(target.address, settings.addressBase) }}
            </p>
          </div>
          <div class="min-w-0">
            <p class="text-[11px] text-muted-foreground">{{ t("writeDialog.currentValue") }}</p>
            <p class="numeric mt-0.5 truncate text-[12px] text-foreground">
              {{ target.current }}
            </p>
          </div>
          <div class="min-w-0 text-right">
            <p class="text-[11px] text-muted-foreground">{{ t("writeDialog.nameType") }}</p>
            <p class="mt-0.5 truncate text-[12px] text-foreground">
              {{ target.alias }} · {{ target.dataType.toUpperCase() }}
            </p>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{ t("writeDialog.slaveId") }}</Label>
            <Input
              v-model="slaveId"
              inputmode="numeric"
              :disabled="!slaveEditable"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px] disabled:opacity-60"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("writeDialog.functionCode")
            }}</Label>
            <FormSelect v-model="mode" :options="functionOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("writeDialog.startAddress")
            }}</Label>
            <Input
              v-model="address"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              {{ t("writeDialog.quantity") }}
              <span class="text-muted-foreground">{{
                t("writeDialog.quantityValue", { count: isMultiple ? quantity : 1 })
              }}</span>
            </Label>
            <div
              class="numeric flex h-9 items-center rounded-lg border border-hairline bg-well-soft px-3 text-[12px] text-muted-foreground"
            >
              {{
                isMultiple
                  ? t("writeDialog.quantityItems", { count: quantity })
                  : t("writeDialog.singleWrite")
              }}
            </div>
          </div>
        </div>

        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">{{ t("writeDialog.value") }}</Label>
          <Input
            v-model="value"
            type="text"
            class="numeric h-10 rounded-lg border-hairline-strong bg-well text-[14px] tracking-tight"
            :placeholder="isMultiple ? '1, 0, 1, 1' : t('writeDialog.valuePlaceholder')"
            @keydown.enter="submit"
          />
          <p class="text-[11px] leading-relaxed text-muted-foreground">{{ valueHint }}</p>
          <p v-if="!slaveEditable" class="text-[11px] leading-relaxed text-muted-foreground">
            {{
              t("writeDialog.autoHint", {
                name: definition?.name ?? "",
              })
            }}
          </p>
        </div>

        <div
          v-if="feedback"
          class="flex items-start gap-2 rounded-lg border px-3 py-2 text-[11px] leading-relaxed"
          :class="
            feedback.tone === 'success'
              ? 'border-signal/30 bg-signal/10 text-signal'
              : 'border-error/30 bg-error/10 text-error'
          "
        >
          <CircleCheck v-if="feedback.tone === 'success'" class="mt-0.5 size-3.5 shrink-0" />
          <CircleAlert v-else class="mt-0.5 size-3.5 shrink-0" />
          <span class="break-words">{{ feedback.message }}</span>
        </div>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            {{ t("writeDialog.close") }}
          </Button>
        </DialogClose>
        <Button
          size="sm"
          class="h-8 min-w-[92px] gap-1.5 text-[12px]"
          :disabled="busy || !target"
          @click="submit"
        >
          <Send class="size-3.5" />
          {{ busy ? t("writeDialog.executing") : t("writeDialog.execute") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
