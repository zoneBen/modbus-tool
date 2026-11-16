<script setup lang="ts">
import {
  ChevronDown,
  ChevronUp,
  Download,
  LineChart,
  Pause,
  Play,
  Radio,
  ScrollText,
  Search,
  Trash2,
} from "@lucide/vue";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import EmptyState from "@/components/common/EmptyState.vue";
import ChartPanel from "@/components/chart/ChartPanel.vue";
import FrameDetail from "@/components/monitor/FrameDetail.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { formatHexBytes, formatLatency, formatTime } from "@/lib/format";
import { useFramesStore } from "@/stores/frames";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";
import type { Frame } from "@/types/models";

const props = defineProps<{
  collapsed: boolean;
}>();

const emit = defineEmits<{
  (event: "toggle"): void;
}>();

const frames = useFramesStore();
const sessions = useSessionsStore();
const settings = useSettingsStore();
const { t } = useI18n();

const listEl = ref<HTMLElement | null>(null);
const initialTab = new URLSearchParams(window.location.search).get("tab");
const activeTab = ref<"frames" | "chart" | "log">(initialTab === "chart" ? "chart" : "frames");
const directionOptions = ["all", "tx", "rx"] as const;

const count = computed(() => frames.filteredFrames.length);

function snippet(frame: Frame): string {
  const text = formatHexBytes(frame.raw);
  return text.length > 74 ? `${text.slice(0, 74)} …` : text;
}

const statusTone: Record<Frame["status"], string> = {
  ok: "text-signal/60",
  exception: "text-warn",
  timeout: "text-error",
  error: "text-error",
};

const directionClass: Record<Frame["direction"], string> = {
  tx: "border-tx/25 bg-tx/10 text-tx",
  rx: "border-rx/25 bg-rx/10 text-rx",
};

async function scrollToEnd(): Promise<void> {
  if (!props.collapsed && settings.monitorFollow) {
    await nextTick();
    if (listEl.value) listEl.value.scrollTop = listEl.value.scrollHeight;
  }
}

watch(() => frames.filteredFrames.length, scrollToEnd);
watch(
  () => settings.monitorFollow,
  (follow) => {
    if (follow) void scrollToEnd();
  },
);

onMounted(scrollToEnd);

function openTab(tab: "frames" | "chart" | "log"): void {
  if (tab === "log") {
    toast.info("CSV 数据日志将在 P6 阶段启用。");
    return;
  }
  activeTab.value = tab;
}

function exportFrames(): void {
  toast.info("报文导出将在 P6 阶段启用。");
}
</script>

<template>
  <section class="flex min-h-0 flex-col border-t border-white/[0.055] bg-[color:var(--surface)]/55">
    <div class="flex h-[38px] shrink-0 items-center gap-0.5 px-2">
      <button
        type="button"
        class="relative flex h-full items-center gap-1.5 px-2 text-[11px] transition-colors"
        :class="
          activeTab === 'frames'
            ? 'text-foreground'
            : 'text-muted-foreground/65 hover:text-foreground'
        "
        @click="openTab('frames')"
      >
        {{ t("monitor.title") }}
        <span
          class="numeric rounded bg-white/[0.06] px-1 py-0.5 text-[9px] text-muted-foreground/70"
        >
          {{ count }}
        </span>
        <span
          v-if="activeTab === 'frames'"
          class="absolute inset-x-1 bottom-0 h-[1.5px] bg-signal/80"
        />
      </button>
      <button
        type="button"
        class="relative flex h-full items-center gap-1.5 px-2 text-[11px] transition-colors"
        :class="
          activeTab === 'chart'
            ? 'text-foreground'
            : 'text-muted-foreground/50 hover:text-foreground'
        "
        @click="openTab('chart')"
      >
        <LineChart class="size-3.5" />
        实时曲线
        <span v-if="activeTab === 'chart'" class="absolute inset-x-1 bottom-0 h-[1.5px] bg-rx/80" />
      </button>
      <button
        type="button"
        class="flex h-full items-center gap-1.5 px-2 text-[11px] text-muted-foreground/50 transition-colors hover:text-foreground"
        @click="openTab('log')"
      >
        <ScrollText class="size-3.5" />
        数据日志
        <span class="rounded border border-white/[0.06] px-1 text-[8px]">P6</span>
      </button>

      <div v-if="activeTab === 'frames'" class="ml-auto flex items-center gap-1.5">
        <div class="relative hidden md:block">
          <Search
            class="pointer-events-none absolute left-2 top-1/2 size-3 -translate-y-1/2 text-muted-foreground/50"
          />
          <Input
            v-model="frames.search"
            placeholder="搜索摘要 / HEX"
            class="h-7 w-[166px] rounded-lg border-white/[0.06] bg-black/20 pl-7 text-[10.5px] placeholder:text-muted-foreground/40"
          />
        </div>

        <div class="flex rounded-lg border border-white/[0.06] bg-black/20 p-0.5">
          <button
            v-for="option in directionOptions"
            :key="option"
            type="button"
            class="numeric rounded-md px-1.5 py-0.5 text-[9px] uppercase transition-colors"
            :class="
              frames.direction === option
                ? 'bg-white/[0.08] text-foreground'
                : 'text-muted-foreground/55 hover:text-foreground'
            "
            @click="frames.direction = option"
          >
            {{ option }}
          </button>
        </div>

        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md text-muted-foreground/70 hover:text-foreground"
          :aria-label="settings.monitorFollow ? t('monitor.pause') : t('monitor.resume')"
          @click="settings.monitorFollow = !settings.monitorFollow"
        >
          <Pause v-if="settings.monitorFollow" class="size-3" />
          <Play v-else class="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md text-muted-foreground/70 hover:text-foreground"
          aria-label="导出报文"
          @click="exportFrames"
        >
          <Download class="size-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          class="rounded-md text-muted-foreground/70 hover:text-foreground"
          aria-label="清空报文"
          @click="frames.clearFrames()"
        >
          <Trash2 class="size-3" />
        </Button>
      </div>
      <Button
        variant="ghost"
        size="icon-xs"
        class="rounded-md text-muted-foreground/70 hover:text-foreground"
        :class="activeTab === 'frames' ? '' : 'ml-auto'"
        :aria-label="collapsed ? '展开监控面板' : '收起监控面板'"
        @click="emit('toggle')"
      >
        <ChevronDown v-if="collapsed" class="size-3.5" />
        <ChevronUp v-else class="size-3.5" />
      </Button>
    </div>

    <div v-if="!collapsed" class="flex min-h-0 flex-1 border-t border-white/[0.05]">
      <ChartPanel v-if="activeTab === 'chart'" class="min-h-0 min-w-0 flex-1" />
      <template v-else>
        <div ref="listEl" class="scrollbar-thin min-h-0 flex-1 overflow-auto">
          <div v-if="frames.filteredFrames.length" class="min-w-[760px]">
            <button
              v-for="frame in frames.filteredFrames"
              :key="frame.seq"
              type="button"
              class="relative grid h-[28px] w-full grid-cols-[84px_36px_112px_minmax(130px,1fr)_minmax(170px,1.35fr)_58px_18px] items-center gap-2 border-b border-white/[0.03] px-3 text-left transition-colors hover:bg-white/[0.03]"
              :class="frame.seq === frames.selectedFrame?.seq ? 'bg-white/[0.045]' : ''"
              @click="frames.selectFrame(frame.seq)"
            >
              <span
                v-if="frame.seq === frames.selectedFrame?.seq"
                class="absolute inset-y-1 left-0 w-[2px] rounded-full bg-rx/70"
              />
              <span class="numeric text-[9.5px] text-muted-foreground/55">{{
                formatTime(frame.timestamp)
              }}</span>
              <span
                class="numeric inline-flex h-[15px] items-center justify-center rounded border text-[8px] font-medium"
                :class="directionClass[frame.direction]"
              >
                {{ frame.direction.toUpperCase() }}
              </span>
              <span class="truncate text-[10px] text-muted-foreground/75">
                {{ sessions.sessionName(frame.sessionId) }}
              </span>
              <span class="truncate text-[10.5px] text-foreground/80">{{ frame.summary }}</span>
              <span class="numeric truncate text-[9.5px] text-muted-foreground/60">{{
                snippet(frame)
              }}</span>
              <span class="numeric text-right text-[9.5px] text-muted-foreground/55">
                {{ formatLatency(frame.latencyMs ?? 0) }}
              </span>
              <span class="flex justify-center">
                <span class="status-dot scale-[0.65]" :class="statusTone[frame.status]" />
              </span>
            </button>
          </div>

          <EmptyState
            v-else
            :title="t('monitor.empty')"
            :description="t('monitor.emptyHint')"
            class="h-full"
          >
            <template #icon>
              <Radio class="size-4" />
            </template>
          </EmptyState>
        </div>

        <FrameDetail
          :frame="frames.selectedFrame"
          class="hidden w-[360px] shrink-0 border-l border-white/[0.05] lg:flex"
        />
      </template>
    </div>
  </section>
</template>
