<script setup lang="ts">
import { Info, LayoutDashboard, Moon, ScanLine, Settings2, Sun } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useScansStore } from "@/stores/scans";
import { useSettingsStore } from "@/stores/settings";

type RailView = "workspace" | "scan" | "settings" | "about";

withDefaults(
  defineProps<{
    activeView?: RailView;
  }>(),
  { activeView: "workspace" },
);

const emit = defineEmits<{
  (event: "open-settings"): void;
  (event: "open-scan"): void;
  (event: "open-about"): void;
}>();

const scans = useScansStore();
const settings = useSettingsStore();
const { t } = useI18n();
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
              class="relative flex size-10 items-center justify-center rounded-xl transition-colors"
              :class="
                activeView === 'workspace'
                  ? 'bg-signal/10 text-signal'
                  : 'text-muted-foreground hover:bg-overlay-3 hover:text-foreground'
              "
              aria-label="工作台"
            >
              <span
                v-if="activeView === 'workspace'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-signal"
              />
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
                activeView === 'scan' || scans.running
                  ? 'bg-signal/10 text-signal'
                  : 'text-muted-foreground hover:text-foreground'
              "
              aria-label="设备扫描"
              @click="emit('open-scan')"
            >
              <span
                v-if="activeView === 'scan'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-signal"
              />
              <ScanLine class="size-[18px]" />
              <span
                v-if="scans.running"
                class="status-dot status-dot-live absolute right-2 top-2 scale-75 text-signal"
              />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.scan") }}</TooltipContent>
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
              class="relative flex size-10 items-center justify-center rounded-xl transition-colors"
              :class="
                activeView === 'about'
                  ? 'bg-signal/10 text-signal'
                  : 'text-muted-foreground hover:bg-overlay-3 hover:text-foreground'
              "
              aria-label="关于"
              @click="emit('open-about')"
            >
              <span
                v-if="activeView === 'about'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-signal"
              />
              <Info class="size-[18px]" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">关于作者与定制服务</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="relative flex size-10 items-center justify-center rounded-xl transition-colors"
              :class="
                activeView === 'settings'
                  ? 'bg-signal/10 text-signal'
                  : 'text-muted-foreground hover:bg-overlay-3 hover:text-foreground'
              "
              aria-label="设置"
              @click="emit('open-settings')"
            >
              <span
                v-if="activeView === 'settings'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-signal"
              />
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
