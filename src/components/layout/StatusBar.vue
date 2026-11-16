<script setup lang="ts">
import { useNow } from "@vueuse/core";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { formatClock, formatLatency, formatNumber } from "@/lib/format";
import { useFramesStore } from "@/stores/frames";
import { useSessionsStore } from "@/stores/sessions";
import { useSettingsStore } from "@/stores/settings";

const sessions = useSessionsStore();
const frames = useFramesStore();
const settings = useSettingsStore();
const { t } = useI18n();
const now = useNow();
const clock = computed(() => formatClock(now.value.getTime()));
</script>

<template>
  <footer
    class="flex h-[28px] shrink-0 items-center gap-3 border-t border-white/[0.06] bg-black/20 px-3 text-[10px] text-muted-foreground/70"
  >
    <div class="flex items-center gap-1.5">
      <span
        class="status-dot status-dot-live"
        :class="sessions.demo ? 'text-sim' : 'text-signal'"
      />
      <span class="font-medium" :class="sessions.demo ? 'text-sim/90' : 'text-signal/90'">
        {{ sessions.demo ? t("status.demo") : "本地运行" }}
      </span>
    </div>
    <span class="h-3 w-px bg-white/10" />
    <div class="flex min-w-0 items-center gap-1.5">
      <span
        class="status-dot"
        :class="
          sessions.activeSession?.status === 'connected' ? 'text-signal' : 'text-muted-foreground'
        "
      />
      <span class="truncate">{{ sessions.activeSession?.name ?? "无活动会话" }}</span>
      <span class="hidden truncate text-muted-foreground/50 sm:inline">
        {{ sessions.activeSession?.endpoint }}
      </span>
    </div>
    <span class="hidden items-center gap-1 md:flex">
      <span>{{ t("status.sessions") }}</span>
      <span class="numeric text-foreground/80"
        >{{ sessions.connectedCount }}/{{ sessions.sessions.length }}</span
      >
    </span>
    <span class="hidden items-center gap-1 lg:flex">
      <span>{{ t("status.frames") }}</span>
      <span class="numeric text-foreground/80">{{ formatNumber(frames.stats.total) }}</span>
      <span class="text-muted-foreground/40">·</span>
      <span class="text-tx/80">TX {{ frames.stats.tx }}</span>
      <span class="text-muted-foreground/40">/</span>
      <span class="text-rx/80">RX {{ frames.stats.rx }}</span>
    </span>
    <span v-if="frames.stats.errors > 0" class="flex items-center gap-1 text-error">
      <span>{{ t("status.errors") }}</span>
      <span class="numeric">{{ frames.stats.errors }}</span>
    </span>

    <div class="ml-auto flex items-center gap-3">
      <span class="hidden items-center gap-1 sm:flex">
        <span>{{ t("status.addressBase") }}</span>
        <span class="numeric text-foreground/80">{{ settings.addressBase }}</span>
      </span>
      <span class="hidden items-center gap-1 md:flex">
        <span>latency</span>
        <span class="numeric text-foreground/80">
          {{ formatLatency(sessions.activeSession?.stats.latencyMs ?? 0) }}
        </span>
      </span>
      <span class="numeric text-muted-foreground/60">{{ clock }}</span>
      <span class="hidden text-muted-foreground/40 lg:inline">
        {{ sessions.demo ? "Demo Runtime" : "Tauri 2 · WebView2" }}
      </span>
    </div>
  </footer>
</template>
