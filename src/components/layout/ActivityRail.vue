<script setup lang="ts">
import {
  BellRing,
  Info,
  LayoutDashboard,
  Moon,
  Network,
  ScanLine,
  Settings2,
  Sun,
} from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useAlertsStore } from "@/stores/alerts";
import { useNetworkStore } from "@/stores/network";
import { useScansStore } from "@/stores/scans";
import { useSettingsStore } from "@/stores/settings";

type RailView = "workspace" | "network" | "scan" | "alerts" | "settings" | "about";

withDefaults(
  defineProps<{
    activeView?: RailView;
  }>(),
  { activeView: "workspace" },
);

const emit = defineEmits<{
  (event: "open-settings"): void;
  (event: "open-network"): void;
  (event: "open-scan"): void;
  (event: "open-alerts"): void;
  (event: "open-about"): void;
}>();

const alerts = useAlertsStore();
const network = useNetworkStore();
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
                activeView === 'network' || network.active
                  ? 'bg-rx/10 text-rx'
                  : 'text-muted-foreground hover:text-foreground'
              "
              :aria-label="t('nav.network')"
              @click="emit('open-network')"
            >
              <span
                v-if="activeView === 'network'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-rx"
              />
              <Network class="size-[18px]" />
              <span
                v-if="network.active"
                class="status-dot status-dot-live absolute right-2 top-2 scale-75 text-signal"
              />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("nav.network") }}</TooltipContent>
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

        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="relative flex size-10 items-center justify-center rounded-xl transition-colors hover:bg-overlay-3"
              :class="
                activeView === 'alerts'
                  ? 'bg-warn/10 text-warn'
                  : 'text-muted-foreground hover:text-foreground'
              "
              aria-label="告警记录"
              @click="emit('open-alerts')"
            >
              <span
                v-if="activeView === 'alerts'"
                class="absolute -left-[10px] h-5 w-[2px] rounded-full bg-warn"
              />
              <BellRing class="size-[18px]" />
              <span
                v-if="alerts.unread"
                class="numeric absolute -right-0.5 -top-0.5 flex min-w-[16px] items-center justify-center rounded-full border border-warn/30 bg-warn px-1 text-[9px] font-semibold leading-4 text-background"
              >
                {{ alerts.unread > 9 ? "9+" : alerts.unread }}
              </span>
              <span
                v-else-if="alerts.activeCount"
                class="absolute right-1.5 top-1.5 size-1.5 rounded-full bg-warn"
              />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{{ t("alerts.title") }}</TooltipContent>
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
