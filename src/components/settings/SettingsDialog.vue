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

const themeOptions = [
  { value: "dark" as const, label: "深色" },
  { value: "light" as const, label: "浅色" },
  { value: "system" as const, label: "系统" },
];

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
      class="gap-0 overflow-hidden border-hairline-strong bg-[color:var(--glass-strong)] p-0 shadow-2xl shadow-black/25 backdrop-blur-2xl sm:max-w-[460px]"
    >
      <DialogHeader class="border-b border-hairline px-5 py-4">
        <DialogTitle class="flex items-center gap-2 text-[13.5px]">
          <span
            class="flex size-7 items-center justify-center rounded-lg border border-hairline-strong bg-overlay-2"
          >
            <Settings2 class="size-3.5 text-muted-foreground" />
          </span>
          显示与交互
        </DialogTitle>
        <DialogDescription class="text-[11.5px] text-muted-foreground">
          这些偏好保存在本地 WebView / 浏览器存储中。
        </DialogDescription>
      </DialogHeader>

      <div class="px-5 py-2">
        <div class="flex items-center justify-between gap-4 py-3">
          <div>
            <p class="text-[12px] font-medium text-foreground">界面主题</p>
            <p class="mt-0.5 text-[11px] text-muted-foreground">
              深色仪器风或浅色高对比，可跟随系统设置。
            </p>
          </div>
          <div class="flex rounded-lg border border-hairline-strong bg-well p-0.5">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="rounded-md px-2 py-1 text-[11px] transition-colors"
              :class="
                settings.theme === option.value
                  ? 'bg-overlay-3 text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              :aria-pressed="settings.theme === option.value"
              @click="settings.setTheme(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <div
          v-for="item in toggles"
          :key="item.key"
          class="flex items-center justify-between gap-4 border-t border-hairline py-3"
        >
          <div>
            <p class="text-[12px] font-medium text-foreground">{{ item.title }}</p>
            <p class="mt-0.5 text-[11px] leading-relaxed text-muted-foreground">
              {{ item.hint }}
            </p>
          </div>
          <Switch
            :model-value="settings[item.key]"
            @update:model-value="(checked) => (settings[item.key] = Boolean(checked))"
          />
        </div>

        <div class="flex items-center justify-between gap-4 border-t border-hairline py-3">
          <div>
            <p class="text-[12px] font-medium text-foreground">地址基址</p>
            <p class="mt-0.5 text-[11px] text-muted-foreground">
              Modbus Poll 风格：0 基址或 1 基址显示。
            </p>
          </div>
          <div class="flex rounded-lg border border-hairline-strong bg-well p-0.5">
            <button
              v-for="base in addressBases"
              :key="base"
              type="button"
              class="numeric w-7 rounded-md py-1 text-[11px] transition-colors"
              :class="
                settings.addressBase === base
                  ? 'bg-overlay-3 text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="settings.addressBase = base"
            >
              {{ base }}
            </button>
          </div>
        </div>
      </div>

      <Separator class="bg-overlay-2" />

      <div class="flex items-center justify-between px-5 py-3 text-[11px] text-muted-foreground">
        <div class="flex items-center gap-2">
          <span class="font-medium text-foreground">Modbus Tool</span>
          <span class="numeric">v0.1.3 · Preview</span>
        </div>
        <span
          v-if="sessions.demo"
          class="rounded-full border border-sim/25 bg-sim/10 px-1.5 py-0.5 text-[11px] text-sim"
        >
          DEMO RUNTIME
        </span>
      </div>

      <DialogFooter class="border-t border-hairline bg-well-soft px-5 py-3">
        <DialogClose as-child>
          <Button size="sm" class="h-8 text-[12px]">完成</Button>
        </DialogClose>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
