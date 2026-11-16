<script setup lang="ts">
import { Settings2 } from "@lucide/vue";
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
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const open = defineModel<boolean>("open", { default: false });
const settings = useSettingsStore();
const sessions = useSessionsStore();
const addressBases = [0, 1] as const;

const toggles = [
  {
    key: "reduceEffects" as const,
    title: "减少特效",
    hint: "关闭毛玻璃模糊，降低低端设备的 GPU 压力。",
  },
  {
    key: "compactRows" as const,
    title: "紧凑行高",
    hint: "寄存器表格使用更小的行间距。",
  },
  {
    key: "monitorFollow" as const,
    title: "报文自动跟随",
    hint: "新帧到达时自动滚动到最新一条。",
  },
  {
    key: "showRawColumn" as const,
    title: "显示原始值列",
    hint: "在寄存器表格中显示原始寄存器 HEX。",
  },
];
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="gap-0 overflow-hidden border-white/[0.08] bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/60 backdrop-blur-2xl sm:max-w-[460px]"
    >
      <DialogHeader class="border-b border-white/[0.06] px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-white/[0.08] bg-white/[0.04]"
          >
            <Settings2 class="size-3.5 text-muted-foreground" />
          </span>
          显示与交互
        </DialogTitle>
        <DialogDescription class="text-[10.5px] text-muted-foreground/65">
          这些偏好保存在本地 WebView / 浏览器存储中。
        </DialogDescription>
      </DialogHeader>

      <div class="px-5 py-2">
        <div
          v-for="(item, index) in toggles"
          :key="item.key"
          class="flex items-center justify-between gap-4 py-3"
          :class="index > 0 ? 'border-t border-white/[0.05]' : ''"
        >
          <div>
            <p class="text-[11.5px] font-medium text-foreground/90">{{ item.title }}</p>
            <p class="mt-0.5 text-[9.5px] leading-relaxed text-muted-foreground/50">
              {{ item.hint }}
            </p>
          </div>
          <Switch
            :model-value="settings[item.key]"
            @update:model-value="(checked) => (settings[item.key] = Boolean(checked))"
          />
        </div>

        <div class="flex items-center justify-between gap-4 border-t border-white/[0.05] py-3">
          <div>
            <p class="text-[11.5px] font-medium text-foreground/90">地址基址</p>
            <p class="mt-0.5 text-[9.5px] text-muted-foreground/50">
              Modbus Poll 风格：0 基址或 1 基址显示。
            </p>
          </div>
          <div class="flex rounded-lg border border-white/[0.08] bg-black/20 p-0.5">
            <button
              v-for="base in addressBases"
              :key="base"
              type="button"
              class="numeric w-7 rounded-md py-1 text-[10px] transition-colors"
              :class="
                settings.addressBase === base
                  ? 'bg-white/[0.09] text-foreground'
                  : 'text-muted-foreground/55 hover:text-foreground'
              "
              @click="settings.addressBase = base"
            >
              {{ base }}
            </button>
          </div>
        </div>
      </div>

      <Separator class="bg-white/[0.05]" />

      <div
        class="flex items-center justify-between px-5 py-3 text-[9.5px] text-muted-foreground/50"
      >
        <div class="flex items-center gap-2">
          <span class="font-medium text-foreground/70">Modbus Tool</span>
          <span class="numeric">v0.1.0 · P1 Preview</span>
        </div>
        <span
          v-if="sessions.demo"
          class="rounded-full border border-sim/25 bg-sim/10 px-1.5 py-0.5 text-[9px] text-sim"
        >
          DEMO RUNTIME
        </span>
      </div>

      <DialogFooter class="border-t border-white/[0.06] bg-black/10 px-5 py-3">
        <DialogClose as-child>
          <Button size="sm" class="h-8 text-[11px]">完成</Button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
