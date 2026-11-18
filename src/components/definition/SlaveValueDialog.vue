<script setup lang="ts">
import { Braces, CircleAlert } from "@lucide/vue";
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
import { DATA_TYPE_OPTIONS, byteOrderOptions } from "@/lib/definition";
import { formatAddress } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ByteOrder, DataType, Definition } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
  address: number;
  current: string;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();
const startAddress = ref("0");
const dataType = ref<DataType>("u16");
const byteOrder = ref<ByteOrder>("abcd");
const value = ref("");
const busy = ref(false);
const error = ref("");
const orderOptions = computed(() => byteOrderOptions(t));

const addressLabel = computed(() =>
  formatAddress(Number(startAddress.value) || 0, settings.addressBase),
);

watch(
  () => [open.value, props.address] as const,
  () => {
    if (!open.value) return;
    startAddress.value = String(props.address);
    dataType.value = "u16";
    byteOrder.value = "abcd";
    value.value = props.current === "—" ? "" : props.current.replace(/,/g, "");
    error.value = "";
    busy.value = false;
  },
  { immediate: true },
);

async function save(): Promise<void> {
  if (busy.value || !props.definition) return;
  const parsedAddress = Number(startAddress.value);
  if (!Number.isInteger(parsedAddress) || parsedAddress < 0 || parsedAddress > 0xffff) {
    error.value = t("slaveValueDialog.invalidAddress");
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    const count = await sessions.writeSlaveValue({
      address: parsedAddress,
      dataType: dataType.value,
      byteOrder: byteOrder.value,
      value: value.value,
    });
    toast.success(t("slaveValueDialog.saved"), {
      description: t("slaveValueDialog.savedDescription", {
        address: addressLabel.value,
        count,
      }),
    });
    open.value = false;
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    error.value = message;
    toast.error(t("slaveValueDialog.failed"), { description: message });
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[460px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-sim/25 bg-sim/10 text-sim"
          >
            <Braces class="size-3.5" />
          </span>
          {{ t("slaveValueDialog.title") }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{
            t("slaveValueDialog.description", {
              name: definition?.name ?? t("slaveValueDialog.fallbackName"),
            })
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("slaveValueDialog.startAddress")
            }}</Label>
            <Input
              v-model="startAddress"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("slaveValueDialog.dataType")
            }}</Label>
            <FormSelect v-model="dataType" :options="DATA_TYPE_OPTIONS" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("slaveValueDialog.order")
            }}</Label>
            <FormSelect v-model="byteOrder" :options="orderOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">{{
              t("slaveValueDialog.value")
            }}</Label>
            <Input
              v-model="value"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              :placeholder="t('slaveValueDialog.valuePlaceholder')"
              @keydown.enter="save"
            />
          </div>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ t("slaveValueDialog.hint") }}
        </p>

        <p
          v-if="error"
          class="flex items-start gap-2 rounded-lg border border-error/30 bg-error/10 px-3 py-2 text-[11px] leading-relaxed text-error"
        >
          <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
          <span class="break-words">{{ error }}</span>
        </p>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-hairline-strong bg-transparent text-[12px]"
          >
            {{ t("slaveValueDialog.cancel") }}
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[96px] text-[12px]" :disabled="busy" @click="save">
          {{ busy ? t("slaveValueDialog.writing") : t("slaveValueDialog.write") }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
