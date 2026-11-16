<script setup lang="ts">
import { PencilLine } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { toast } from "vue-sonner";
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
import type { WriteTarget } from "@/types/ui";

const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  target: WriteTarget | null;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const value = ref("");
const busy = ref(false);

const isFloat = computed(
  () => props.target?.dataType === "f32" || props.target?.dataType === "f64",
);

watch(
  () => [open.value, props.target] as const,
  () => {
    if (open.value && props.target) {
      value.value = props.target.current.replace(/,/g, "");
    }
  },
  { immediate: true },
);

async function submit(): Promise<void> {
  if (!props.target || busy.value) return;
  const trimmed = value.value.trim();
  if (!trimmed) {
    toast.error("请输入写入值");
    return;
  }
  busy.value = true;
  try {
    await sessions.writeRegister(props.target.address, trimmed);
    toast.success(`已写入地址 ${formatAddress(props.target.address, settings.addressBase)}`, {
      description: `值：${trimmed} · 已同步到寄存器缓存与报文监视器`,
    });
    open.value = false;
  } catch (error: unknown) {
    toast.error("写入失败", {
      description: error instanceof Error ? error.message : String(error),
    });
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[420px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
          >
            <PencilLine class="size-3.5" />
          </span>
          写入寄存器
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          演示模式下写入会立即更新本地缓存，并生成 FC06 / FC16 报文。
        </DialogDescription>
      </DialogHeader>

      <div v-if="target" class="space-y-4 px-5 py-4">
        <div
          class="flex items-center justify-between rounded-lg border border-hairline bg-overlay-1 px-3 py-2.5"
        >
          <div>
            <p class="text-[11px] text-muted-foreground">目标地址</p>
            <p class="numeric mt-0.5 text-[13px] text-foreground">
              {{ formatAddress(target.address, settings.addressBase) }}
            </p>
          </div>
          <div class="min-w-0 text-right">
            <p class="truncate text-[11px] text-muted-foreground">{{ target.alias }}</p>
            <p class="numeric mt-0.5 text-[13px] text-foreground">{{ target.current }}</p>
          </div>
          <span
            class="numeric ml-3 rounded border border-rx/20 bg-rx/10 px-1.5 py-0.5 text-[11px] text-rx"
          >
            {{ target.dataType.toUpperCase() }}
          </span>
        </div>

        <div class="space-y-1.5">
          <Label class="text-[11px] text-muted-foreground">写入值</Label>
          <Input
            v-model="value"
            :step="isFloat ? 'any' : 1"
            type="text"
            inputmode="decimal"
            class="numeric h-11 rounded-lg border-hairline-strong bg-well text-[16px] tracking-tight"
            @keydown.enter="submit"
          />
          <p class="text-[11px] text-muted-foreground">
            {{ isFloat ? "支持小数，按定义的浮点字节序编码" : "0 – 65535 的整数" }}
          </p>
        </div>
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
        <Button
          size="sm"
          class="h-8 min-w-[76px] text-[12px]"
          :disabled="busy || !target"
          @click="submit"
        >
          {{ busy ? "写入中…" : "写入" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
