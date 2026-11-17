<script setup lang="ts">
import { RefreshCw, Usb } from "@lucide/vue";
import { computed } from "vue";
import FormSelect from "@/components/common/FormSelect.vue";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { SerialPortInfo } from "@/types/models";

const props = defineProps<{
  ports: SerialPortInfo[];
  loading: boolean;
  error: string;
}>();

const emit = defineEmits<{
  (event: "refresh"): void;
}>();

const model = defineModel<string>({ required: true });

const showSelect = computed(() => props.ports.length > 0);
const options = computed(() => {
  const list = props.ports.map((port) => ({ label: port.label, value: port.path }));
  if (model.value && !list.some((option) => option.value === model.value)) {
    list.unshift({
      label: `${model.value} · 当前配置（未检测到）`,
      value: model.value,
    });
  }
  return list;
});
</script>

<template>
  <div class="space-y-1.5">
    <div class="flex items-center justify-between gap-2">
      <Label class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        <Usb class="size-3" />
        串口端口
        <span v-if="ports.length" class="numeric text-foreground/45">
          · {{ ports.length }} 个设备
        </span>
      </Label>
      <button
        type="button"
        class="flex items-center gap-1 rounded px-1 py-0.5 text-[10.5px] text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50"
        :disabled="loading"
        aria-label="刷新串口列表"
        @click="emit('refresh')"
      >
        <RefreshCw class="size-3" :class="loading ? 'animate-spin' : ''" />
        刷新
      </button>
    </div>

    <FormSelect v-if="showSelect" v-model="model" :options="options" :disabled="loading" />
    <Input
      v-else
      v-model="model"
      :disabled="loading"
      :placeholder="loading ? '正在检测串口…' : '例如 COM21'"
      class="numeric h-9 rounded-lg border-hairline-strong bg-well text-[12px]"
    />

    <p v-if="error" class="text-[10.5px] leading-relaxed text-error">{{ error }}</p>
    <p v-else-if="!loading && !ports.length" class="text-[10.5px] leading-relaxed text-warn">
      未检测到串口设备，请检查驱动或手动输入端口名。
    </p>
    <p v-else-if="loading" class="text-[10.5px] text-muted-foreground">正在读取本机串口列表…</p>
    <p v-else class="text-[10.5px] text-muted-foreground">
      已读取本机串口，打开对话框时会自动刷新。
    </p>
  </div>
</template>
