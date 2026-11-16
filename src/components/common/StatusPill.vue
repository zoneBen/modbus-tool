<script setup lang="ts">
import { computed } from "vue";
import type { SessionStatus } from "@/types/models";

const props = withDefaults(
  defineProps<{
    status: SessionStatus;
    polling?: boolean;
    compact?: boolean;
  }>(),
  { polling: false, compact: false },
);

const label = computed(() => {
  switch (props.status) {
    case "connected":
      return props.polling ? "轮询中" : "已连接";
    case "connecting":
      return "连接中";
    case "error":
      return "错误";
    default:
      return "未连接";
  }
});

const tone = computed(() => {
  switch (props.status) {
    case "connected":
      return "text-signal";
    case "connecting":
      return "text-warn";
    case "error":
      return "text-error";
    default:
      return "text-muted-foreground";
  }
});
</script>

<template>
  <span class="inline-flex items-center gap-1.5" :class="tone">
    <span
      class="status-dot"
      :class="{ 'status-dot-live': status === 'connected' && polling }"
      aria-hidden="true"
    />
    <span v-if="!compact" class="text-[12px] font-medium">{{ label }}</span>
  </span>
</template>
