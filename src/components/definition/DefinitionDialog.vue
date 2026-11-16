<script setup lang="ts">
import { SlidersHorizontal, Trash2 } from "@lucide/vue";
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
import { Switch } from "@/components/ui/switch";
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
const confirmDelete = ref(false);

const functionOptions = [
  { label: "FC01 · 线圈", value: "1" },
  { label: "FC02 · 离散输入", value: "2" },
  { label: "FC03 · 保持寄存器", value: "3" },
  { label: "FC04 · 输入寄存器", value: "4" },
];

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

const byteOrderOptions: Array<{ label: string; value: ByteOrder }> = [
  { label: "ABCD · 大端", value: "abcd" },
  { label: "DCBA · 小端", value: "dcba" },
  { label: "BADC · 字节交换", value: "badc" },
  { label: "CDAB · 字交换", value: "cdab" },
];

const isBitFunction = computed(() => functionCode.value === "1" || functionCode.value === "2");

const quantityLimit = computed(() => (isBitFunction.value ? 2000 : 125));

const dataTypeHint = computed(() => {
  if (isBitFunction.value) return "位读取固定使用 BOOL";
  const span = dataType.value === "f32" || dataType.value === "i32" ? 2 : 1;
  return span > 1 ? "多寄存器类型将跨行合并显示" : "每个寄存器解析为一个值";
});

watch(
  () => [open.value, props.definition] as const,
  ([isOpen]) => {
    if (!isOpen) return;
    confirmDelete.value = false;
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
    } else {
      name.value = "保持寄存器 · 新定义";
      functionCode.value = "3";
      slaveId.value = "1";
      startAddress.value = "0";
      quantity.value = "16";
      dataType.value = "u16";
      byteOrder.value = "abcd";
      intervalMs.value = "1000";
      enabled.value = true;
    }
  },
  { immediate: true },
);

watch(functionCode, (value) => {
  if (value === "1" || value === "2") {
    dataType.value = "bool";
  } else if (dataType.value === "bool") {
    dataType.value = "u16";
  }
});

async function submit(): Promise<void> {
  if (saving.value) return;
  const input: DefinitionInput = {
    name: name.value.trim() || "未命名定义",
    functionCode: Number(functionCode.value) as FunctionCode,
    slaveId: Number(slaveId.value) || 1,
    startAddress: Math.max(0, Number(startAddress.value) || 0),
    quantity: Math.max(1, Number(quantity.value) || 1),
    dataType: isBitFunction.value ? "bool" : dataType.value,
    byteOrder: byteOrder.value,
    intervalMs: Math.max(10, Number(intervalMs.value) || 1000),
    enabled: enabled.value,
    aliases: props.definition?.aliases ?? {},
  };
  saving.value = true;
  try {
    if (props.definition) {
      await sessions.updateDefinition(props.definition.id, input);
      toast.success("定义已更新");
    } else {
      await sessions.createDefinition(input);
      toast.success("定义已创建");
    }
    open.value = false;
  } catch (error: unknown) {
    toast.error("保存定义失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    saving.value = false;
  }
}

async function remove(): Promise<void> {
  if (!props.definition) return;
  if (!confirmDelete.value) {
    confirmDelete.value = true;
    return;
  }
  try {
    await sessions.deleteDefinition(props.definition.id);
    toast.success("定义已删除");
    open.value = false;
  } catch (error: unknown) {
    toast.error("删除定义失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-white/[0.08] bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/60 backdrop-blur-2xl sm:max-w-[580px]"
    >
      <DialogHeader class="border-b border-white/[0.06] px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <SlidersHorizontal class="size-3.5" />
          </span>
          {{ definition ? "编辑读取定义" : "新建读取定义" }}
        </DialogTitle>
        <DialogDescription class="text-[10.5px] text-muted-foreground/65">
          功能码、地址、数据类型与端序由 Rust 核心执行；保存后立即按新规则轮询。
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4 px-5 py-4">
        <div class="space-y-1.5">
          <Label class="text-[10px] text-muted-foreground/70">定义名称</Label>
          <Input
            v-model="name"
            class="h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11.5px]"
          />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">功能码</Label>
            <FormSelect v-model="functionCode" :options="functionOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">从站 ID</Label>
            <Input
              v-model="slaveId"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11.5px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">起始地址</Label>
            <Input
              v-model="startAddress"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11.5px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">
              数量 <span class="text-muted-foreground/40">（≤ {{ quantityLimit }}）</span>
            </Label>
            <Input
              v-model="quantity"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11.5px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">数据类型</Label>
            <FormSelect v-model="dataType" :options="dataTypeOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">字节 / 字序</Label>
            <FormSelect v-model="byteOrder" :options="byteOrderOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">轮询周期 (ms)</Label>
            <Input
              v-model="intervalMs"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-white/[0.08] bg-black/20 text-[11.5px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[10px] text-muted-foreground/70">启用轮询</Label>
            <div
              class="flex h-9 items-center justify-between rounded-lg border border-white/[0.08] bg-black/20 px-3"
            >
              <span class="text-[10px] text-muted-foreground/60">
                {{ enabled ? "已启用" : "已停用" }}
              </span>
              <Switch v-model="enabled" />
            </div>
          </div>
        </div>

        <p
          class="rounded-lg border border-white/[0.05] bg-white/[0.02] px-3 py-2 text-[9.5px] leading-relaxed text-muted-foreground/55"
        >
          {{ dataTypeHint }} · 数据将以原始字 + 格式化值 + 数值同时返回。
        </p>
      </div>

      <DialogFooter class="border-t border-white/[0.06] bg-black/10 px-5 py-3">
        <Button
          v-if="definition"
          variant="ghost"
          size="sm"
          class="mr-auto h-8 gap-1.5 text-[10.5px]"
          :class="confirmDelete ? 'text-error' : 'text-muted-foreground/60'"
          @click="remove"
        >
          <Trash2 class="size-3" />
          {{ confirmDelete ? "再次点击确认删除" : "删除定义" }}
        </Button>
        <DialogClose as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 border-white/[0.08] bg-transparent text-[11px]"
          >
            取消
          </Button>
        </DialogClose>
        <Button size="sm" class="h-8 min-w-[84px] text-[11px]" :disabled="saving" @click="submit">
          {{ saving ? "保存中…" : "保存定义" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
