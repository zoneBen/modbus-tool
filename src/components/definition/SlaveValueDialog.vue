<script setup lang="ts">
import { Braces, CircleAlert } from "@lucide/vue";
import { computed, ref, watch } from "vue";
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
import { BYTE_ORDER_OPTIONS, DATA_TYPE_OPTIONS } from "@/lib/definition";
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
const startAddress = ref("0");
const dataType = ref<DataType>("u16");
const byteOrder = ref<ByteOrder>("abcd");
const value = ref("");
const busy = ref(false);
const error = ref("");

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
    error.value = "地址需为 0–65535 的整数";
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
    toast.success("模拟值已写入", {
      description: `${addressLabel.value} 起 ${count} 个寄存器`,
    });
    open.value = false;
  } catch (cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    error.value = message;
    toast.error("写入模拟值失败", { description: message });
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
          修改模拟值
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ definition?.name ?? "从站模拟映射" }} · 值会按所选类型与端序逆向编码到保持寄存器。
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">起始地址</Label>
            <Input
              v-model="startAddress"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">数据类型</Label>
            <FormSelect v-model="dataType" :options="DATA_TYPE_OPTIONS" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">字节 / 字序</Label>
            <FormSelect v-model="byteOrder" :options="BYTE_ORDER_OPTIONS" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">写入值</Label>
            <Input
              v-model="value"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
              placeholder="输入数值或文本"
              @keydown.enter="save"
            />
          </div>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          多寄存器类型（FLOAT32、INT32、FLOAT64 等）会从起始地址开始连续写入；
          主站随后读取到的是编码后的原始寄存器。
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
            取消
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[96px] text-[12px]" :disabled="busy" @click="save">
          {{ busy ? "写入中…" : "写入模拟值" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
