<script setup lang="ts">
import {
  FlaskConical,
  LayoutDashboard,
  Moon,
  ScanLine,
  Settings2,
  Sparkles,
  Sun,
} from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useScansStore } from "@/stores/scans";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const emit = defineEmits<{
  (event: "open-settings"): void;
  (event: "open-scan"): void;
}>();

const sessions = useSessionsStore();
const scans = useScansStore();
const settings = useSettingsStore();
const { t } = useI18n();

const simulatorActive = computed(() => sessions.activeSession?.role === "slave");

function openSimulator(): void {
  const simulator = sessions.sessions.find((session) => session.role === "slave");
  if (simulator) {
    sessions.setActiveSession(simulator.id);
  } else {
    toast.info("还没有模拟器会话，请通过“新建会话”创建从站。");
  }
}
</script>

<template>
  <TooltipProvider :delay-duration="180">
    <aside
      class="relative flex h-full w-[60px] shrink-0 flex-col items-center border-r border-hairline bg-well py-3"
    >
      <div class="relative mb-5 flex size-11 items-center justify-center">
        <div class="absolute inset-0 rounded-2xl bg-signal/15 blur-xl" aria-hidden="true" />
        <img
          src="/logo.svg"
          alt="Modbus Tool"
          class="relative size-9 rounded-xl shadow-lg shadow-black/25"
        />
      </div>

      <nav class="flex flex-col items-center gap-1.5">
        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="relative flex size-10 items-center justify-center rounded-xl text-foreground transition-colors hover:bg-overlay-3"
              aria-label="工作台"
            >
              <span class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-signal" />
              <LayoutDashboard class="size-[18px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.workspace") }}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="relative flex size-10 items-center justify-center rounded-xl transition-colors hover:bg-overlay-3"
              :class="
                scans.running
                  ? 'bg-signal/10 text-signal'
                  : 'text-muted-foreground hover:text-foreground'
              "
              aria-label="设备扫描"
              @click="emit('open-scan')"
            >
              <ScanLine class="size-[18px]" />
              <span
                v-if="scans.running"
                class="status-dot status-dot-live absolute right-2 top-2 scale-75 text-signal"
              />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.scan") }}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="flex size-10 items-center justify-center rounded-xl transition-colors hover:bg-overlay-3"
              :class="
                simulatorActive
                  ? 'bg-sim/10 text-sim'
                  : 'text-muted-foreground hover:text-foreground'
              "
              aria-label="从站模拟器"
              @click="openSimulator"
            >
              <FlaskConical class="size-[18px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.simulator") }}</TooltipContent>
        </Tooltip>
      </nav>

      <div class="mt-auto flex flex-col items-center gap-1.5">
        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="flex size-10 items-center justify-center rounded-xl text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
              :aria-label="settings.resolvedTheme === 'dark' ? '切换到浅色主题' : '切换到深色主题'"
              @click="settings.toggleTheme()"
            >
              <Sun v-if="settings.resolvedTheme === 'dark'" class="size-[17px]" />
              <Moon v-else class="size-[17px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">
            {{ settings.resolvedTheme === "dark" ? "切换到浅色主题" : "切换到深色主题" }}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="flex size-10 items-center justify-center rounded-xl transition-colors hover:bg-overlay-3"
              :class="
                settings.reduceEffects ? 'text-warn' : 'text-muted-foreground hover:text-foreground'
              "
              aria-label="减少特效"
              @click="settings.reduceEffects = !settings.reduceEffects"
            >
              <Sparkles class="size-[17px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("dialog.reduceEffects") }}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="flex size-10 items-center justify-center rounded-xl text-muted-foreground transition-colors hover:bg-overlay-3 hover:text-foreground"
              aria-label="设置"
              @click="emit('open-settings')"
            >
              <Settings2 class="size-[18px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.settings") }}</TooltipContent>
        </Tooltip>

        <div
          class="mt-2 flex size-8 items-center justify-center rounded-full border border-hairline bg-overlay-2"
        >
          <span class="status-dot status-dot-live text-signal" />
        </div>
      </div>
    </aside>
  </TooltipProvider>
</template>
