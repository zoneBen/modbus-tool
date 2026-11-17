<script setup lang="ts">
import { CircleAlert, Plus, SlidersHorizontal, X } from "@lucide/vue";
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

const functionOptions = [
  { label: "FC01 线圈", value: "1" },
  { label: "FC02 离散输入", value: "2" },
  { label: "FC03 保持寄存器", value: "3" },
  { label: "FC04 输入寄存器", value: "4" },
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
      name.value = "保持寄存器 · 新定义";
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
    fail("定义名称不能为空");
    return;
  }
  const slave = Number(slaveId.value);
  if (!Number.isInteger(slave) || slave < 1 || slave > 247) {
    fail("从站 ID 需在 1–247 之间");
    return;
  }
  const start = Number(startAddress.value);
  if (!Number.isInteger(start) || start < 0 || start > 65535) {
    fail("起始地址需为 0–65535 的整数");
    return;
  }
  const count = Number(quantity.value);
  if (!Number.isInteger(count) || count < 1 || count > quantityLimit.value) {
    fail(`数量需为 1–${quantityLimit.value} 的整数`);
    return;
  }
  if (start + count > 65536) {
    fail("地址范围超出 16 位地址空间");
    return;
  }
  const interval = Number(intervalMs.value);
  if (!Number.isInteger(interval) || interval < 10 || interval > 600_000) {
    fail("轮询周期需为 10–600000 ms 的整数");
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
      toast.success("定义已更新");
    } else {
      await sessions.createDefinition(input);
      toast.success("定义已创建");
    }
    open.value = false;
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    errorMessage.value = message;
    toast.error("保存定义失败", { description: message });
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
          {{ definition ? "编辑读取定义" : "新建读取定义" }}
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          功能码、地址、数据类型与端序由 Rust 核心执行；保存后立即按新规则轮询。
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
          <Label class="text-[11px] text-muted-foreground">定义名称</Label>
          <Input v-model="name" class="h-9 rounded-lg border-hairline-strong bg-well text-[12px]" />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">功能码</Label>
            <FormSelect v-model="functionCode" :options="functionOptions" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">从站 ID</Label>
            <Input
              v-model="slaveId"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">起始地址</Label>
            <Input
              v-model="startAddress"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">
              数量 <span class="text-muted-foreground">（≤ {{ quantityLimit }}）</span>
            </Label>
            <Input
              v-model="quantity"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">数据类型</Label>
            <FormSelect v-model="dataType" :options="dataTypeOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">字节 / 字序</Label>
            <FormSelect v-model="byteOrder" :options="byteOrderOptions" :disabled="isBitFunction" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">轮询周期 (ms)</Label>
            <Input
              v-model="intervalMs"
              inputmode="numeric"
              class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
            />
          </div>
          <div class="space-y-1.5">
            <Label class="text-[11px] text-muted-foreground">启用轮询</Label>
            <div
              class="flex h-9 items-center justify-between rounded-lg border border-hairline-strong bg-well px-3"
            >
              <span class="text-[11px] text-muted-foreground">
                {{ enabled ? "已启用" : "已停用" }}
              </span>
              <Switch v-model="enabled" />
            </div>
          </div>
        </div>

        <div class="space-y-2 rounded-xl border border-hairline bg-overlay-1 p-3">
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="text-[12px] font-medium text-foreground">寄存器别名 / 含义</p>
              <p class="mt-0.5 text-[11px] text-muted-foreground">
                为地址维护名称，寄存器表格、检查器与曲线会显示别名。
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              class="h-7 shrink-0 gap-1 border-hairline-strong bg-overlay-2 text-[11.5px]"
              @click="addAliasRow"
            >
              <Plus class="size-3" />
              添加
            </Button>
          </div>
          <div v-if="aliasRows.length" class="space-y-1.5">
            <div v-for="(row, index) in aliasRows" :key="index" class="flex items-center gap-2">
              <Input
                v-model="row.address"
                inputmode="numeric"
                placeholder="地址"
                class="numeric h-8 w-[86px] shrink-0 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <Input
                v-model="row.name"
                placeholder="含义，例如 电机温度"
                class="h-8 min-w-0 flex-1 rounded-lg border-hairline-strong bg-well text-[12px]"
              />
              <button
                type="button"
                class="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-error/10 hover:text-error"
                aria-label="删除别名"
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
            暂无别名，表格将显示地址。
          </p>
        </div>

        <p
          class="rounded-lg border border-hairline bg-overlay-1 px-3 py-2 text-[11px] leading-relaxed text-muted-foreground"
        >
          {{ dataTypeHint }} · 数据将以原始字 + 格式化值 + 数值同时返回。
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
        <Button size="sm" class="h-8 min-w-[84px] text-[12px]" :disabled="saving" @click="submit">
          {{ saving ? "保存中…" : "保存定义" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
