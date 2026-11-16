<script setup lang="ts" generic="T extends string | number">
import { ChevronDown } from "@lucide/vue";

const model = defineModel<T>({ required: true });

defineProps<{
  options: Array<{ label: string; value: T }>;
  disabled?: boolean;
}>();
</script>

<template>
  <div class="relative">
    <select
      v-model="model"
      :disabled="disabled"
      class="h-9 w-full appearance-none rounded-lg border border-hairline-strong bg-well px-3 pr-8 text-[12px] text-foreground outline-none transition-colors focus:border-signal/40 focus:ring-2 focus:ring-signal/10 disabled:cursor-not-allowed disabled:opacity-50"
    >
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
        class="bg-popover text-popover-foreground"
      >
        {{ option.label }}
      </option>
    </select>
    <ChevronDown
      class="pointer-events-none absolute right-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
    />
  </div>
</template>
