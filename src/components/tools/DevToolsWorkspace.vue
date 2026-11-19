<script setup lang="ts">
import { Binary, Braces, Clock, Hash, Search } from "@lucide/vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BaseTool from "@/components/tools/dev/BaseTool.vue";
import FormatterTool from "@/components/tools/dev/FormatterTool.vue";
import HashTool from "@/components/tools/dev/HashTool.vue";
import SearchTool from "@/components/tools/dev/SearchTool.vue";
import TimestampTool from "@/components/tools/dev/TimestampTool.vue";
import ToolPlaceholder from "@/components/tools/ToolPlaceholder.vue";
import ToolboxWorkspace from "@/components/tools/ToolboxWorkspace.vue";
import type { ToolboxItem } from "@/components/tools/ToolboxWorkspace.vue";
import { DEV_TOOLS, isDevToolId } from "@/lib/tool-catalog";
import { useToolboxStore } from "@/stores/toolbox";

const props = withDefaults(
  defineProps<{
    initialTool?: string | null;
  }>(),
  { initialTool: null },
);

const toolbox = useToolboxStore();
const { t } = useI18n();

const icons = {
  formatter: Braces,
  timestamp: Clock,
  base: Binary,
  hash: Hash,
  search: Search,
} as const;

if (isDevToolId(props.initialTool)) {
  toolbox.setDevTool(props.initialTool);
}

const items = computed<ToolboxItem[]>(() =>
  DEV_TOOLS.map((tool) => ({
    id: tool.id,
    title: t(tool.labelKey),
    description: t(tool.descriptionKey),
    icon: icons[tool.id],
    disabled: tool.phase > 1,
    badge: tool.phase > 1 ? t("tools.comingSoon") : undefined,
  })),
);

const active = computed<string>({
  get: () => toolbox.config.devTool,
  set: (value) => {
    if (isDevToolId(value)) toolbox.setDevTool(value);
  },
});

const activeItem = computed(() => items.value.find((item) => item.id === active.value));
</script>

<template>
  <ToolboxWorkspace
    v-model:active="active"
    :title="t('devTools.title')"
    :subtitle="t('devTools.subtitle')"
    :items="items"
    accent="sim"
  >
    <FormatterTool v-if="active === 'formatter'" />
    <TimestampTool v-else-if="active === 'timestamp'" />
    <BaseTool v-else-if="active === 'base'" />
    <HashTool v-else-if="active === 'hash'" />
    <SearchTool v-else-if="active === 'search'" />
    <ToolPlaceholder
      v-else
      :title="activeItem?.title ?? t('devTools.title')"
      :description="activeItem?.description ?? ''"
    />
  </ToolboxWorkspace>
</template>
