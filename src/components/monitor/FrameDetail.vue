<script setup lang="ts">
import { ArrowDownLeft, ArrowUpRight, Copy, Radio } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { toast } from "vue-sonner";
import HexBytes from "@/components/monitor/HexBytes.vue";
import { Button } from "@/components/ui/button";
import {
  formatAsciiBytes,
  formatFunctionCode,
  formatHexBytes,
  formatLatency,
  formatTime,
} from "@/lib/format";
import { useSessionsStore } from "@/stores/sessions";
import type { Frame } from "@/types/models";

const props = defineProps<{
  frame: Frame | null;
}>();

const sessions = useSessionsStore();
const { t } = useI18n();

const isTx = computed(() => props.frame?.direction === "tx");
const sessionName = computed(() =>
  props.frame ? sessions.sessionName(props.frame.sessionId) : "—",
);

const toneDot: Record<string, string> = {
  header: "bg-muted-foreground/35",
  unit: "bg-warn",
  function: "bg-rx",
  data: "bg-foreground/45",
  checksum: "bg-error/60",
};

async function copyFrame(): Promise<void> {
  if (!props.frame) return;
  try {
    await navigator.clipboard.writeText(formatHexBytes(props.frame.raw));
    toast.success(t("common.copied"));
  } catch {
    toast.error("复制失败，请检查剪贴板权限");
  }
}
</script>

<template>
  <aside class="flex min-h-0 flex-col bg-well-soft">
    <template v-if="frame">
      <div class="flex h-[38px] shrink-0 items-center gap-2 border-b border-hairline px-3">
        <span class="label-caps">{{ t("monitor.detail") }}</span>
        <span class="numeric text-[10px] text-muted-foreground">#{{ frame.seq }}</span>
        <Button
          variant="ghost"
          size="icon-xs"
          class="ml-auto rounded-md text-muted-foreground hover:text-foreground"
          aria-label="复制原始帧"
          @click="copyFrame"
        >
          <Copy class="size-3" />
        </Button>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto p-3">
        <div class="flex items-center gap-2">
          <span
            class="inline-flex size-6 items-center justify-center rounded-md border"
            :class="isTx ? 'border-tx/25 bg-tx/10 text-tx' : 'border-rx/25 bg-rx/10 text-rx'"
          >
            <ArrowUpRight v-if="isTx" class="size-3.5" />
            <ArrowDownLeft v-else class="size-3.5" />
          </span>
          <div class="min-w-0">
            <p class="truncate text-[12px] font-medium">{{ frame.summary }}</p>
            <p class="numeric text-[10px] text-muted-foreground">
              {{ formatTime(frame.timestamp) }} · {{ sessionName }}
            </p>
          </div>
          <span
            class="numeric ml-auto rounded border border-hairline bg-overlay-1 px-1.5 py-0.5 text-[10px] text-muted-foreground"
          >
            {{ formatLatency(frame.latencyMs ?? 0) }}
          </span>
        </div>

        <div class="mt-4 space-y-1.5">
          <p class="label-caps">{{ t("monitor.decoded") }}</p>
          <div
            v-for="field in frame.decoded.fields"
            :key="`${field.label}-${field.offset}`"
            class="flex items-center gap-2 rounded-md border border-hairline bg-overlay-1 px-2 py-1.5"
          >
            <span class="size-1.5 shrink-0 rounded-full" :class="toneDot[field.tone]" />
            <span class="text-[11px] text-muted-foreground">{{ field.label }}</span>
            <span class="numeric ml-auto truncate text-[12px] text-foreground">{{
              field.value
            }}</span>
          </div>
          <div
            v-if="frame.decoded.functionCode"
            class="flex items-center justify-between px-2 pt-0.5 text-[10px] text-muted-foreground"
          >
            <span>从站 {{ frame.decoded.slaveId }}</span>
            <span class="numeric">{{ formatFunctionCode(frame.decoded.functionCode) }}</span>
          </div>
        </div>

        <div class="mt-4 space-y-1.5">
          <p class="label-caps">{{ t("monitor.raw") }}</p>
          <div class="rounded-lg border border-hairline bg-well p-2">
            <HexBytes :bytes="frame.raw" :fields="frame.decoded.fields" class="break-all" />
          </div>
        </div>

        <div v-if="frame.decoded.ascii" class="mt-3 space-y-1.5">
          <p class="label-caps">ASCII 帧</p>
          <p
            class="numeric rounded-lg border border-hairline bg-well p-2 text-[12px] text-foreground"
          >
            {{ frame.decoded.ascii }}
          </p>
        </div>

        <div class="mt-3 space-y-1.5">
          <p class="label-caps">{{ t("monitor.chars") }}</p>
          <p
            class="numeric rounded-lg border border-hairline bg-well p-2 text-[12px] text-foreground"
          >
            {{ formatAsciiBytes(frame.raw) }}
          </p>
        </div>
      </div>
    </template>

    <div v-else class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
      <div class="glass flex size-10 items-center justify-center rounded-xl text-muted-foreground">
        <Radio class="size-4" />
      </div>
      <p class="text-[12px] text-muted-foreground">选择一条报文查看解析详情</p>
    </div>
  </aside>
</template>
