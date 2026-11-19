<script setup lang="ts">
import type { Component } from "vue";

defineProps<{
  title: string;
  description: string;
  icon: Component;
}>();
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header
      class="flex min-h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4 py-2"
    >
      <span
        class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-signal/20 bg-signal/10 text-signal"
      >
        <component :is="icon" class="size-4" />
      </span>
      <div class="min-w-0 flex-1">
        <p class="truncate text-[13px] font-semibold">{{ title }}</p>
        <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">{{ description }}</p>
      </div>
      <slot name="actions" />
    </header>
    <div class="scrollbar-thin min-h-0 flex-1 overflow-y-auto px-4 py-4">
      <slot />
    </div>
    <footer
      v-if="$slots.footer"
      class="flex shrink-0 items-center justify-end gap-2 border-t border-hairline bg-well-soft px-4 py-3"
    >
      <slot name="footer" />
    </footer>
  </div>
</template>
