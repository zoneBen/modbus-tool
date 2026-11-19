<script setup lang="ts">
import { Hammer } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import EmptyState from "@/components/common/EmptyState.vue";

defineProps<{
  title: string;
  description: string;
}>();

const { t } = useI18n();
</script>

<template>
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header class="flex h-[54px] shrink-0 items-center gap-3 border-b border-hairline px-4">
      <span
        class="flex size-8 shrink-0 items-center justify-center rounded-lg border border-hairline bg-overlay-1 text-muted-foreground"
      >
        <Hammer class="size-4" />
      </span>
      <div class="min-w-0 flex-1">
        <p class="truncate text-[13px] font-semibold">{{ title }}</p>
        <p class="mt-0.5 truncate text-[10.5px] text-muted-foreground">{{ description }}</p>
      </div>
    </header>

    <EmptyState
      :title="t('tools.placeholderTitle')"
      :description="t('tools.placeholderHint')"
      class="m-auto flex-1"
    >
      <template #icon>
        <Hammer class="size-5" />
      </template>
    </EmptyState>
  </div>
</template>
