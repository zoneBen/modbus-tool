<script setup lang="ts">
import { Gauge, Pause, PencilLine, Play, RefreshCw, Settings2 } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import {
  formatFunctionCode,
  formatInterval,
  formatLatency,
  formatRange,
  formatTime,
} from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const emit = defineEmits<{
  (event: "write"): void;
  (event: "open-settings"): void;
}>();

const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const session = computed(() => sessions.activeSession);
const definition = computed(() => sessions.activeDefinition);
const isSlave = computed(() => session.value?.role === "slave");
const lastUpdate = computed(() => sessions.activeSnapshot?.timestamp ?? sessions.lastUpdate);
const addressBases = [0, 1] as const;
</script>

<template>
  <div class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-center gap-2">
        <span class="truncate text-[13px] font-semibold tracking-tight">
          {{ definition?.name ?? session?.name ?? "未选择会话" }}
        </span>
        <template v-if="definition">
          <span
            class="numeric rounded border border-rx/20 bg-rx/10 px-1.5 py-0.5 text-[11px] font-medium text-rx"
          >
            {{ formatFunctionCode(definition.functionCode) }}
          </span>
          <span
            class="numeric rounded border border-hairline bg-overlay-1 px-1.5 py-0.5 text-[11px] text-muted-foreground"
          >
            从站 {{ definition.slaveId }}
          </span>
          <span
            class="numeric hidden rounded border border-hairline bg-well px-1.5 py-0.5 text-[11px] text-muted-foreground sm:inline"
          >
            {{ formatRange(definition.startAddress, definition.quantity, settings.addressBase) }}
          </span>
          <span
            class="numeric rounded px-1.5 py-0.5 text-[11px] font-medium uppercase"
            :class="
              definition.dataType === 'f32' || definition.dataType === 'f64'
                ? 'border border-rx/20 bg-rx/10 text-rx'
                : 'border border-signal/20 bg-signal/10 text-signal'
            "
          >
            {{ definition.dataType.toUpperCase() }}
          </span>
        </template>
      </div>
      <div class="mt-0.5 flex min-w-0 items-center gap-2 text-[11px] text-muted-foreground">
        <span class="truncate">{{ session?.endpoint ?? "—" }}</span>
        <span class="text-foreground/15">/</span>
        <span class="uppercase">{{ session?.framing ?? "—" }}</span>
        <template v-if="definition">
          <span class="text-foreground/15">/</span>
          <span class="numeric">{{ formatInterval(definition.intervalMs) }}</span>
        </template>
        <template v-if="lastUpdate">
          <span class="text-foreground/15">/</span>
          <span class="numeric">{{ formatTime(lastUpdate) }}</span>
        </template>
      </div>
    </div>

    <div class="hidden items-center gap-2 xl:flex">
      <Gauge class="size-3.5 text-muted-foreground" />
      <span class="numeric text-[12px] text-muted-foreground">
        {{ formatLatency(session?.stats.latencyMs ?? 0) }}
      </span>
    </div>

    <div class="mr-1 hidden items-center gap-2 md:flex">
      <span class="text-[11px] text-muted-foreground">{{ t("status.addressBase") }}</span>
      <div class="flex rounded-lg border border-hairline bg-well p-0.5">
        <button
          v-for="base in addressBases"
          :key="base"
          type="button"
          class="numeric rounded-md px-1.5 py-0.5 text-[11px] transition-colors"
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

    <TooltipProvider :delay-duration="180">
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 gap-1.5 border-hairline-strong bg-overlay-1 text-[12px]"
            :disabled="!definition || isSlave"
            @click="emit('write')"
          >
            <PencilLine class="size-3.5" />
            <span class="hidden lg:inline">{{ t("toolbar.write") }}</span>
          </Button>
        </TooltipTrigger>
        <TooltipContent side="bottom">打开写入寄存器弹窗</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="outline"
            size="sm"
            class="h-8 gap-1.5 border-hairline-strong bg-overlay-1 text-[12px]"
            :disabled="!definition || isSlave"
            @click="sessions.readOnce()"
          >
            <RefreshCw class="size-3.5" />
            <span class="hidden lg:inline">{{ t("toolbar.readOnce") }}</span>
          </Button>
        </TooltipTrigger>
        <TooltipContent side="bottom">立即读取当前定义</TooltipContent>
      </Tooltip>
    </TooltipProvider>

    <Button
      v-if="isSlave"
      variant="outline"
      size="sm"
      class="h-8 gap-1.5 border-sim/25 bg-sim/10 text-[12px] text-sim"
      disabled
    >
      <span class="status-dot status-dot-live" />
      {{ t("toolbar.listening") }}
    </Button>
    <Button
      v-else
      size="sm"
      class="h-8 gap-1.5 text-[12px]"
      :variant="session?.polling ? 'default' : 'outline'"
      :class="session?.polling ? 'glow-signal' : 'border-hairline-strong bg-overlay-1'"
      :disabled="!session"
      @click="sessions.togglePolling()"
    >
      <Pause v-if="session?.polling" class="size-3.5" />
      <Play v-else class="size-3.5" />
      {{ session?.polling ? t("toolbar.stop") : t("toolbar.start") }}
    </Button>

    <TooltipProvider :delay-duration="180">
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            variant="ghost"
            size="icon-sm"
            class="rounded-lg text-muted-foreground hover:text-foreground"
            aria-label="设置"
            @click="emit('open-settings')"
          >
            <Settings2 class="size-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="bottom">{{ t("toolbar.settings") }}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  </div>
</template>
