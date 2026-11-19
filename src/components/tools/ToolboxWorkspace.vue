<script setup lang="ts">
import type { Component } from "vue";
import { useI18n } from "vue-i18n";

export interface ToolboxItem {
  id: string;
  title: string;
  description: string;
  icon: Component;
  disabled?: boolean;
  badge?: string;
}

const props = withDefaults(
  defineProps<{
    title: string;
    subtitle: string;
    items: ToolboxItem[];
    active: string;
    accent?: "rx" | "sim" | "signal";
  }>(),
  { accent: "signal" },
);

const emit = defineEmits<{
  (event: "update:active", id: string): void;
}>();

const { t } = useI18n();

const accentClass: Record<string, { active: string; icon: string; bar: string }> = {
  rx: { active: "bg-rx/10 text-rx", icon: "border-rx/25 bg-rx/10 text-rx", bar: "bg-rx" },
  sim: { active: "bg-sim/10 text-sim", icon: "border-sim/25 bg-sim/10 text-sim", bar: "bg-sim" },
  signal: {
    active: "bg-signal/10 text-signal",
    icon: "border-signal/20 bg-signal/10 text-signal",
    bar: "bg-signal",
  },
};
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1">
    <aside class="flex h-full w-[244px] shrink-0 flex-col border-r border-hairline bg-overlay-1">
      <div class="flex h-[52px] shrink-0 items-center justify-between px-4">
        <div class="min-w-0">
          <p class="truncate text-[13px] font-semibold tracking-tight">{{ title }}</p>
          <p class="mt-0.5 truncate text-[11px] text-muted-foreground">{{ subtitle }}</p>
        </div>
        <span
          class="numeric rounded border border-hairline bg-well px-1.5 py-0.5 text-[10px] text-muted-foreground"
        >
          {{ items.length.toString().padStart(2, "0") }}
        </span>
      </div>

      <div class="scrollbar-thin min-h-0 flex-1 space-y-1 overflow-y-auto px-2.5 pb-3">
        <button
          v-for="item in props.items"
          :key="item.id"
          type="button"
          class="group relative flex w-full items-start gap-2.5 rounded-xl border px-2.5 py-2.5 text-left transition-all duration-200"
          :class="[
            item.id === props.active
              ? `border-hairline-strong ${accentClass[props.accent]?.active ?? accentClass.signal?.active} shadow-lg shadow-black/20`
              : 'border-transparent hover:border-hairline hover:bg-overlay-1',
            item.disabled ? 'cursor-not-allowed opacity-55' : '',
          ]"
          :disabled="item.disabled"
          :aria-current="item.id === props.active ? 'page' : undefined"
          @click="!item.disabled && emit('update:active', item.id)"
        >
          <span
            v-if="item.id === props.active"
            class="absolute inset-y-3 left-0 w-[2px] rounded-full"
            :class="accentClass[props.accent]?.bar ?? accentClass.signal?.bar"
          />
          <span
            class="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border"
            :class="
              item.id === props.active
                ? (accentClass[props.accent]?.icon ?? accentClass.signal?.icon)
                : 'border-hairline bg-well text-muted-foreground group-hover:text-foreground'
            "
          >
            <component :is="item.icon" class="size-3.5" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-1.5">
              <span class="truncate text-[12px] font-medium">{{ item.title }}</span>
              <span
                v-if="item.disabled"
                class="shrink-0 rounded-full border border-hairline bg-well px-1.5 py-0.5 text-[9px] text-muted-foreground"
              >
                {{ item.badge ?? t("tools.comingSoon") }}
              </span>
            </span>
            <span class="mt-0.5 line-clamp-2 text-[10.5px] leading-relaxed text-muted-foreground">
              {{ item.description }}
            </span>
          </span>
        </button>
      </div>

      <div
        class="shrink-0 border-t border-hairline px-3.5 py-2.5 text-[10.5px] text-muted-foreground"
      >
        {{ t("tools.localHint") }}
      </div>
    </aside>

    <div class="flex min-h-0 min-w-0 flex-1 flex-col">
      <slot />
    </div>
  </div>
</template>
