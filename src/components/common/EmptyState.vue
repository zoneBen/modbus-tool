<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center">
    <div
      class="glass flex size-12 items-center justify-center rounded-2xl text-muted-foreground shadow-lg shadow-black/20"
    >
      <slot name="icon" />
    </div>
    <div class="space-y-1">
      <p class="text-sm font-medium text-foreground/90">{{ title }}</p>
      <p v-if="description" class="mx-auto max-w-sm text-xs leading-relaxed text-muted-foreground">
        {{ description }}
      </p>
    </div>
    <slot />
  </div>
</template>
