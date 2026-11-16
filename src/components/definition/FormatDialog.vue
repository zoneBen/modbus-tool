<script setup lang="ts">
import { Braces } from "@lucide/vue";
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
import { Label } from "@/components/ui/label";
import { BYTE_ORDER_OPTIONS, DATA_TYPE_OPTIONS, toDefinitionInput } from "@/lib/definition";
import { formatFunctionCode, formatRange } from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { ByteOrder, DataType, Definition } from "@/types/models";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  definition: Definition | null;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const dataType = ref<DataType>("u16");
const byteOrder = ref<ByteOrder>("abcd");
const saving = ref(false);

const isBit = computed(
  () => props.definition?.functionCode === 1 || props.definition?.functionCode === 2,
);

const rangeLabel = computed(() =>
  props.definition
    ? formatRange(props.definition.startAddress, props.definition.quantity, settings.addressBase)
    : "—",
);

watch(
  () => [open.value, props.definition] as const,
  () => {
    if (!open.value || !props.definition) return;
    dataType.value = props.definition.dataType;
    byteOrder.value = props.definition.byteOrder;
  },
  { immediate: true },
);

async function save(): Promise<void> {
  const definition = props.definition;
  if (!definition || saving.value) return;
  if (isBit.value) {
    open.value = false;
    return;
  }
  saving.value = true;
  try {
    await sessions.updateDefinition(definition.id, {
      ...toDefinitionInput(definition),
      dataType: dataType.value,
      byteOrder: byteOrder.value,
    });
    toast.success("解析格式已更新", {
      description: `${dataType.value.toUpperCase()} · ${BYTE_ORDER_OPTIONS.find((item) => item.value === byteOrder.value)?.label ?? byteOrder.value}`,
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error("更新格式失败", {
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
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[440px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <Braces class="size-3.5" />
          </span>
          编辑解析格式
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          {{ definition?.name ?? "未选择定义" }} ·
          {{
            definition
              ? `${formatFunctionCode(definition.functionCode)} · ${rangeLabel}`
              : "选择读取定义后修改"
          }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">数据类型</Label>
            <FormSelect
              v-model="dataType"
              :options="DATA_TYPE_OPTIONS"
              :disabled="isBit || saving"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">字节 / 字序</Label>
            <FormSelect
              v-model="byteOrder"
              :options="BYTE_ORDER_OPTIONS"
              :disabled="isBit || saving"
            />
          </div>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{
            isBit
              ? "FC01 / FC02 线圈读取固定为 BOOL，无字节序参数。"
              : "修改后立即按新格式轮询；多寄存器类型会跨行合并显示，并影响曲线与写入编码。"
          }}
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
        <Button size="sm" class="h-8 min-w-[84px] text-[12px]" :disabled="saving" @click="save">
          {{ saving ? "保存中…" : "保存格式" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
